
use avian3d::{math::*, prelude::*};
use bevy::input::mouse::*;
use bevy::{ecs::query::Has, prelude::*};
use serde::{Deserialize, Serialize};

use crate::client::{
     ClientMovement, ControlledPlayer,
};
use crate::consts::PSEUDO_MAX_AIR_SPEED;
use crate::input:: MovementIntent;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerAction>()
            .add_systems(Startup, show_player_ui)
            .add_systems(Update, (mouse_input, update_player_ui))
            .add_systems(
                FixedUpdate,
                (
                    update_grounded,
                    apply_movement_damping,
                    movement,
                    movement_2,
                    //check_player_death,
                ),
            )
            .add_event::<ClientMovement>()
            .register_type::<Health>();
    }
}

/// An event sent for a movement input action.
#[derive(Event, Clone, Deserialize, Serialize)]
pub enum PlayerAction {
    Move([f32; 3]),
    Jump,
    Shoot,
    Rotate([f32; 4]),
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;
/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(pub Scalar);

/// The damping factor used for slowing down movement.
#[derive(Component)]
pub struct MovementDampingFactor(pub Scalar);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(pub Scalar);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

#[derive(Component, Debug, Reflect)]
pub struct Health(pub usize);

/// A bundle that contains the components needed for a basic
/// kinematic character controller.
#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    rigid_body: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    locked_axes: LockedAxes,
    movement: MovementBundle,
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    damping: MovementDampingFactor,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(
        acceleration: Scalar,
        damping: Scalar,
        jump_impulse: Scalar,
        max_slope_angle: Scalar,
    ) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            damping: MovementDampingFactor(damping),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 0.9, 7.0, PI * 0.45)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            rigid_body: RigidBody::Dynamic,
            collider,
            ground_caster: ShapeCaster::new(
                caster_shape,
                Vector::ZERO,
                Quaternion::default(),
                Dir3::NEG_Y,
            )
            .with_max_distance(0.2),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: Scalar,
        damping: Scalar,
        jump_impulse: Scalar,
        max_slope_angle: Scalar,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, damping, jump_impulse, max_slope_angle);
        self
    }
}

fn mouse_input(
    mut evr_scroll: EventReader<MouseWheel>,
    mut movement_event_writer: EventWriter<PlayerAction>,
) {
    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                movement_event_writer.send(PlayerAction::Jump);
            }
            MouseScrollUnit::Pixel => {
                continue;
            }
        }
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).try_insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    time_fixed: Res<Time<Fixed>>,
    mut movement_event_reader: EventReader<PlayerAction>,
    mut controllers: Query<
        (
            &JumpImpulse,
            &mut LinearVelocity,
            Has<Grounded>,
            &mut Transform,
        ),
        With<ControlledPlayer>,
    >,
) {

    for event in movement_event_reader.read() {
        for (
            jump_impulse,
            mut linear_velocity,
            is_grounded,
            mut player_tf,
        ) in &mut controllers
        {
            match event {
                PlayerAction::Move(direction) => {
                    /*
                    linear_velocity.x += move_intent.0.x * movement_acceleration.0 * delta_time;
                    linear_velocity.z += move_intent.0.z * movement_acceleration.0 * delta_time;
                    debug!("delta_time: {:?}", delta_time);
                    //debug!("linear_velocity: {:?}", linear_velocity);

                    let mut air_acc = air_accelerate(*direction, &linear_velocity);

                    let mut accel_speed = 100. * delta_time;
                    if accel_speed > air_acc {
                        accel_speed = air_acc;
                    }
                    debug!("accell_speed: {:?}", accel_speed);

                    linear_velocity.x += accel_speed;
                    linear_velocity.z += accel_speed;
                     */
                }
                PlayerAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = jump_impulse.0;
                        debug!("jumping");
                    }
                }
                PlayerAction::Rotate(rotation) => {
                    player_tf.rotation = Quat::from_array(*rotation);
                }
                _ => {}
            }
        }
    }
}

// moves all players based on their intent
fn movement_2(
    time_fixed: Res<Time<Fixed>>,
    mut controllers: Query<(&MovementAcceleration, &mut LinearVelocity, &MovementIntent)>,
) {
    let delta_time = time_fixed.delta_secs();
    for (movement_acceleration, mut linear_velocity, move_intent) in &mut controllers {
        //linear_velocity.x += move_intent.0.x * movement_acceleration.0 * delta_time;
        //linear_velocity.z += move_intent.0.z * movement_acceleration.0 * delta_time;
        //debug!("velocity: {:?}", linear_velocity.length());

        // Vector projection of Current velocity onto accelDir.
        let proj_vel = linear_velocity.0.dot(move_intent.0);

        // Accelerated velocity in direction of movment
        let mut accel_vel = movement_acceleration.0 * delta_time;

        // If necessary, truncate the accelerated velocity so the vector projection does not exceed max_velocity
        if proj_vel + accel_vel > PSEUDO_MAX_AIR_SPEED {
            accel_vel = PSEUDO_MAX_AIR_SPEED - proj_vel;
        }

        linear_velocity.0 += move_intent.0 * accel_vel;
    }
}

/*
fn air_movement(
    mut controllers: Query<
        (&MovementAcceleration, &mut LinearVelocity, &MovementIntent),
        Without<Grounded>,
    >,

    time_fixed: Res<Time<Fixed>>,
) {
    let delta_time = time_fixed.delta_secs();
    for (movement_acceleration, mut linear_velocity, move_intent) in &mut controllers {
        let proj = linear_velocity.0.project_onto(move_intent.0);

        let is_away = move_intent.0.dot(proj) <= 0.0;

        if proj.norm() < MAX_AIR_SPEED || is_away {
            let mut vc = move_intent.0 * 10.0;

            if !is_away {
                vc = vc.clamp_length_max(MAX_AIR_SPEED - proj.norm());
            } else {
                vc = vc.clamp_length_max(MAX_AIR_SPEED + proj.norm());
            }

            linear_velocity.0 += delta_time * vc;
        }
    }
}


*/

/// Slows down movement in the XZ plane.
pub fn apply_movement_damping(
    mut query: Query<(&MovementDampingFactor, &mut LinearVelocity), With<Grounded>>,
) {
    for (damping_factor, mut linear_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= damping_factor.0;
        linear_velocity.z *= damping_factor.0;
    }
}

#[derive(Component)]
pub struct PlayerHealthUi;

pub fn show_player_ui(mut commands: Commands) {
    debug!("spawning player health ui");
    commands.spawn((
        Name::new("Player health ui"),
        Text::new(""),
        PlayerHealthUi,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
        TextFont {
            font_size: 84.0,
            ..default()
        },
    ));
}

#[derive(Default)]
pub struct PreviousFrameHealth(usize);

pub fn update_player_ui(
    mut health_ui: Query<&mut Text, With<PlayerHealthUi>>,
    player_q: Query<Ref<Health>, With<ControlledPlayer>>,
    mut previous_health: Local<PreviousFrameHealth>,
) {
    let Ok(health) = player_q.get_single() else {
        return;
    };
    // since we mutable deref to sync players. Changed<> doesn't really work
    if health.0 == previous_health.0 {
        return;
    } else {
        previous_health.0 = health.0;
    }
    if !health.is_changed() {
        return;
    }
    let Ok(mut txt) = health_ui.get_single_mut() else {
        return;
    };
    txt.0 = format!("{}", health.0);
}

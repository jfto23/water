use std::time::Duration;

use avian3d::{math::*, prelude::*};
use bevy::input::mouse::*;
use bevy::time::common_conditions::on_timer;
use bevy::{ecs::query::Has, prelude::*};
use bevy_renet::renet::RenetClient;
use serde::{Deserialize, Serialize};

use crate::camera::PlayerMarker;
use crate::client::{ClientChannel, ClientMovement, ControlledPlayer, CurrentClientId};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>()
            .add_systems(Update, (mouse_input, keyboard_input, movement))
            .add_systems(
                FixedUpdate,
                (update_grounded, apply_movement_damping).chain(),
            )
            .add_event::<ClientMovement>();
    }
}

/// An event sent for a movement input action.
#[derive(Event, Clone, Deserialize, Serialize)]
pub enum MovementAction {
    Move([f32; 3]),
    Jump,
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
pub struct MovementDampingFactor(Scalar);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(pub Scalar);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

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

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_q: Query<(&Transform, &GlobalTransform), With<ControlledPlayer>>,
) {
    let forward = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let back = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let Ok((player_tf, global_player_tf)) = player_q.get_single() else {
        return;
    };

    let x_axis = right as i8 - left as i8;
    let z_axis = back as i8 - forward as i8;
    let local_direction =
        Vector3::new(x_axis as Scalar, 0.0 as Scalar, z_axis as Scalar).clamp_length_max(1.0);

    let mut global_direction = global_player_tf.affine().transform_vector3(local_direction);

    global_direction.y = 0.0;
    global_direction = global_direction.normalize();

    /*
    debug!(
        "local_direction: {:?}, global_direction: {:?}",
        local_direction, global_direction,
    );
     */
    if local_direction != Vector3::ZERO {
        movement_event_writer.send(MovementAction::Move(global_direction.to_array()));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_event_writer.send(MovementAction::Jump);
    }
}

fn mouse_input(
    mut evr_scroll: EventReader<MouseWheel>,
    mut movement_event_writer: EventWriter<MovementAction>,
) {
    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                movement_event_writer.send(MovementAction::Jump);
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
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<
        (
            &MovementAcceleration,
            &JumpImpulse,
            &mut LinearVelocity,
            Has<Grounded>,
        ),
        With<ControlledPlayer>,
    >,
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
) {
    // TODO CLEAN THIS MESS
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    // Precision is adjusted so that the example works with
    // both the `f32` and `f64` features. Otherwise you don't need this.
    let delta_time = time.delta_secs_f64().adjust_precision();

    for event in movement_event_reader.read() {
        for (movement_acceleration, jump_impulse, mut linear_velocity, is_grounded) in
            &mut controllers
        {
            match event {
                MovementAction::Move(direction) => {
                    linear_velocity.x += direction[0] * movement_acceleration.0 * delta_time;
                    linear_velocity.z += direction[2] * movement_acceleration.0 * delta_time;

                    /*

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
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = jump_impulse.0;
                    }
                }
                MovementAction::Rotate(_) => {}
            }
        }
        //todo. send the inputs to server. Send a "start_pressed" to server and "stop_pressed" to server to avoid spamming messages
        //https://gamedev.stackexchange.com/questions/74655/what-to-send-to-server-in-real-time-fps-game
        let input_message = bincode::serialize(&ClientMovement {
            movement: event.clone(),
            client_id: client_id.0.into(),
        })
        .unwrap();
        client.send_message(ClientChannel::Input, input_message);
    }
}

fn air_accelerate(wish_velocity: Vec3, current_velocity: &LinearVelocity) -> f32 {
    let wish_speed = f32::min(30.0, current_velocity.0.length());
    let current_speed = wish_velocity.dot(current_velocity.0);
    let add_speed = wish_speed - current_speed;
    return add_speed;
}

/// Slows down movement in the XZ plane.
pub fn apply_movement_damping(mut query: Query<(&MovementDampingFactor, &mut LinearVelocity)>) {
    for (damping_factor, mut linear_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= damping_factor.0;
        linear_velocity.z *= damping_factor.0;
    }
}

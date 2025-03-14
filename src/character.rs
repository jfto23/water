use std::time::Duration;

use avian3d::{math::*, prelude::*};
use bevy::color::palettes::css::WHITE;
use bevy::input::mouse::*;
use bevy::pbr::NotShadowCaster;
use bevy::render::view::RenderLayers;
use bevy::{ecs::query::Has, prelude::*};
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

use crate::camera::{CameraSensitivity, PlayerMarker, WorldCamera};
use crate::client::{ClientAction, ControlledPlayer};
use crate::consts::{
    CHARACTER_MODEL_PATH, PLAYER_HEALTH, PSEUDO_MAX_AIR_SPEED, SHOOT_COOLDOWN,
    VIEW_MODEL_RENDER_LAYER,
};
use crate::input::{build_input_map, Action, LookDirection, MovementIntent};
use crate::server::{Player, WeaponCooldown};

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
            .add_event::<ClientAction<Action>>()
            .register_type::<Health>();
    }
}

/// An event sent for a movement input action.
#[derive(Event, Clone, Deserialize, Serialize)]
pub enum PlayerAction {
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
        for (jump_impulse, mut linear_velocity, is_grounded, mut player_tf) in &mut controllers {
            match event {
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

pub enum NetworkScenario {
    Server,
    MyClient,
    OtherClient,
}

pub fn build_player_ent(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    client_id: u64,
    scenario: NetworkScenario,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let player_entity = commands
        .spawn((
            Name::new("Player entity"),
            NotShadowCaster,
            Transform::from_xyz(0.0, 1.5, 0.0),
            CharacterControllerBundle::new(Collider::cuboid(1.0, 2.0, 1.0)).with_movement(
                50.0,
                0.9,
                7.0,
                (20.0 as Scalar).to_radians(),
            ),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            GravityScale(2.0),
            PlayerMarker,
            MovementIntent::default(),
            TransformInterpolation,
            CameraSensitivity::default(),
            InputManagerBundle::with_map(build_input_map()),
            Player { id: client_id },
        ))
        .id();

    commands
        .entity(player_entity)
        .insert(LookDirection::default());
    commands.entity(player_entity).insert(Health(PLAYER_HEALTH));
    commands
        .entity(player_entity)
        .insert(WeaponCooldown(Timer::new(
            Duration::from_secs_f32(SHOOT_COOLDOWN),
            TimerMode::Once,
        )));

    match scenario {
        NetworkScenario::Server | NetworkScenario::OtherClient => {
            let mut player_model_tf = Transform::from_xyz(0., -1., 0.);
            player_model_tf.rotate_local_y(PI / 2.);
            let player_model = commands
                .spawn((
                    SceneRoot(
                        asset_server
                            .load(GltfAssetLabel::Scene(0).from_asset(CHARACTER_MODEL_PATH)),
                    ),
                    player_model_tf,
                    Name::new("Player Model"),
                ))
                .id();
            commands.entity(player_entity).add_child(player_model);
        }

        NetworkScenario::MyClient => {
            commands.entity(player_entity).insert(ControlledPlayer);
            let world_cam = commands
                .spawn((
                    Name::new("World Camera"),
                    WorldCamera,
                    Camera3d::default(),
                    Transform::from_xyz(0., 0.5, 0.),
                    Projection::from(PerspectiveProjection {
                        fov: 90.0_f32.to_radians(),
                        ..default()
                    }),
                ))
                .id();
            commands.entity(player_entity).add_child(world_cam);

            let view_model_cam = commands
                .spawn((
                    Name::new("View Model Camera"),
                    Camera3d::default(),
                    Camera {
                        // Bump the order to render on top of the world model.
                        order: 1,
                        ..default()
                    },
                    Projection::from(PerspectiveProjection {
                        fov: 90.0_f32.to_radians(),
                        ..default()
                    }),
                    // Only render objects belonging to the view model.
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                ))
                .id();

            let arm_mesh = meshes.add(Cuboid::new(0.1, 0.1, 0.5));
            let arm_material = materials.add(Color::from(WHITE));
            let arm = commands
                .spawn((
                    Name::new("Player Arm"),
                    Mesh3d(arm_mesh),
                    MeshMaterial3d(arm_material),
                    Transform::from_xyz(0.3, -0.2, -0.3),
                    // Ensure the arm is only rendered by the view model camera.
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    NotShadowCaster,
                ))
                .id();

            let crosshair = commands
                .spawn((
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    Name::new("Crosshair"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            width: Val::Px(7.0),
                            height: Val::Px(7.0),
                            ..default()
                        },
                        BackgroundColor(WHITE.into()),
                    ));
                });

            commands.entity(player_entity).add_child(view_model_cam);
            commands.entity(player_entity).add_child(arm);
            commands.entity(player_entity).insert(Name::new("MyPlayer"));
        }
    }

    return player_entity;
}

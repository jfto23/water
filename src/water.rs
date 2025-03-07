use std::f32::consts::PI;

use crate::camera::*;
use crate::consts::*;
use avian3d::math::Scalar;
use avian3d::prelude::*;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::PURPLE;
use bevy::math::NormedVectorSpace;
use bevy::{color::palettes::css::WHITE, pbr::NotShadowCaster, prelude::*};
use bevy_renet::renet::RenetServer;

use crate::{
    camera::{CameraSensitivity, PlayerMarker},
    character::*,
};
use bevy::render::view::RenderLayers;

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (water_setup /*spawn_player*/,))
            .add_systems(
                FixedUpdate,
                (handle_rocket_collision, handle_rocket_explosion).chain(),
            )
            .add_systems(Update, debug_rocket_explosion)
            .add_event::<RocketExplosion>();
    }
}
fn water_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("maps/map_test.glb"))),
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
        RigidBody::Static,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 120.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
    ));

    /*
    commands.spawn((
        Name::new("Floor"),
        RigidBody::Static,
        Collider::cylinder(300.0, 0.1),
        Mesh3d(meshes.add(Cylinder::new(300.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));
     */

    /*
    commands.spawn((
        Name::new("Surf Cube"),
        RigidBody::Static,
        Collider::cuboid(6.0, 6.0, 42.0),
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(6.0, 6.0, 42.0)))),
        MeshMaterial3d(materials.add(Color::srgb_u8(154, 144, 255))),
        Transform::from_xyz(20.0, 0.6, 0.0).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            0.,
            0.3,
            -0.5,
        )),
    ));

    commands.spawn((
        Name::new("Cube1"),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(154, 144, 255))),
        Transform::from_xyz(0.0, 0.6, 0.0),
    ));

    commands.spawn((
        Name::new("Cube1"),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(154, 144, 255))),
        Transform::from_xyz(4.0, 0.6, 3.0),
    ));

     */

    commands.spawn((
        Name::new("Big Cube"),
        RigidBody::Static,
        Collider::cuboid(10.0, 10.0, 10.0),
        Mesh3d(meshes.add(Cuboid::from_length(10.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(154, 144, 255))),
        Transform::from_xyz(2.0, 5.0, -17.0),
    ));

    /*
    commands.spawn((
        Name::new("Cube2"),
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        AngularVelocity(Vec3::new(2.5, 3.5, 1.5)),
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 4.0, 0.0),
    ));
    commands.spawn((
        Name::new("Cube3"),
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        AngularVelocity(Vec3::new(2.5, 3.5, 1.5)),
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));

    commands.spawn((
        Name::new("Cube4"),
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        AngularVelocity(Vec3::new(2.5, 3.5, 1.5)),
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 16.0, 0.0),
    ));

     */
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 83.0),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(25.0, -5.0, 25.0),
    ));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(17.0, -2.0, 75.0),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(14.0, 8.0, 4.0),
    ));
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    commands
        .spawn((
            Name::new("Player entity"),
            Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            Transform::from_xyz(0.0, 1.5, 0.0),
            NotShadowCaster,
            CharacterControllerBundle::new(Collider::cuboid(1.0, 2.0, 1.0)).with_movement(
                50.0,
                0.94,
                7.0,
                (20.0 as Scalar).to_radians(),
            ),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            GravityScale(2.0),
            PlayerMarker,
            //TransformInterpolation,
            CameraSensitivity::default(),
        ))
        .add_child(world_cam)
        .add_child(view_model_cam)
        .add_child(arm);
}

#[derive(Component)]
pub struct Rocket;

#[derive(Event, Clone)]
pub struct RocketExplosion {
    pos: Vec3,
    ent: Entity,
}

fn handle_rocket_collision(
    collisions: Res<Collisions>,
    rockets: Query<(Entity, &Transform), With<Rocket>>,
    collider_parents: Query<&ColliderParent, Without<Sensor>>,
    mut explosion: EventWriter<RocketExplosion>,
) {
    for contacts in collisions.iter() {
        let Ok([collider_parent1, collider_parent2]) =
            collider_parents.get_many([contacts.entity1, contacts.entity2])
        else {
            continue;
        };

        let (ent, rocket_tf) = if let Ok((ent, rocket_tf)) = rockets.get(collider_parent1.get()) {
            (ent, rocket_tf)
        } else if let Ok((ent, rocket_tf)) = rockets.get(collider_parent2.get()) {
            (ent, rocket_tf)
        } else {
            continue;
        };

        explosion.send(RocketExplosion {
            pos: rocket_tf.translation,
            ent,
        });
    }
}

#[derive(Default)]
pub struct PreviousImpulses {
    pub start: Vec<Vec3>,
    pub end: Vec<Vec3>,
}

fn handle_rocket_explosion(
    mut explosion: EventReader<RocketExplosion>,
    mut commands: Commands,
    mut players_q: Query<(&mut LinearVelocity, &Transform, &mut Health), With<PlayerMarker>>,
    server: Option<ResMut<RenetServer>>,
) {
    for ev in explosion.read() {
        debug!("explosion at {:?}", ev.pos);
        commands.entity(ev.ent).despawn();

        for (mut player_vel, player_tf, mut player_health) in players_q.iter_mut() {
            debug!("player health {:?}", player_health);
            if player_tf.translation.distance(ev.pos) <= ROCKET_EXPLOSION_RADIUS {
                let distance = player_tf.translation - ev.pos;
                let normalized_impulse = distance.normalize();

                //player_vel.0 += normalized_impulse * ROCKET_EXPLOSION_FORCE * (1.0 / distance.norm_squared());
                player_vel.0 += normalized_impulse * ROCKET_EXPLOSION_FORCE;

                if server.is_some() {
                    let damage = (MAX_ROCKET_DAMAGE as f32
                        * (distance.norm() / ROCKET_EXPLOSION_RADIUS))
                        as usize;
                    debug!("Damage computed: {:?}", damage);
                    //player_health.0 = player_health.0.saturating_sub(damage);
                }
                debug!(
                    "impulse vector {:?}",
                    normalized_impulse * ROCKET_EXPLOSION_FORCE
                );
            }
        }
    }
}

#[derive(Default)]
pub struct PreviousExplosions {
    pub explosions: Vec<Vec3>,
}

fn debug_rocket_explosion(
    mut explosion: EventReader<RocketExplosion>,
    mut gizmos: Gizmos,
    mut previous_explosions: Local<PreviousExplosions>,
    mut previous_impulses: Local<PreviousImpulses>,
    mut players_q: Query<&Transform, With<PlayerMarker>>,
) {
    previous_explosions.explosions.iter().for_each(|pos| {
        gizmos.sphere(*pos, ROCKET_EXPLOSION_RADIUS, PURPLE);
    });
    previous_impulses
        .start
        .iter()
        .enumerate()
        .for_each(|(i, _)| {
            gizmos.line(previous_impulses.start[i], previous_impulses.end[i], GREEN);
        });
    for ev in explosion.read() {
        // todo make this server authoritative.
        // it sometimes collides only on the server but then the client will not see the explosion
        debug!("explosion at {:?}", ev.pos);
        previous_explosions.explosions.push(ev.pos);

        for player_tf in players_q.iter_mut() {
            if player_tf.translation.distance(ev.pos) <= ROCKET_EXPLOSION_RADIUS {
                previous_impulses.start.push(ev.pos);
                previous_impulses.end.push(player_tf.translation);
            }
        }
    }
}

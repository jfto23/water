use crate::camera::*;
use crate::consts::*;
use avian3d::math::Scalar;
use avian3d::prelude::*;
use bevy::{
    color::palettes::css::WHITE,
    pbr::{wireframe::Wireframe, NotShadowCaster},
    prelude::*,
};

use crate::{
    camera::{CameraSensitivity, PlayerMarker},
    character::*,
};
use bevy::render::view::RenderLayers;

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (water_setup /*spawn_player*/,));
    }
}
fn water_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Floor"),
        RigidBody::Static,
        Collider::cylinder(30.0, 0.1),
        Mesh3d(meshes.add(Cylinder::new(30.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
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

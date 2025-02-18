use bevy::prelude::*;
use bevy::render::mesh::PlaneMeshBuilder;
use bevy::render::{
    mesh::{Indices, VertexAttributeValues},
    render_asset::RenderAssetUsages,
    render_resource::PrimitiveTopology,
};

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
            VertexFormat,
        },
    },
};

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, water_setup);
    }
}

#[derive(Debug, Copy, Clone)]
struct MyPlane {
    size: f32,
    num_vertices: u32,
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
pub struct WaterMaterial {
    #[uniform(0)]
    color: LinearRgba,
    //#[uniform(1)]
    //ship_position: Vec3,
}

const SHADER_ASSET_PATH: &str = "shaders/shader.wgsl";

const ATTRIBUTE_BLEND_COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("BlendColor", 988540917, VertexFormat::Float32x4);

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_BLEND_COLOR.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

fn water_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_material: ResMut<Assets<WaterMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    /*
    // land
    let mut land = MyPlane {
        size: 1000.0,
        num_vertices: 1000,
    }
    .create_mesh();
    let mesh_handle: Handle<Mesh> = meshes.add(land);
     */

    let plane = PlaneMeshBuilder::new(Dir3::Y, Vec2::splat(100.));
    //plane.subdivisions(10);
    let mut mesh = plane.build();
    mesh.insert_attribute(ATTRIBUTE_BLEND_COLOR, vec![[1.0, 0.0, 0.0, 1.0]; 4]);
    let mesh_handle: Handle<Mesh> = meshes.add(mesh);

    // Render the mesh with the custom texture, and add the marker.
    commands.spawn((
        Name::new("water mesh"),
        Mesh3d(mesh_handle),
        MeshMaterial3d(water_material.add(WaterMaterial {
            color: LinearRgba::RED,
        })),
        Transform::from_xyz(-2., 1., 1.0),
    ));
}

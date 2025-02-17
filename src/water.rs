use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::{
    mesh::{Indices, VertexAttributeValues},
    render_asset::RenderAssetUsages,
    render_resource::PrimitiveTopology,
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
#[derive(AsBindGroup, Debug, Clone)]
pub struct MyPlaneMaterial {
    #[uniform(0)]
    time: f32,
    #[uniform(1)]
    ship_position: Vec3,
}

fn water_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    // land
    let mut land = MyPlane {
        size: 1000.0,
        num_vertices: 1000,
    }
    .create_mesh();

    let cube_mesh_handle: Handle<Mesh> = meshes.add(land);

    // Render the mesh with the custom texture, and add the marker.
    commands.spawn((
        Name::new("custom mesh"),
        Mesh3d(cube_mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            //base_color_texture: Some(custom_texture_handle),
            ..default()
        })),
        Transform::from_xyz(-2., 1., 1.0),
    ));
}

impl MyPlane {
    fn create_mesh(&self) -> Mesh {
        // Keep the mesh data accessible in future frames to be able to mutate it in toggle_texture.
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            // Each array is an [x, y, z] coordinate in local space.
            // The camera coordinate space is right-handed x-right, y-up, z-back. This means "forward" is -Z.
            // Meshes always rotate around their local [0, 0, 0] when a rotation is applied to their Transform.
            // By centering our mesh around the origin, rotating the mesh preserves its center of mass.
            vec![
                // top (facing towards +y)
                [-0.5, 0.5, -0.5], // vertex with index 0
                [0.5, 0.5, -0.5],  // vertex with index 1
                [0.5, 0.5, 0.5],   // etc. until 23
                [-0.5, 0.5, 0.5],
                // bottom   (-y)
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
                // right    (+x)
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5], // This vertex is at the same position as vertex with index 2, but they'll have different UV and normal
                [0.5, 0.5, -0.5],
                // left     (-x)
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
                // back     (+z)
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, -0.5, 0.5],
                // forward  (-z)
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
            ],
        )
        // Set-up UV coordinates to point to the upper (V < 0.5), "dirt+grass" part of the texture.
        // Take a look at the custom image (assets/textures/array_texture.png)
        // so the UV coords will make more sense
        // Note: (0.0, 0.0) = Top-Left in UV mapping, (1.0, 1.0) = Bottom-Right in UV mapping
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![
                // Assigning the UV coords for the top side.
                [0.0, 0.2],
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.2],
                // Assigning the UV coords for the bottom side.
                [0.0, 0.45],
                [0.0, 0.25],
                [1.0, 0.25],
                [1.0, 0.45],
                // Assigning the UV coords for the right side.
                [1.0, 0.45],
                [0.0, 0.45],
                [0.0, 0.2],
                [1.0, 0.2],
                // Assigning the UV coords for the left side.
                [1.0, 0.45],
                [0.0, 0.45],
                [0.0, 0.2],
                [1.0, 0.2],
                // Assigning the UV coords for the back side.
                [0.0, 0.45],
                [0.0, 0.2],
                [1.0, 0.2],
                [1.0, 0.45],
                // Assigning the UV coords for the forward side.
                [0.0, 0.45],
                [0.0, 0.2],
                [1.0, 0.2],
                [1.0, 0.45],
            ],
        )
        // For meshes with flat shading, normals are orthogonal (pointing out) from the direction of
        // the surface.
        // Normals are required for correct lighting calculations.
        // Each array represents a normalized vector, which length should be equal to 1.0.
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![
                // Normals for the top side (towards +y)
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                // Normals for the bottom side (towards -y)
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                // Normals for the right side (towards +x)
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                // Normals for the left side (towards -x)
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                // Normals for the back side (towards +z)
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                // Normals for the forward side (towards -z)
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ],
        )
        // Create the triangles out of the 24 vertices we created.
        // To construct a square, we need 2 triangles, therefore 12 triangles in total.
        // To construct a triangle, we need the indices of its 3 defined vertices, adding them one
        // by one, in a counter-clockwise order (relative to the position of the viewer, the order
        // should appear counter-clockwise from the front of the triangle, in this case from outside the cube).
        // Read more about how to correctly build a mesh manually in the Bevy documentation of a Mesh,
        // further examples and the implementation of the built-in shapes.
        //
        // The first two defined triangles look like this (marked with the vertex indices,
        // and the axis), when looking down at the top (+y) of the cube:
        //   -Z
        //   ^
        // 0---1
        // |  /|
        // | / | -> +X
        // |/  |
        // 3---2
        //
        // The right face's (+x) triangles look like this, seen from the outside of the cube.
        //   +Y
        //   ^
        // 10--11
        // |  /|
        // | / | -> -Z
        // |/  |
        // 9---8
        //
        // The back face's (+z) triangles look like this, seen from the outside of the cube.
        //   +Y
        //   ^
        // 17--18
        // |\  |
        // | \ | -> +X
        // |  \|
        // 16--19
        .with_inserted_indices(Indices::U32(vec![
            0, 3, 1, 1, 3, 2, // triangles making up the top (+y) facing side.
            4, 5, 7, 5, 6, 7, // bottom (-y)
            8, 11, 9, 9, 11, 10, // right (+x)
            12, 13, 15, 13, 14, 15, // left (-x)
            16, 19, 17, 17, 19, 18, // back (+z)
            20, 21, 23, 21, 22, 23, // forward (-z)
        ]))
    }
}

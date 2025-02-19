use avian3d::PhysicsPlugins;
use bevy::color::palettes::css::GREEN;
use bevy::render::RenderPlugin;
use bevy::{prelude::*, window::WindowResolution};

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::render::{render_resource::WgpuFeatures, settings::WgpuSettings};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

mod camera;
mod character;
mod consts;
mod water;

#[derive(Resource, Deref, DerefMut)]
pub struct RngResource(StdRng);

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    Main,
    Debug,
}

fn main() {
    let mut app = App::new();
    let mut rng = rand::thread_rng();

    app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    #[cfg(debug_assertions)] // debug/dev builds only
    {
        use bevy::diagnostic::LogDiagnosticsPlugin;
        //app.add_plugins(LogDiagnosticsPlugin::default());
    }
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        consts::RES_WIDTH as f32,
                        consts::RES_HEIGHT as f32,
                    )
                    .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(LogPlugin {
                level: Level::DEBUG,
                filter: "wgpu=error,bevy_render=info,bevy_ecs=trace,naga=info".to_string(),
                custom_layer: |_| None,
            })
            .set(RenderPlugin {
                render_creation: bevy::render::settings::RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(WireframePlugin)
    .insert_resource(ClearColor(Color::srgb(0., 0., 0.)))
    .insert_resource(RngResource(StdRng::seed_from_u64(rng.gen::<u64>())));

    app.insert_resource(WireframeConfig {
        global: false,
        default_color: GREEN.into(),
    });

    app.add_plugins(WorldInspectorPlugin::new())
        .add_plugins(camera::CameraPlugin)
        .add_plugins(water::WaterPlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(character::CharacterControllerPlugin)
        .run();
}

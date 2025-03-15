use avian3d::PhysicsPlugins;
use bevy::color::palettes::css::GREEN;
use bevy::render::RenderPlugin;
use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::WindowResolution};

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::Window;
use bevy::render::{render_resource::WgpuFeatures, settings::WgpuSettings};
use bevy_egui::EguiContext;
use bevy_egui::EguiPlugin;
use client::ClientPlugin;
use input::Action;
use leafwing_input_manager::prelude::*;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use server::ServerPlugin;

mod animation;
mod bimap;
mod camera;
mod character;
mod client;
mod console;
mod consts;
mod input;
mod network_visualizer;
mod server;
mod water;
mod menu;

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
        app.add_plugins(LogDiagnosticsPlugin::default());
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
                filter: "wgpu=error,bevy_render=info,bevy_ecs=trace,naga=info,leafwing_input_manager=info".to_string(),
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

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        panic!("No argument found, pass either client or server");
    }

    let exec_type = &args[1];
    let is_host = match exec_type.as_str() {
        "client" => false,
        "server" => true,
        _ => panic!("Invalid argument, must be \"client\" or \"server\"."),
    };

    debug!("is_host: {:?}", is_host);

    if is_host {
        debug!("Adding ServerPlugin to app");
        app.add_plugins(ServerPlugin);
    } else {
        debug!("Adding ClientPlugin to app");
        app.add_plugins(ClientPlugin);
    }

    app.insert_resource(WireframeConfig {
        global: false,
        default_color: GREEN.into(),
    });

    app.add_plugins(EguiPlugin)
        .add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(water::WaterPlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(character::CharacterControllerPlugin)
        .add_plugins(input::InputPlugin)
        .add_systems(Update, inspector_ui)
        .add_plugins(console::ConsolePlugin)
        .add_plugins(animation::AnimationPlugin)
        .run();
}

fn inspector_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("UI").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // equivalent to `WorldInspectorPlugin`
            bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);

            egui::CollapsingHeader::new("Materials").show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_assets::<StandardMaterial>(world, ui);
            });

            ui.heading("Entities");
            bevy_inspector_egui::bevy_inspector::ui_for_entities(world, ui);
        });
    });
}

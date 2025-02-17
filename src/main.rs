use bevy::{prelude::*, window::WindowResolution};

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

mod camera;
mod consts;

#[derive(Resource, Deref, DerefMut)]
pub struct RngResource(StdRng);

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
            }),
    )
    .insert_resource(ClearColor(Color::srgb(0., 0., 0.)))
    .insert_resource(RngResource(StdRng::seed_from_u64(rng.gen::<u64>())));

    app.add_plugins(WorldInspectorPlugin::new());

    app.add_plugins(camera::CameraPlugin);
    app.run();
}

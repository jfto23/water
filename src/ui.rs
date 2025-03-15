pub struct UiPlugin;

use bevy::{prelude::*, window::WindowResized};

use crate::{
    consts::{RES_HEIGHT, RES_WIDTH},
    menu::despawn_screen,
    water::GameState,
};

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_ui_camera);
        app.add_systems(OnExit(GameState::MainMenu), despawn_screen::<UiCamera>);
    }
}

#[derive(Component)]
pub struct UiCamera;

// for now, used for menu
fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Ui Camera"),
        UiCamera,
        Camera2d::default(),
        Transform::default(),
    ));
}

/*
 * todo implement ui scaling
fn scale_ui(
    mut ui_scale: ResMut<UiScale>,
    projections: Query<&OrthographicProjection, With<UiCamera>>,
) {
    if let Ok(cam) = projections.get_single() {
        ui_scale.0 = if cam.scale.is_infinite() {
            1.
        } else {
            1. / cam.scale
        }
    }
}

*/

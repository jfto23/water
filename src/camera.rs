pub struct CameraPlugin;

use bevy::time::common_conditions::on_timer;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use bevy_renet::renet::RenetClient;

use crate::character::PlayerAction;
use crate::client::{
    ClientChannel, ClientLookDirection, ClientMouseMovement, ControlledPlayer, CurrentClientId,
};
use crate::input::LookDirection;
use crate::AppState;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (cursor_grab));
        app.add_systems(Update, (input, toggle_cursor_grab));
        app.add_systems(
            Update,
            (camera_look_around /*camera_look_around_network*/,).run_if(in_state(AppState::Main)),
        );
        app.add_systems(OnEnter(AppState::Main), toggle_cursor_grab);
        app.add_systems(OnEnter(AppState::Debug), toggle_cursor_grab);
        app.add_systems(PostUpdate, sync_look_direction);
        app.add_systems(FixedUpdate, send_look_direction);
        app.add_event::<ClientMouseMovement>();

        app.insert_state(AppState::Main);
    }
}

#[derive(Component)]
pub struct PlayerMarker;

#[derive(Component)]
pub struct WorldCamera;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            // These factors are just arbitrary mouse sensitivity values.
            // It's often nicer to have a faster horizontal sensitivity than vertical.
            // We use a component for them so that we can make them user-configurable at runtime
            // for accessibility reasons.
            // It also allows you to inspect them in an editor if you `Reflect` the component.
            Vec2::new(0.003, 0.003),
        )
    }
}

pub const CAMERA_SPEED: f32 = 10.;

fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    //mut camera_q: Query<&mut Transform, With<MyCamera>>,
    mut app_state: ResMut<NextState<AppState>>,
    current_app_state: Res<State<AppState>>,
) {
    /*
    let mut camera_tf = camera_q.single_mut();
    let forward = camera_tf.forward().normalize();
    let right = camera_tf.right().normalize();

    if keys.pressed(KeyCode::KeyW) {
        camera_tf.translation += forward * CAMERA_SPEED * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyA) {
        camera_tf.translation -= right * CAMERA_SPEED * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyS) {
        camera_tf.translation -= forward * CAMERA_SPEED * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyD) {
        camera_tf.translation += right * CAMERA_SPEED * time.delta_secs();
    }
     */
    if keys.just_pressed(KeyCode::Escape) {
        if let AppState::Main = *current_app_state.get() {
            debug!("Entered debug mode");
            app_state.set(AppState::Debug);
        } else {
            debug!("Entered main mode");
            app_state.set(AppState::Main);
        }
    }
}

// from https://bevyengine.org/examples/camera/first-person-view-model/
fn camera_look_around(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player_q: Query<(&Transform, &CameraSensitivity), With<ControlledPlayer>>,
    mut camera_q: Query<(&mut Transform), (With<WorldCamera>, Without<ControlledPlayer>)>,
    mut movement_action: EventWriter<PlayerAction>,
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    let Ok((transform, camera_sensitivity)) = player_q.get_single() else {
        return;
    };
    let Ok((mut camera_tf)) = camera_q.get_single_mut() else {
        return;
    };
    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        // Note that we are not multiplying by delta_time here.
        // The reason is that for mouse movement, we already get the full movement that happened since the last frame.
        // This means that if we multiply by delta_time, we will get a smaller rotation than intended by the user.
        // This situation is reversed when reading e.g. analog input from a gamepad however, where the same rules
        // as for keyboard input apply. Such an input should be multiplied by delta_time to get the intended rotation
        // independent of the framerate.
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        // If the pitch was ±¹⁄₂ π, the camera would look straight up or down.
        // When the user wants to move the camera back to the horizon, which way should the camera face?
        // The camera has no way of knowing what direction was "forward" before landing in that extreme position,
        // so the direction picked will for all intents and purposes be arbitrary.
        // Another issue is that for mathematical reasons, the yaw will effectively be flipped when the pitch is at the extremes.
        // To not run into these issues, we clamp the pitch to a safe range.
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        let rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0., roll);
        //transform.rotation = rotation;
        movement_action.send(PlayerAction::Rotate(rotation.to_array()));

        let (yaw, pitch, roll) = camera_tf.rotation.to_euler(EulerRot::YXZ);
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        let input_message = bincode::serialize(&ClientMouseMovement {
            rotation,
            client_id: client_id.0.into(),
        })
        .unwrap();
        client.send_message(ClientChannel::MouseInput, input_message);

        camera_tf.rotation = Quat::from_euler(EulerRot::YXZ, 0., pitch, 0.);
        //debug!("camera_tf.rotation: {:?}", transform.rotation);
    }
}

fn sync_look_direction(
    mut look_direction_q: Query<&mut LookDirection, With<ControlledPlayer>>,
    mut camera_q: Query<(&mut GlobalTransform), With<WorldCamera>>,
) {
    let Ok(mut look_dir) = look_direction_q.get_single_mut() else {
        return;
    };
    let Ok(mut camera_global_tf) = camera_q.get_single_mut() else {
        return;
    };

    look_dir.0 = camera_global_tf.forward().normalize();
    //debug!("look_dir: {:?}", look_dir);
}

fn send_look_direction(
    look_direction_q: Query<&LookDirection, With<ControlledPlayer>>,
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    for look_dir in look_direction_q.iter() {
        let input_message = bincode::serialize(&ClientLookDirection {
            dir: look_dir.0,
            client_id: client_id.0.into(),
        })
        .unwrap();
        client.send_message(ClientChannel::ClientData, input_message);
    }
}

/*
fn camera_look_around_network(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
    player_q: Query<&CameraSensitivity, With<ControlledPlayer>>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    let Ok(camera_sensitivity) = player_q.get_single() else {
        return;
    };
    let delta = accumulated_mouse_motion.delta;

    if delta == Vec2::ZERO {
        return;
    }
    //https://gamedev.stackexchange.com/questions/118981/sending-a-players-mouse-movement-to-the-server-in-an-fps
    let input_message = bincode::serialize(&ClientMouseMovement {
        mouse_delta: delta * camera_sensitivity.0,
        client_id: client_id.0.into(),
    })
    .unwrap();
    client.send_message(ClientChannel::MouseInput, input_message);
}

*/

fn cursor_grab(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();

    primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;

    // also hide the cursor
    primary_window.cursor_options.visible = false;
}

fn toggle_cursor_grab(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    current_app_state: Res<State<AppState>>,
) {
    let mut primary_window = q_windows.single_mut();
    if let AppState::Main = *current_app_state.get() {
        primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor_options.visible = false;
    } else {
        primary_window.cursor_options.grab_mode = CursorGrabMode::None;
        primary_window.cursor_options.visible = true;
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Server Free Cam"),
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraSensitivity::default(),
    ));
}

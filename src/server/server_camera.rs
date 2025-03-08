use std::f32::consts::FRAC_PI_2;

use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

use crate::{camera::CameraSensitivity, consts::SERVER_CAMERA_SPEED};

#[derive(Component)]
pub struct ServerCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Server Free Cam"),
        ServerCamera,
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraSensitivity::default(),
    ));
}

pub fn server_camera_controller(
    mut camera_q: Query<&mut Transform, With<ServerCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok(mut camera_tf) = camera_q.get_single_mut() else {
        return;
    };

    if keys.pressed(KeyCode::KeyW) {
        let dir = camera_tf.forward().as_vec3();
        camera_tf.translation += time.delta_secs() * dir * SERVER_CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyA) {
        let dir = camera_tf.right().as_vec3();
        camera_tf.translation -= time.delta_secs() * dir * SERVER_CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyS) {
        let dir = camera_tf.forward().as_vec3();
        camera_tf.translation -= time.delta_secs() * dir * SERVER_CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyD) {
        let dir = camera_tf.right().as_vec3();
        camera_tf.translation += time.delta_secs() * dir * SERVER_CAMERA_SPEED;
    }
}

pub fn server_camera_look(
    mut camera_q: Query<(&mut Transform, &CameraSensitivity), With<ServerCamera>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
) {
    let Ok((mut camera_tf, camera_sensitivity)) = camera_q.get_single_mut() else {
        return;
    };
    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = camera_tf.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        camera_tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

        //debug!("camera_tf.rotation: {:?}", transform.rotation);
    }
}

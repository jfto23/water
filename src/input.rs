use crate::character::*;
use crate::client::ClientChannel;
use crate::client::*;
use avian3d::math::{Scalar, Vector3};
use bevy::{prelude::*, utils::HashMap};
use bevy_renet::renet::RenetClient;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (read_input_map).chain());
    }
}

// global normalized direction vector of the intent of the player. gets after reading input_map
#[derive(Component, Default, Debug, Clone)]
pub struct MovementIntent(pub Vec3);

// synchronized to camera.forward()
#[derive(Component, Default, Debug, Clone)]
pub struct LookDirection(pub Vec3);

fn read_input_map(
    mut movement_event_writer: EventWriter<PlayerAction>,
    mut player_q: Query<
        (&GlobalTransform, &ActionState<Action>, &mut MovementIntent),
        With<ControlledPlayer>,
    >,
) {
    let Ok((global_player_tf, action_state, mut move_intent)) = player_q.get_single_mut() else {
        return;
    };

    let forward = action_state.pressed(&Action::Forward);
    let left = action_state.pressed(&Action::Left);
    let back = action_state.pressed(&Action::Back);
    let right = action_state.pressed(&Action::Right);

    let x_axis = right as i8 - left as i8;
    let z_axis = back as i8 - forward as i8;
    let local_direction =
        Vector3::new(x_axis as Scalar, 0.0 as Scalar, z_axis as Scalar).clamp_length_max(1.0);

    let mut global_direction = global_player_tf.affine().transform_vector3(local_direction);

    global_direction.y = 0.0;
    global_direction = global_direction.normalize();

    //debug!("move_intent {:?}", move_intent);

    /*
    debug!(
        "local_direction: {:?}, global_direction: {:?}",
        local_direction, global_direction,
    );
     */
    if local_direction != Vector3::ZERO {
        move_intent.0 = global_direction;
    } else {
        move_intent.0 = Vector3::ZERO;
    }

    if action_state.just_pressed(&Action::Jump) {
        movement_event_writer.send(PlayerAction::Jump);
    }
}

fn generate_client_button_state(
    just_pressed: bool,
    just_released: bool,
    client_input: ClientInput,
) -> Vec<ClientButtonState> {
    let mut inputs = Vec::new();
    if just_pressed {
        inputs.push(ClientButtonState::Pressed(client_input));
    }
    if just_released {
        inputs.push(ClientButtonState::Released(client_input));
    }
    return inputs;
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
pub enum Action {
    Forward,
    Left,
    Back,
    Right,
    Shoot,
    Jump,
}

pub fn build_input_map() -> InputMap<Action> {
    let input_map = InputMap::new([
        (Action::Jump, KeyCode::Space),
        (Action::Forward, KeyCode::KeyW),
        (Action::Left, KeyCode::KeyA),
        (Action::Back, KeyCode::KeyS),
        (Action::Right, KeyCode::KeyD),
    ])
    .with(Action::Shoot, MouseButton::Left);

    return input_map;
}

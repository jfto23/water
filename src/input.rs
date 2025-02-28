use crate::character::*;
use crate::client::ClientChannel;
use crate::client::*;
use avian3d::math::{Scalar, Vector3};
use avian3d::prelude::*;
use bevy::{prelude::*, utils::HashMap};
use bevy_renet::renet::RenetClient;
use serde::{Deserialize, Serialize};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_input_map, read_input_map, keyboard_network_input).chain(),
        );
    }
}

// tracks pressed and release events
#[derive(Component, Default, Debug, Clone, Serialize, Deserialize)]
pub struct InputMap(pub HashMap<ClientInput, bool>);

// global normalized direction vector of the intent of the player. gets after reading input_map
#[derive(Component, Default, Debug, Clone)]
pub struct MovementIntent(pub Vec3);

fn update_input_map(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_q: Query<&mut InputMap, With<ControlledPlayer>>,
) {
    let Ok((mut input_map)) = input_q.get_single_mut() else {
        return;
    };
    let forward = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let back = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let jump = keyboard_input.any_pressed([KeyCode::Space]);

    input_map.0.insert(ClientInput::Forward, forward);
    input_map.0.insert(ClientInput::Back, back);
    input_map.0.insert(ClientInput::Left, left);
    input_map.0.insert(ClientInput::Right, right);
    input_map.0.insert(ClientInput::Jump, jump);
    //debug!("inputmap: {:?}", input_map);
}

fn read_input_map(
    mut movement_event_writer: EventWriter<MovementAction>,
    mut player_q: Query<
        (
            &Transform,
            &GlobalTransform,
            &mut InputMap,
            &mut MovementIntent,
        ),
        With<ControlledPlayer>,
    >,
) {
    let Ok((player_tf, global_player_tf, mut input_map, mut move_intent)) =
        player_q.get_single_mut()
    else {
        return;
    };

    let x_axis = *input_map.0.get(&ClientInput::Right).unwrap() as i8
        - *input_map.0.get(&ClientInput::Left).unwrap() as i8;
    let z_axis = *input_map.0.get(&ClientInput::Back).unwrap() as i8
        - *input_map.0.get(&ClientInput::Forward).unwrap() as i8;
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
        movement_event_writer.send(MovementAction::Move(global_direction.to_array()));
        move_intent.0 = global_direction;
    } else {
        move_intent.0 = Vector3::ZERO;
    }

    if input_map
        .0
        .get(&ClientInput::Jump)
        .is_some_and(|inner| *inner)
    {
        movement_event_writer.send(MovementAction::Jump);
    }
}

/*

fn send_input_map(
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
    player_q: Query<(&InputMap), With<ControlledPlayer>>,
) {
    let Ok(input_map) = player_q.get_single() else {
        return;
    };
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    let input_message = bincode::serialize(&ClientMovement {
        client_input: input_map.clone(),
        client_id: client_id.0.into(),
    })
    .unwrap();
    client.send_message(ClientChannel::Input, input_message);
}
*/

fn keyboard_network_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_q: Query<(&Transform, &GlobalTransform), With<ControlledPlayer>>,
    mut client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
) {
    // TODO CLEAN THIS MESS XDDDDDDDD
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    let forward_pressed = keyboard_input.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let back_pressed = keyboard_input.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left_pressed = keyboard_input.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right_pressed = keyboard_input.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let jump_pressed = keyboard_input.any_just_pressed([KeyCode::Space]);

    let forward_released = keyboard_input.any_just_released([KeyCode::KeyW, KeyCode::ArrowUp]);
    let back_released = keyboard_input.any_just_released([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left_released = keyboard_input.any_just_released([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right_released = keyboard_input.any_just_released([KeyCode::KeyD, KeyCode::ArrowRight]);
    let jump_released = keyboard_input.any_just_released([KeyCode::Space]);

    //todo: I dont think we need to track jump pressed/unpressed. just consider every single press
    [
        generate_client_button_state(forward_pressed, forward_released, ClientInput::Forward),
        generate_client_button_state(back_pressed, back_released, ClientInput::Back),
        generate_client_button_state(left_pressed, left_released, ClientInput::Left),
        generate_client_button_state(right_pressed, right_released, ClientInput::Right),
        generate_client_button_state(jump_pressed, jump_released, ClientInput::Jump),
    ]
    .into_iter()
    .flatten()
    .for_each(|button_state: ClientButtonState| {
        let input_message = bincode::serialize(&ClientMovement {
            button_state,
            client_id: client_id.0.into(),
        })
        .unwrap();
        client.send_message(ClientChannel::Input, input_message);
        // todo:
        // this is not good, if the packets are lost, the server will not have accurate input map,
        // we should probably store input on client as well. Update the input on change and maybe every second
        // to make sure it's synced with server
        //

        // COMBINE THIS FUNCTION WITH THE ONE ABOVE THIS IS ALL MESSED UP ASJDHKLAWJDAWDAWKLJKLJ
    });

    //https://gamedev.stackexchange.com/questions/74655/what-to-send-to-server-in-real-time-fps-game
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

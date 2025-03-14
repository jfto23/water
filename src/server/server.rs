use avian3d::{
    math::{Scalar, Vector3},
    parry::utils::hashmap::HashMap,
    prelude::{
        CoefficientCombine, Collider, Friction, GravityScale, LinearVelocity, Restitution,
        RigidBody, TransformInterpolation,
    },
};
use bevy_egui::EguiContexts;
use leafwing_input_manager::prelude::ActionState;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::PI,
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use bevy::{pbr::NotShadowCaster, prelude::*, time::common_conditions::on_timer};
use bevy_renet::{
    netcode::{NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::{
        ChannelConfig, ClientId, ConnectionConfig, DefaultChannel, RenetServer, SendType,
        ServerEvent,
    },
    RenetServerPlugin,
};

use crate::{
    camera::{CameraSensitivity, PlayerMarker},
    character::*,
    client::{ClientAction, ClientChannel, ClientInput, ClientLookDirection, ClientMouseMovement},
    consts::{
        CHARACTER_MODEL_PATH, PLAYER_DEATH_TIMER, PLAYER_HEALTH, ROCKET_SPEED, SHOOT_COOLDOWN,
    },
    input::{build_input_map, Action, LookDirection, MovementIntent},
    water::Rocket,
    AppState,
};
use leafwing_input_manager::prelude::*;

use crate::network_visualizer::visualizer::RenetServerVisualizer;

use super::server_camera::*;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetServerPlugin);

        let server = RenetServer::new(connection_config());
        app.insert_resource(server);
        app.insert_resource(ServerLobby::default());
        // Transport layer setup
        app.add_plugins(NetcodeServerPlugin);
        let server_addr = "127.0.0.1:5000".parse().unwrap();
        let socket = UdpSocket::bind(server_addr).unwrap();
        let server_config = ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![server_addr],
            authentication: ServerAuthentication::Unsecure,
        };
        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
        app.insert_resource(transport);

        app.add_plugins(InputManagerPlugin::<Action>::server());

        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            Update,
            (server_camera_controller, server_camera_look).run_if(in_state(AppState::Main)),
        );
        app.insert_resource(RenetServerVisualizer::<200>::default());

        app.add_systems(FixedUpdate, handle_events_system);
        app.add_systems(FixedUpdate, (check_player_death, respawn_player));
        app.add_systems(
            FixedUpdate,
            (
                handle_server_player_action,
                server_mouse,
                tick_shoot_cooldown, //server_network_sync,
            ), //.after(handle_events_system)
               //.chain(),
        );
        //app.add_systems(FixedUpdate, server_mouse.after(handle_events_system));

        //https://www.reddit.com/r/gamedev/comments/4eigzo/generally_how_often_do_most_realtime_multiplayer/
        app.add_systems(
            Update,
            server_network_sync.run_if(on_timer(Duration::from_millis(50))),
        );

        app.add_systems(Update, update_visualizer_system);

        app.add_systems(PreUpdate, update_client_input_state);
        app.add_systems(FixedUpdate, read_client_input_state);

        app.add_event::<ServerPlayerAction>();
    }
}

fn send_message_system(mut server: ResMut<RenetServer>) {
    let channel_id = 0;
    // Send a text message for all clients
    // The enum DefaultChannel describe the channels used by the default configuration
    //server.broadcast_message(DefaultChannel::ReliableOrdered, "server message");
}

fn receive_message_system(mut server: ResMut<RenetServer>) {
    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            // Handle received message
            debug!("message received: {:?}", message);
        }
    }
}
pub enum ServerChannel {
    ServerMessages,
    NetworkedEntities,
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::ServerMessages => 0,
            ServerChannel::NetworkedEntities => 1,
        }
    }
}

impl ServerChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::ServerMessages.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::NetworkedEntities.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate {
        entity: Entity,
        id: ClientId,
        translation: [f32; 3],
    },
    PlayerRemove {
        id: ClientId,
    },
    BulletCreate {
        translation: [f32; 3],
        dir: [f32; 3], // normalized
    },
    PlayerDeath {
        server_ent: Entity,
        id: u64,
    },
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub velocities: Vec<[f32; 3]>,
    pub health: Vec<usize>,
}

#[derive(Debug, Component)]
pub struct Player {
    pub id: ClientId,
}

#[derive(Debug, Component)]
pub struct WeaponCooldown(pub Timer);

#[derive(Debug, Default, Resource)]
pub struct ServerLobby {
    pub players: HashMap<ClientId, Entity>,
}

fn handle_events_system(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    mut players: Query<(Entity, &Player, &Transform, &mut LookDirection)>,
    mut lobby: ResMut<ServerLobby>,
    mut commands: Commands,
    mut movement_event_writer: EventWriter<ClientAction<Action>>,
    mut mouse_event_writer: EventWriter<ClientMouseMovement>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                debug!("Client {client_id} connected");
                visualizer.add_client(*client_id);

                // Initialize other players for this new client
                for (entity, player, transform, _) in players.iter() {
                    let translation: [f32; 3] = transform.translation.into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation,
                    })
                    .unwrap();
                    server.send_message(*client_id, ServerChannel::ServerMessages, message);
                }

                let player_entity = build_player_ent(
                    &mut commands,
                    &asset_server,
                    *client_id,
                    NetworkScenario::Server,
                    &mut meshes,
                    &mut materials,
                );

                lobby.players.insert(*client_id, player_entity);
                let translation: [f32; 3] = Vec3::new(0.0, 1.5, 0.0).into();
                let message = bincode::serialize(&ServerMessages::PlayerCreate {
                    id: *client_id,
                    entity: player_entity,
                    translation,
                })
                .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                debug!("Client {client_id} disconnected: {reason}");
                visualizer.remove_client(*client_id);
                if let Some(player_entity) = lobby.players.remove(client_id) {
                    commands.entity(player_entity).despawn();
                }

                let message =
                    bincode::serialize(&ServerMessages::PlayerRemove { id: *client_id }).unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
        }
    }

    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Input) {
            let client_move: ClientAction<Action> = bincode::deserialize(&message).unwrap();
            debug!("received ClientMovement {:?}", client_move);
            if let Some(player_entity) = lobby.players.get(&client_move.client_id) {
                movement_event_writer.send(client_move);
            }
        }
        while let Some(message) = server.receive_message(client_id, ClientChannel::MouseInput) {
            let client_mouse: ClientMouseMovement = bincode::deserialize(&message).unwrap();
            debug!("received ClientMouseMovement {:?}", client_mouse);
            if let Some(player_entity) = lobby.players.get(&client_mouse.client_id) {
                mouse_event_writer.send(client_mouse);
            }
        }
        while let Some(message) = server.receive_message(client_id, ClientChannel::ClientData) {
            let client_data: ClientLookDirection = bincode::deserialize(&message).unwrap();
            //debug!("received ClientLookDirection {:?}", client_data);
            if let Some(player_entity) = lobby.players.get(&client_data.client_id) {
                let Ok((_, _, _, mut look_dir)) = players.get_mut(*player_entity) else {
                    continue;
                };
                look_dir.0 = client_data.dir;
            }
        }
    }
}

fn update_client_input_state(
    mut movement_event_reader: EventReader<ClientAction<Action>>,
    //mut controllers: Query<(Entity, &mut ActionState<Action>), With<PlayerMarker>>,
    mut action_state_query: Query<&mut ActionState<Action>, With<PlayerMarker>>,
    server_lobby: Res<ServerLobby>,
) {
    for ev in movement_event_reader.read() {
        debug!("processing client movement event");

        if let Some(player_ent) = server_lobby.players.get(&ev.client_id) {
            if let Ok((mut action_state)) = action_state_query.get_mut(*player_ent) {
                action_state.apply_diff(&ev.action_diff);
            }
        }
    }
}

// reads all clients input maps and sends MovementAction event
fn read_client_input_state(
    mut clients_q: Query<(
        &ActionState<Action>,
        &Transform,
        &GlobalTransform,
        Entity,
        &mut MovementIntent,
    )>,
    mut player_action: EventWriter<ServerPlayerAction>,
) {
    for (action_state, client_tf, client_global_tf, ent, mut move_intent) in clients_q.iter_mut() {
        if action_state.pressed(&Action::Shoot) {
            player_action.send(ServerPlayerAction {
                action: PlayerAction::Shoot,
                ent,
            });
        }
        if action_state.pressed(&Action::Jump) {
            player_action.send(ServerPlayerAction {
                action: PlayerAction::Jump,
                ent,
            });
        }

        let forward = action_state.pressed(&Action::Forward);
        let left = action_state.pressed(&Action::Left);
        let back = action_state.pressed(&Action::Back);
        let right = action_state.pressed(&Action::Right);

        let x_axis = right as i8 - left as i8;
        let z_axis = back as i8 - forward as i8;

        let local_direction =
            Vector3::new(x_axis as Scalar, 0.0 as Scalar, z_axis as Scalar).clamp_length_max(1.0);

        let mut global_direction = client_global_tf.affine().transform_vector3(local_direction);

        global_direction.y = 0.0;
        global_direction = global_direction.normalize();

        if local_direction != Vector3::ZERO {
            move_intent.0 = global_direction;
        } else {
            move_intent.0 = Vector3::ZERO;
        }
    }
}

#[derive(Event, Clone)]
pub struct ServerPlayerAction {
    action: PlayerAction,
    ent: Entity,
}

fn handle_server_player_action(
    time_fixed: Res<Time<Fixed>>,
    mut movement_event_reader: EventReader<ServerPlayerAction>,
    mut controllers: Query<(
        &JumpImpulse,
        &mut LinearVelocity,
        Has<Grounded>,
        &Transform,
        &mut WeaponCooldown,
        &LookDirection,
    )>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let delta_time = time_fixed.delta_secs();
    for event in movement_event_reader.read() {
        //debug!("reading movement action on the server");
        let Ok((
            jump_impulse,
            mut linear_velocity,
            is_grounded,
            player_tf,
            mut weapon_timer,
            look_direction,
        )) = controllers.get_mut(event.ent)
        else {
            continue;
        };
        match event.action {
            PlayerAction::Jump => {
                if is_grounded {
                    linear_velocity.y = jump_impulse.0;
                }
            }
            PlayerAction::Rotate(_) => {}
            PlayerAction::Shoot => {
                if weapon_timer.0.finished() {
                    weapon_timer.0.reset();
                    // todo: should be based on camera, not player_tf
                    let spawn_location =
                        player_tf.translation + (look_direction.0 * 2.0) + Vec3::new(0.0, 0.4, 0.0);
                    let rocket_speed = look_direction.0 * ROCKET_SPEED;
                    commands.spawn((
                        Name::new("Bullet"),
                        Rocket,
                        LinearVelocity(rocket_speed),
                        RigidBody::Kinematic,
                        Collider::cuboid(0.2, 0.2, 0.2),
                        Mesh3d(meshes.add(Cuboid::from_length(0.2))),
                        MeshMaterial3d(materials.add(Color::srgb_u8(154, 109, 100))),
                        Transform::from_translation(spawn_location),
                    ));
                    let message = bincode::serialize(&ServerMessages::BulletCreate {
                        translation: spawn_location.into(),
                        dir: look_direction.0.to_array(),
                    })
                    .unwrap();
                    server.broadcast_message(ServerChannel::ServerMessages, message);
                }
            }
        }
    }
}

fn tick_shoot_cooldown(mut timer_q: Query<&mut WeaponCooldown>, time: Res<Time>) {
    for mut timer in timer_q.iter_mut() {
        timer.0.tick(time.delta());
    }
}

fn server_mouse(
    mut mouse_event: EventReader<ClientMouseMovement>,
    mut player_q: Query<(Entity, &mut Transform, &CameraSensitivity)>,
    server_lobby: Res<ServerLobby>,
) {
    for event in mouse_event.read() {
        for (player_ent, mut player_tf, player_camera_sens) in player_q.iter_mut() {
            if server_lobby
                .players
                .get(&event.client_id)
                .is_some_and(|inner| *inner == player_ent)
            {
                debug!("Mouse movement for {:?} ", player_ent);
                /*

                let delta_yaw = -event.mouse_delta.x;
                let delta_pitch = -event.mouse_delta.y;

                let (yaw, pitch, roll) = player_tf.rotation.to_euler(EulerRot::YXZ);
                let yaw = yaw + delta_yaw;

                const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
                let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
                 */

                player_tf.rotation = event.rotation;
            }
        }
    }
}

fn update_visualizer_system(
    mut egui_contexts: EguiContexts,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    server: Res<RenetServer>,
) {
    visualizer.update(&server);
    visualizer.show_window(egui_contexts.ctx_mut());
}

fn server_network_sync(
    mut server: ResMut<RenetServer>,
    query: Query<(Entity, &Transform, &LinearVelocity, &Health), With<PlayerMarker>>,
) {
    let mut networked_entities = NetworkedEntities::default();
    for (entity, transform, velocity, health) in query.iter() {
        networked_entities.entities.push(entity);
        networked_entities
            .translations
            .push(transform.translation.into());
        networked_entities
            .rotations
            .push(transform.rotation.to_array());
        networked_entities.velocities.push(velocity.to_array());
        networked_entities.health.push(health.0);
    }

    let sync_message = bincode::serialize(&networked_entities).unwrap();
    server.broadcast_message(ServerChannel::NetworkedEntities, sync_message);
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: ClientChannel::channels_config(),
        server_channels_config: ServerChannel::channels_config(),
    }
}

#[derive(Component)]
pub struct DeathTimer {
    pub timer: Timer,
    pub id: u64,
}

pub fn check_player_death(
    player_q: Query<(Entity, &Player, &Health, &Transform), With<PlayerMarker>>,
    mut server: ResMut<RenetServer>,
    mut commands: Commands,
) {
    for (player_ent, player_id, health, player_tf) in player_q.iter() {
        if health.0 == 0 || player_tf.translation.y <= -20.0 {
            commands.entity(player_ent).despawn_recursive();
            let message = bincode::serialize(&ServerMessages::PlayerDeath {
                server_ent: player_ent,
                id: player_id.id,
            })
            .unwrap();
            server.broadcast_message(ServerChannel::ServerMessages, message);

            commands.spawn(DeathTimer {
                timer: Timer::new(Duration::from_secs_f32(PLAYER_DEATH_TIMER), TimerMode::Once),
                id: player_id.id,
            });
        }
    }
}

pub fn respawn_player(
    mut death_timers: Query<(Entity, &mut DeathTimer)>,
    mut server_lobby: ResMut<ServerLobby>,
    time_fixed: Res<Time<Fixed>>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (ent, mut death_timer) in death_timers.iter_mut() {
        death_timer.timer.tick(time_fixed.delta());

        if death_timer.timer.just_finished() {
            commands.entity(ent).despawn();

            let transform = Transform::from_xyz(0.0, 1.5, 0.0);
            let player_entity = build_player_ent(
                &mut commands,
                &asset_server,
                death_timer.id,
                NetworkScenario::Server,
                &mut meshes,
                &mut materials,
            );

            server_lobby.players.insert(death_timer.id, player_entity);
            let translation: [f32; 3] = transform.translation.into();
            let message = bincode::serialize(&ServerMessages::PlayerCreate {
                id: death_timer.id,
                entity: player_entity,
                translation,
            })
            .unwrap();
            server.broadcast_message(ServerChannel::ServerMessages, message);
        }
    }
}

use avian3d::{
    math::Scalar,
    parry::utils::hashmap::HashMap,
    prelude::{CoefficientCombine, Collider, Friction, GravityScale, LinearVelocity, Restitution},
};
use bevy_egui::EguiContexts;
use serde::{Deserialize, Serialize};
use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use bevy::{pbr::NotShadowCaster, prelude::*};
use bevy_renet::{
    netcode::{NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::{
        ChannelConfig, ClientId, ConnectionConfig, DefaultChannel, RenetServer, SendType,
        ServerEvent,
    },
    RenetServerPlugin,
};

use crate::{
    camera::{spawn_camera, CameraSensitivity, PlayerMarker},
    character::*,
    client::{ClientChannel, ClientMovement, ControlledPlayer},
};

use crate::network_visualizer::visualizer::RenetServerVisualizer;

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

        app.add_systems(Startup, spawn_camera);
        app.insert_resource(RenetServerVisualizer::<200>::default());

        app.add_systems(
            FixedUpdate,
            (
                //send_message_system,
                //receive_message_system,
                handle_events_system,
                move_players_system,
                //apply_movement_damping,
                server_network_sync,
            )
                .chain(),
        );

        app.add_systems(Update, update_visualizer_system);
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
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Debug, Component)]
pub struct Player {
    pub id: ClientId,
}

#[derive(Debug, Default, Resource)]
pub struct ServerLobby {
    pub players: HashMap<ClientId, Entity>,
}

fn handle_events_system(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    players: Query<(Entity, &Player, &Transform)>,
    mut lobby: ResMut<ServerLobby>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut movement_event_writer: EventWriter<ClientMovement>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                debug!("Client {client_id} connected");
                visualizer.add_client(*client_id);

                // Initialize other players for this new client
                for (entity, player, transform) in players.iter() {
                    let translation: [f32; 3] = transform.translation.into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation,
                    })
                    .unwrap();
                    server.send_message(*client_id, ServerChannel::ServerMessages, message);
                }

                let transform = Transform::from_xyz(0.0, 1.5, 0.0);
                let player_entity = commands
                    .spawn((
                        Name::new("Player entity"),
                        Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
                        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                        NotShadowCaster,
                        transform,
                        CharacterControllerBundle::new(Collider::cuboid(1.0, 2.0, 1.0))
                            .with_movement(50.0, 0.72, 7.0, (20.0 as Scalar).to_radians()),
                        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
                        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
                        GravityScale(2.0),
                        PlayerMarker,
                        //TransformInterpolation,
                        CameraSensitivity::default(),
                        Player { id: *client_id },
                    ))
                    .id();

                lobby.players.insert(*client_id, player_entity);
                let translation: [f32; 3] = transform.translation.into();
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
            let client_move: ClientMovement = bincode::deserialize(&message).unwrap();
            if let Some(player_entity) = lobby.players.get(&client_move.client_id) {
                movement_event_writer.send(client_move);
            }
        }
    }
}

fn move_players_system(
    mut movement_event_reader: EventReader<ClientMovement>,
    mut controllers: Query<
        (
            Entity,
            &MovementAcceleration,
            &JumpImpulse,
            &mut LinearVelocity,
            &mut Transform,
            Has<Grounded>,
        ),
        With<PlayerMarker>,
    >,
    server_lobby: Res<ServerLobby>,
    time: Res<Time>,
) {
    for ev in movement_event_reader.read() {
        debug!("processing client movement event");
        for (
            player_ent,
            movement_acceleration,
            jump_impulse,
            mut linear_velocity,
            mut tf,
            is_grounded,
        ) in &mut controllers
        {
            if server_lobby
                .players
                .get(&ev.client_id)
                .is_some_and(|inner| *inner == player_ent)
            {
                debug!("Applying server movement on {:?}", player_ent);
                match ev.movement {
                    MovementAction::Move(direction) => {
                        linear_velocity.x +=
                            direction[0] * movement_acceleration.0 * time.delta_secs();
                        linear_velocity.z +=
                            direction[2] * movement_acceleration.0 * time.delta_secs();

                        /*

                        let mut air_acc = air_accelerate(*direction, &linear_velocity);

                        let mut accel_speed = 100. * delta_time;
                        if accel_speed > air_acc {
                            accel_speed = air_acc;
                        }
                        debug!("accell_speed: {:?}", accel_speed);

                        linear_velocity.x += accel_speed;
                        linear_velocity.z += accel_speed;
                         */
                    }
                    MovementAction::Jump => {
                        if is_grounded {
                            linear_velocity.y = jump_impulse.0;
                        }
                    }
                    MovementAction::Rotate(arr) => {
                        tf.rotation = Quat::from_array(arr);
                    }
                }
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
    query: Query<(Entity, &Transform), With<PlayerMarker>>,
) {
    let mut networked_entities = NetworkedEntities::default();
    for (entity, transform) in query.iter() {
        networked_entities.entities.push(entity);
        networked_entities
            .translations
            .push(transform.translation.into());
        networked_entities
            .rotations
            .push(transform.rotation.to_array());
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

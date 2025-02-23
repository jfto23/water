use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use crate::{
    camera::{CameraSensitivity, PlayerMarker},
    character::{CharacterControllerBundle, MovementAction},
    server::{connection_config, NetworkedEntities, Player},
};
use avian3d::{
    math::Scalar,
    prelude::{CoefficientCombine, Collider, Friction, GravityScale, Restitution},
};
use bevy::{pbr::NotShadowCaster, prelude::*, render::view::RenderLayers, utils::HashMap};
use bevy_egui::EguiContexts;
use bevy_renet::{
    netcode::{
        ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport, NetcodeServerPlugin,
        NetcodeServerTransport, ServerAuthentication, ServerConfig,
    },
    renet::{
        ChannelConfig, ClientId, ConnectionConfig, DefaultChannel, RenetClient, RenetServer,
        SendType, ServerEvent,
    },
    RenetClientPlugin, RenetServerPlugin,
};
use serde::{Deserialize, Serialize};

use crate::{
    camera::WorldCamera,
    consts::VIEW_MODEL_RENDER_LAYER,
    server::{ServerChannel, ServerMessages},
};

use crate::network_visualizer::visualizer::{RenetClientVisualizer, RenetVisualizerStyle};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetClientPlugin);

        let client = RenetClient::new(connection_config());
        app.insert_resource(client);

        // Setup the transport layer
        app.add_plugins(NetcodeClientPlugin);

        let server_addr = "127.0.0.1:5000".parse().unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let client_id = current_time.as_millis() as u64;
        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id,
            user_data: None,
            protocol_id: 0,
        };
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let mut transport =
            NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        app.insert_resource(transport);
        app.insert_resource(CurrentClientId(client_id));

        app.insert_resource(ClientLobby::default());
        //app.insert_resource(PlayerInput::default());
        app.insert_resource(NetworkMapping::default());
        app.insert_resource(RenetClientVisualizer::<200>::new(
            RenetVisualizerStyle::default(),
        ));

        app.add_systems(FixedUpdate, (send_message_system, receive_message_system));
        app.add_systems(Update, update_visualizer_system);
    }
}

fn send_message_system(mut client: ResMut<RenetClient>) {
    // Send a text message to the server
    //debug!("Sending dummy message");
    //client.send_message(DefaultChannel::ReliableOrdered, "server message");
}

fn update_visualizer_system(
    mut egui_contexts: EguiContexts,
    mut visualizer: ResMut<RenetClientVisualizer<200>>,
    client: Res<RenetClient>,
    mut show_visualizer: Local<bool>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    visualizer.add_network_info(client.network_info());
    if keyboard_input.just_pressed(KeyCode::F1) {
        *show_visualizer = !*show_visualizer;
    }
    if *show_visualizer {
        visualizer.show_window(egui_contexts.ctx_mut());
    }
}

#[derive(Component)]
pub struct ControlledPlayer;

#[derive(Debug, Default, Resource)]
struct ClientLobby {
    players: HashMap<ClientId, PlayerInfo>,
}
#[derive(Debug, Resource)]
pub struct CurrentClientId(pub u64);

#[derive(Debug)]
struct PlayerInfo {
    client_entity: Entity,
    server_entity: Entity,
}

#[derive(Event, Clone, Deserialize, Serialize)]
pub struct ClientMovement {
    pub movement: MovementAction,
    pub client_id: u64,
}

pub enum ClientChannel {
    Input,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Input => 0,
        }
    }
}
impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![ChannelConfig {
            channel_id: Self::Input.into(),
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::ZERO,
            },
        }]
    }
}

#[derive(Default, Resource)]
// maps from server enttiy to client entity
struct NetworkMapping(HashMap<Entity, Entity>);

fn receive_message_system(
    mut client: ResMut<RenetClient>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    client_id: Res<CurrentClientId>,
    mut players_q: Query<&mut Transform, With<PlayerMarker>>,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                translation,
                entity,
            } => {
                debug!("Player {} connected.", id);
                let client_entity = commands
                    .spawn((
                        Name::new("Player entity"),
                        Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
                        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                        Transform::from_xyz(0.0, 1.5, 0.0),
                        NotShadowCaster,
                        CharacterControllerBundle::new(Collider::cuboid(1.0, 2.0, 1.0))
                            .with_movement(50.0, 0.72, 7.0, (20.0 as Scalar).to_radians()),
                        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
                        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
                        GravityScale(2.0),
                        PlayerMarker,
                        //TransformInterpolation,
                        CameraSensitivity::default(),
                        Player { id },
                    ))
                    .id();

                if client_id.0 == id {
                    debug!("spawning world camera and view model camera");
                    commands.entity(client_entity).insert(ControlledPlayer);
                    let world_cam = commands
                        .spawn((
                            Name::new("World Camera"),
                            WorldCamera,
                            Camera3d::default(),
                            Transform::from_xyz(0., 0.5, 0.),
                            Projection::from(PerspectiveProjection {
                                fov: 90.0_f32.to_radians(),
                                ..default()
                            }),
                        ))
                        .id();
                    commands.entity(client_entity).add_child(world_cam);

                    let view_model_cam = commands
                        .spawn((
                            Name::new("View Model Camera"),
                            Camera3d::default(),
                            Camera {
                                // Bump the order to render on top of the world model.
                                order: 1,
                                ..default()
                            },
                            Projection::from(PerspectiveProjection {
                                fov: 90.0_f32.to_radians(),
                                ..default()
                            }),
                            // Only render objects belonging to the view model.
                            RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                        ))
                        .id();
                    commands.entity(client_entity).add_child(view_model_cam);
                    commands.entity(client_entity).insert(Name::new("MyPlayer"));
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity,
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity);
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {} disconnected.", id);
                if let Some(PlayerInfo {
                    server_entity,
                    client_entity,
                }) = lobby.players.remove(&id)
                {
                    commands.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let networked_entities: NetworkedEntities = bincode::deserialize(&message).unwrap();

        for i in 0..networked_entities.entities.len() {
            if let Some(entity) = network_mapping.0.get(&networked_entities.entities[i]) {
                let translation = networked_entities.translations[i].into();
                let rotation = Quat::from_array(networked_entities.rotations[i]);
                let transform = Transform {
                    translation,
                    //rotation,
                    ..Default::default()
                };
                debug!(
                    "Updating transform of {:?}, New Transform: {:?}",
                    entity, transform
                );

                let Ok(mut player_tf) = players_q.get_mut(*entity) else {
                    continue;
                };

                //player_tf.translation = translation;

                player_tf.translation = player_tf.translation.lerp(translation, 0.5);

                //commands.entity(*entity).insert(transform);
            }
        }
    }
}

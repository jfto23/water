use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use crate::{
    camera::{CameraSensitivity, PlayerMarker},
    character::{CharacterControllerBundle, Health},
    consts::{PLAYER_HEALTH, ROCKET_SPEED},
    input::{build_input_map, Action, LookDirection, MovementIntent},
    server::{connection_config, NetworkedEntities, Player},
    water::Rocket,
    AppState,
};
use avian3d::{
    math::Scalar,
    prelude::{
        CoefficientCombine, Collider, Friction, GravityScale, LinearVelocity, Restitution,
        RigidBody, TransformInterpolation,
    },
};
use bevy::{
    color::palettes::css::WHITE, pbr::NotShadowCaster, prelude::*, render::view::RenderLayers,
    utils::HashMap,
};
use bevy_egui::EguiContexts;
use bevy_renet::{
    netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport},
    renet::{ChannelConfig, ClientId, RenetClient, SendType},
    RenetClientPlugin,
};
use serde::{Deserialize, Serialize};

use crate::{
    camera::WorldCamera,
    consts::VIEW_MODEL_RENDER_LAYER,
    server::{ServerChannel, ServerMessages},
};

use crate::network_visualizer::visualizer::{RenetClientVisualizer, RenetVisualizerStyle};
use leafwing_input_manager::action_diff::{ActionDiff, ActionDiffEvent};
use leafwing_input_manager::prelude::*;
use leafwing_input_manager::systems::generate_action_diffs;

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
        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        app.add_plugins(InputManagerPlugin::<Action>::default());

        app.insert_resource(transport);
        app.insert_resource(CurrentClientId(client_id));

        app.insert_resource(ClientLobby::default());
        //app.insert_resource(PlayerInput::default());
        app.insert_resource(NetworkMapping::default());
        app.insert_resource(RenetClientVisualizer::<200>::new(
            RenetVisualizerStyle::default(),
        ));

        app.add_systems(PostUpdate, generate_action_diffs::<Action>);
        app.add_systems(
            FixedUpdate,
            send_action_diffs::<Action>.run_if(in_state(AppState::Main)),
        );
        app.add_systems(FixedUpdate, (send_message_system, receive_message_system));
        app.add_systems(Update, update_visualizer_system);
        app.add_event::<ActionDiffEvent<Action>>();
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

#[derive(Event, Clone, Deserialize, Serialize, Debug)]
pub struct ClientAction<A: Actionlike> {
    pub action_diff: ActionDiff<A>,
    pub client_id: u64,
}

#[derive(Event, Clone, Deserialize, Serialize, Debug)]
pub struct ClientMouseMovement {
    pub rotation: Quat, // this is mouse delta * camera sens of player
    pub client_id: u64,
}

#[derive(Event, Clone, Deserialize, Serialize, Debug)]
pub struct ClientLookDirection {
    pub dir: Vec3,
    pub client_id: u64,
}

#[derive(Deserialize, Serialize, Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub enum ClientInput {
    Forward,
    Back,
    Right,
    Left,
    Jump,
    Shoot,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum ClientButtonState {
    Pressed(ClientInput),
    Released(ClientInput),
    //Once(ClientInput),
}

pub enum ClientChannel {
    Input,
    MouseInput,
    ClientData, // client authoritative data (?)
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Input => 0,
            ClientChannel::MouseInput => 1,
            ClientChannel::ClientData => 2,
        }
    }
}
impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Input.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::ZERO,
                },
            },
            ChannelConfig {
                channel_id: Self::MouseInput.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::ClientData.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
        ]
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
    mut players_q: Query<
        (&mut Transform, &mut LinearVelocity, &mut Health, Entity),
        With<PlayerMarker>,
    >,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                translation,
                entity,
            } => {
                debug!("Spawning player entity for  client:{}", id);
                let client_entity = commands
                    .spawn((
                        Name::new("Player entity"),
                        Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
                        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                        Transform::from_xyz(0.0, 1.5, 0.0),
                        NotShadowCaster,
                        CharacterControllerBundle::new(Collider::cuboid(1.0, 2.0, 1.0))
                            .with_movement(50.0, 0.9, 7.0, (20.0 as Scalar).to_radians()),
                        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
                        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
                        GravityScale(2.0),
                        PlayerMarker,
                        MovementIntent::default(),
                        TransformInterpolation,
                        CameraSensitivity::default(),
                        InputManagerBundle::with_map(build_input_map()),
                        Player { id },
                    ))
                    .id();

                commands
                    .entity(client_entity)
                    .insert(LookDirection::default());
                commands.entity(client_entity).insert(Health(PLAYER_HEALTH));
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

                    let arm_mesh = meshes.add(Cuboid::new(0.1, 0.1, 0.5));
                    let arm_material = materials.add(Color::from(WHITE));
                    let arm = commands
                        .spawn((
                            Name::new("Player Arm"),
                            Mesh3d(arm_mesh),
                            MeshMaterial3d(arm_material),
                            Transform::from_xyz(0.3, -0.2, -0.3),
                            // Ensure the arm is only rendered by the view model camera.
                            RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                            NotShadowCaster,
                        ))
                        .id();

                    commands.entity(client_entity).add_child(view_model_cam);
                    commands.entity(client_entity).add_child(arm);
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
            ServerMessages::BulletCreate { translation, dir } => {
                let rocket_speed = Vec3::from_array(dir) * ROCKET_SPEED;
                commands.spawn((
                    Name::new("Bullet"),
                    Rocket,
                    LinearVelocity(rocket_speed),
                    RigidBody::Kinematic,
                    Collider::cuboid(0.2, 0.2, 0.2),
                    Mesh3d(meshes.add(Cuboid::from_length(0.2))),
                    MeshMaterial3d(materials.add(Color::srgb_u8(154, 109, 100))),
                    Transform::from_translation(translation.into()),
                ));
            }

            ServerMessages::PlayerDeath { server_ent, id } => {
                let client_ent = network_mapping.0.get(&server_ent);
                if let Some(client_ent) = client_ent {
                    if let Some(commands) = commands.get_entity(*client_ent) {
                        debug!("received player death, despawning");
                        commands.despawn_recursive();
                    }
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let networked_entities: NetworkedEntities = bincode::deserialize(&message).unwrap();

        for i in 0..networked_entities.entities.len() {
            if let Some(entity) = network_mapping.0.get(&networked_entities.entities[i]) {
                let translation = networked_entities.translations[i].into();
                //let rotation = Quat::from_array(networked_entities.rotations[i]);
                let velocity = LinearVelocity(networked_entities.velocities[i].into());
                /*
                debug!(
                    "Updating transform of {:?}, New Transform: {:?}",
                    entity, transform
                );
                 */

                let Ok((mut player_tf, mut player_velocity, mut player_health, _)) =
                    players_q.get_mut(*entity)
                else {
                    continue;
                };

                player_tf.translation = translation;
                //player_tf.rotation = rotation;
                *player_velocity = velocity;
                player_health.0 = networked_entities.health[i];

                //commands.entity(*entity).insert(transform);
            }
        }
    }
}

fn send_action_diffs<A: Actionlike + Serialize>(
    mut action_state_query: Query<&mut ActionState<A>>,
    mut action_diff_events: EventReader<ActionDiffEvent<A>>,
    client: Option<ResMut<RenetClient>>,
    client_id: Option<Res<CurrentClientId>>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(client_id) = client_id else {
        return;
    };
    for action_diff_event in action_diff_events.read() {
        if let Some(owner) = action_diff_event.owner {
            let mut action_state = action_state_query.get_mut(owner).unwrap();
            action_diff_event.action_diffs.iter().for_each(|diff| {
                // @performance should we send entire vec maybe?
                let input_message = bincode::serialize(&ClientAction {
                    action_diff: diff.clone(),
                    client_id: client_id.0.into(),
                })
                .unwrap();
                client.send_message(ClientChannel::Input, input_message);
            });
        }
    }
}

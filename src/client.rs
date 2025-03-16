use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use crate::{
    camera::PlayerMarker,
    character::{build_player_ent, Health, NetworkScenario},
    consts::ROCKET_SPEED,
    input::Action,
    server::{connection_config, NetworkedEntities},
    water::{GameState, Rocket},
    AppState,
};
use avian3d::prelude::{Collider, LinearVelocity, RigidBody};
use bevy::{prelude::*, utils::HashMap};
use bevy_egui::EguiContexts;
use bevy_renet::{
    netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport},
    renet::{ChannelConfig, ClientId, RenetClient, SendType},
    client_connected,
    RenetClientPlugin,
};
use serde::{Deserialize, Serialize};

use crate::server::{ServerChannel, ServerMessages};

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

        #[cfg(feature = "netcode")]
        app.add_systems(OnEnter(GameState::Game), setup_client_netcode);

        #[cfg(feature = "steam")]
        app.add_systems(OnEnter(GameState::Game), setup_client_steam);

        app.add_plugins(InputManagerPlugin::<Action>::default());

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

#[cfg(feature = "netcode")]
fn setup_client_netcode(mut commands: Commands) {

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
    commands.insert_resource(transport);
    commands.insert_resource(CurrentClientId(client_id));
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Connected;


#[cfg(feature = "steam")]
fn setup_client_steam(app: &mut App) {
    use bevy_renet::steam::{SteamClientPlugin, SteamClientTransport, SteamTransportError};
    use steamworks::{SingleClient, SteamId};
    use bevy_renet::client_connected;

    let (steam_client, single) = steamworks::Client::init_app(480).unwrap();

    steam_client.networking_utils().init_relay_network_access();

    let args: Vec<String> = std::env::args().collect();
    let server_steam_id: u64 = args[1].parse().unwrap();
    let server_steam_id = SteamId::from_raw(server_steam_id);

    let client = RenetClient::new(connection_config());
    let transport = SteamClientTransport::new(&steam_client, &server_steam_id).unwrap();

    app.add_plugins(SteamClientPlugin);
    app.insert_resource(client);
    app.insert_resource(transport);
    app.insert_resource(CurrentClientId(steam_client.user().steam_id().raw()));

    app.configure_sets(Update, Connected.run_if(client_connected));


    app.insert_non_send_resource(single);
    fn steam_callbacks(client: NonSend<SingleClient>) {
        client.run_callbacks();
    }

    app.add_systems(PreUpdate, steam_callbacks);

    // If any error is found we just panic
    #[allow(clippy::never_loop)]
    fn panic_on_error_system(mut renet_error: EventReader<SteamTransportError>) {
        for e in renet_error.read() {
            panic!("{}", e);
        }
    }

    app.add_systems(Update, panic_on_error_system);
}


fn send_message_system(client: ResMut<RenetClient>) {
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
    asset_server: Res<AssetServer>,
    client_id: Option<Res<CurrentClientId>>,
    mut players_q: Query<
        (&mut Transform, &mut LinearVelocity, &mut Health, Entity),
        With<PlayerMarker>,
    >,
) {
    let Some(client_id) = client_id else {
        return;
    };
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                translation,
                entity,
            } => {
                debug!("Spawning player entity for  client:{}", id);

                let client_entity = if client_id.0 == id {
                    build_player_ent(
                        &mut commands,
                        &asset_server,
                        id,
                        NetworkScenario::MyClient,
                        &mut meshes,
                        &mut materials,
                    )
                } else {
                    build_player_ent(
                        &mut commands,
                        &asset_server,
                        id,
                        NetworkScenario::OtherClient,
                        &mut meshes,
                        &mut materials,
                    )
                };

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
                *player_velocity = velocity;
                player_health.0 = networked_entities.health[i];

                if lobby
                    .players
                    .get(&client_id.0)
                    .is_some_and(|inner| inner.client_entity != *entity)
                {
                    player_tf.rotation = Quat::from_array(networked_entities.rotations[i]);
                }
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
            let action_state = action_state_query.get_mut(owner).unwrap();
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

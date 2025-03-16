use bevy::prelude::*;

use super::server::*;
use crate::camera::*;
use crate::character::*;
use crate::consts::*;
use bevy_renet::renet::RenetServer;
use std::time::Duration;

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

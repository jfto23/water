use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use serde::{Deserialize, Serialize};

use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::consts::FIXED_TIME_INTERVAL;
use crate::shared::tick::Tick;

use super::ClientChannel;

#[derive(Deserialize, Serialize, Debug)]
pub struct Ping {
    pub timestamp: u128,
}

#[derive(Resource, Default, Debug)]
pub struct RoundTripTimes {
    pub history: Vec<u128>,
}

// offset tick that the client should be from the server. Client tick should always be ahead of server
#[derive(Resource, Default, Debug)]
pub struct TickOffset {
    pub offset: u16,
}

#[derive(Resource, Default, Debug)]
pub struct LastServerTick {
    pub tick: u64,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum SyncState {
    #[default]
    Sync,
    Done,
}

pub fn ping_server(client_tick: ResMut<Tick>, mut client: Option<ResMut<RenetClient>>) {
    let Some(mut client) = client else {
        return;
    };
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let message = bincode::serialize(&Ping {
        timestamp: since_the_epoch.as_millis(),
    })
    .unwrap();
    client.send_message(ClientChannel::Ping, message);
}

pub fn check_handshake_progress(
    round_trips: Res<RoundTripTimes>,
    mut next_state: ResMut<NextState<SyncState>>,
) {
    if round_trips.history.len() > 20 {
        next_state.set(SyncState::Done);
    }
}

pub fn finalize_sync(
    round_trips: Res<RoundTripTimes>,
    mut next_state: ResMut<NextState<SyncState>>,
    mut tick_offset: ResMut<TickOffset>,
    mut server_tick: Res<LastServerTick>,
    mut client_tick: ResMut<Tick>,
) {
    let rtt = round_trips.history.iter().sum::<u128>() / round_trips.history.len() as u128;
    debug!("synchronization done! average rtt: {:?}", rtt);

    tick_offset.offset = ((rtt / 2) as f32 / (1000.0 / FIXED_TIME_INTERVAL)).round() as u16;
    // client needs to be ahead of server so inputs arrive at the correct tick
    debug!("tick offset of client: {:?}", tick_offset);
    client_tick.current_tick = server_tick.tick + tick_offset.offset as u64;
}

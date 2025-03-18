use std::collections::VecDeque;

use avian3d::prelude::LinearVelocity;
use bevy::{
    color::palettes::css::{GREEN, RED},
    prelude::*,
};

use crate::{consts::CLIENT_SNAPSHOT_HISTORY_LENGTH, shared::tick::Tick};

use super::ControlledPlayer;

#[derive(Resource, Default, Debug)]
// todo: this should be claled PredictionHistory
pub struct SnapshotHistory {
    pub history: VecDeque<Snapshot>,
}

//https://gamedev.stackexchange.com/questions/59866/client-side-prediction-on-fps-game
#[derive(Debug)]
pub struct Snapshot {
    pub translation: Vec3,
    pub velocity: Vec3,
    pub tick: u64,
}

pub fn save_snapshot(
    player_q: Query<(&Transform, &LinearVelocity), With<ControlledPlayer>>,
    client_tick: Res<Tick>,
    mut snapshot_history: ResMut<SnapshotHistory>,
) {
    let Ok((player_tf, player_vel)) = player_q.get_single() else {
        return;
    };

    if snapshot_history.history.len() == CLIENT_SNAPSHOT_HISTORY_LENGTH {
        snapshot_history.history.pop_front();
    }
    snapshot_history.history.push_back(Snapshot {
        translation: player_tf.translation,
        velocity: **player_vel,
        tick: client_tick.current_tick,
    });
}

pub fn reconcile(
    translation: &Vec3,
    snapshot_history: &Res<SnapshotHistory>,
    confirmed_tick: u64,
) -> (bool, Vec3) {
    /*

    debug!(
        "snapshot_history: {:?}, confirmed_tick: {:?}",
        snapshot_history, confirmed_tick
    );
     */

    //debug!("last confirmed_tick by server: {:?}", confirmed_tick);

    for snapshot in snapshot_history.history.iter() {
        if snapshot.tick == confirmed_tick {
            if translation.distance(snapshot.translation) > 0.1 {
                debug!("reconciliation triggered ");
                return (true, snapshot.translation);
            } else {
                debug!("prediction is correct. Continuing");
                return (false, Vec3::ZERO);
            }
        }
    }
    return (false, Vec3::ZERO);
}

#[derive(Default)]
pub struct Reconciliation(VecDeque<[Vec3; 2]>);

#[derive(Event, Clone)]
pub struct ReconciliationEvent {
    pub old: Vec3,
    pub new: Vec3,
}

pub fn debug_reconciliation_event(
    mut rec: Local<Reconciliation>,
    mut gizmos: Gizmos,
    mut reconciliation_event: EventReader<ReconciliationEvent>,
) {
    for ev in reconciliation_event.read() {
        debug!("reconciliation: {:?}, {:?}", ev.old, ev.new);
        if rec.0.len() == 32 {
            rec.0.pop_front();
        }
        rec.0.push_back([ev.old, ev.new]);
    }

    for r in rec.0.iter() {
        gizmos.cuboid(Transform::from_translation(r[0]), RED);
        gizmos.cuboid(Transform::from_translation(r[1]), GREEN);
    }
}

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Tick {
    pub current_tick: u64,
}

impl Tick {
    pub fn increment(&mut self) {
        self.current_tick = self.current_tick.overflowing_add(1).0;
    }
}

pub fn update_tick(mut tick: ResMut<Tick>) {
    tick.increment();
}

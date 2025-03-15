pub struct ConsolePlugin;

use std::{
    collections::{HashMap, VecDeque},
    mem,
};

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_egui::{egui, EguiContexts};
impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (ui_example_system, handle_console_commands))
            .init_resource::<ConsoleInput>()
            .init_resource::<ConsoleHistory>()
            .init_resource::<ConsoleCommands>()
            .init_resource::<GameSettings>()
            .add_event::<UserInput>();
    }
}

#[derive(Resource, Default)]
struct ConsoleInput(String);

#[derive(Resource, Default)]
struct ConsoleHistory(VecDeque<String>);

#[derive(Event, Clone)]
struct UserInput(String);

#[derive(Resource)]
struct ConsoleCommands(HashMap<String, SystemId<In<Vec<String>>>>);

#[derive(Resource)]
pub struct GameSettings {
    pub show_debug_rocket: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            show_debug_rocket: false,
        }
    }
}

impl FromWorld for ConsoleCommands {
    #[rustfmt::skip]
    fn from_world(world: &mut World) -> Self {
        let mut console_commands = ConsoleCommands(HashMap::new());

        console_commands.0.insert("clear".into(), world.register_system(console_clear));
        console_commands.0.insert("show_rocket_debug".into(), world.register_system(console_show_rocket_debug));


        //register_command!("clear",console_clear)
        console_commands
    }
}

fn console_clear(In(_input): In<Vec<String>>, mut history: ResMut<ConsoleHistory>) {
    history.0.clear();
}

fn console_show_rocket_debug(In(_input): In<Vec<String>>, mut settings: ResMut<GameSettings>) {
    settings.show_debug_rocket = !settings.show_debug_rocket;
}

fn ui_example_system(
    mut contexts: EguiContexts,
    mut console_input: ResMut<ConsoleInput>,
    mut console_history: ResMut<ConsoleHistory>,
    mut input_event: EventWriter<UserInput>,
) {
    egui::Window::new("Console")
        .default_open(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut console_input.0));
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let old_command = mem::replace(&mut console_input.0, String::new());
                    console_history.0.push_back(old_command.clone());
                    input_event.send(UserInput(old_command));
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(prev_command) = console_history.0.back() {
                        console_input.0 = prev_command.clone();
                    }
                }
                console_history.0.iter().rev().for_each(|cmd| {
                    ui.label(cmd);
                });
            });
        });
}

fn handle_console_commands(
    mut user_input: EventReader<UserInput>,
    console_commands: Res<ConsoleCommands>,
    mut commands: Commands,
) {
    for ev in user_input.read() {
        let input_vec: Vec<_> = ev.0.split(" ").map(|s| s.to_string()).collect();
        if input_vec.len() == 0 {
            continue;
        }
        if let Some(system_id) = console_commands.0.get(&input_vec[0]) {
            commands.run_system_with_input(*system_id, input_vec);
        }
    }
}

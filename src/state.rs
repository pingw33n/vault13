use sdl2::event::Event;
use std::time::Duration;

use crate::ui::Ui;
use crate::ui::command::UiCommand;

pub trait AppState {
    fn handle_event(&mut self, event: &Event, ui: &mut Ui) -> bool;
    fn handle_ui_command(&mut self, command: UiCommand, ui: &mut Ui);
    fn update(&mut self, delta: Duration, ui: &mut Ui);
}
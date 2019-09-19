use sdl2::event::Event;
use std::time::Duration;

use crate::ui::Ui;
use crate::ui::out::OutEvent;

pub trait AppState {
    fn handle_event(&mut self, event: &Event, ui: &mut Ui) -> bool;
    fn handle_ui_out_event(&mut self, event: OutEvent, ui: &mut Ui);
    fn update(&mut self, delta: Duration);
}
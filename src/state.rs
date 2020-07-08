use sdl2::event::{Event as SdlEvent};
use std::time::{Duration, Instant};

use crate::event::{Event, Sink};
use crate::ui::Ui;

pub struct HandleEvent<'a> {
    pub event: Event,
    pub sink: &'a mut Sink<'a>,
    pub ui: &'a mut Ui,
}

pub struct Update<'a> {
    pub time: Instant,
    pub delta: Duration,
    pub sink: &'a mut Sink<'a>,
    pub ui: &'a mut Ui,
}

pub trait AppState {
    fn handle_input(&mut self, event: &SdlEvent, ui: &mut Ui) -> bool;
    fn handle_event(&mut self, ctx: HandleEvent);
    fn update(&mut self, ctx: Update);
}
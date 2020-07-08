use crate::event::Event;

use super::*;

pub struct SendEvent {
    event: Option<Event>,
}

impl SendEvent {
    pub fn new(event: Event) -> Self {
        Self {
            event: Some(event),
        }
    }
}

impl Sequence for SendEvent {
    fn update(&mut self, ctx: &mut Update) -> Result {
        if let Some(event) = self.event.take() {
            ctx.sink.send(event);
        }
        Result::Done
    }
}
use crate::game::object;
use crate::graphics::EPoint;

use super::*;

#[derive(Clone, Debug)]
pub enum Event {
    ObjectMoved {
        obj: object::Handle,
        old_pos: EPoint,
        new_pos: EPoint,
    },
    Talk {
        talker: object::Handle,
        talked: object::Handle,
    },
}

pub struct PushEvent {
    event: Option<Event>,
}

impl PushEvent {
    pub fn new(event: Event) -> Self {
        Self {
            event: Some(event),
        }
    }
}

impl Sequence for PushEvent {
    fn update(&mut self, ctx: &mut Update) -> Result {
        if let Some(event) = self.event.take() {
            ctx.out.push(event);
        }
        Result::Done
    }
}
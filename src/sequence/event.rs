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
    SetDoorState {
        door: object::Handle,
        open: bool,
    },
    Talk {
        talker: object::Handle,
        talked: object::Handle,
    },
    Use {
        user: object::Handle,
        used: object::Handle,
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
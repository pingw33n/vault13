use super::*;

use crate::game::object;
use crate::graphics::EPoint;

/// Output event for signaling widget-specific events to callee.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutEvent {
    pub source: Handle,
    pub data: OutEventData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutEventData {
    ObjectPick {
        kind: ObjectPickKind,
        obj: object::Handle,
    },
    HexPick {
        action: bool,
        pos: EPoint,
    },
    Action {
        action: action_menu::Action,
    },

    #[doc(hidden)]
    __NonExhaustive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectPickKind {
    Hover,
    DefaultAction,
    ActionMenu,
}
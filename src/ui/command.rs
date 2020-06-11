use super::*;

use crate::game::object;
use crate::graphics::EPoint;

/// Command for signaling widget-specific events to callee.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCommand {
    pub source: Handle,
    pub data: UiCommandData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiCommandData {
    ObjectPick {
        kind: ObjectPickKind,
        obj: object::Handle,
    },
    HexPick {
        action: bool,
        pos: EPoint,
    },
    Action {
        action: crate::game::ui::action_menu::Action,
    },
    Pick {
        id: u32,
    },
    Scroll,
    Skilldex(SkilldexCommand),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectPickKind {
    Hover,
    DefaultAction,
    ActionMenu,
    Skill(crate::asset::Skill),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkilldexCommand {
    Cancel,
    Show,
    Skill {
        skill: crate::asset::Skill,
        target: Option<object::Handle>,
    },
}
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
    Inventory(inventory::Command),
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

pub mod inventory {
    use super::*;
    use crate::game::ui::action_menu::Action;
    use crate::game::ui::inventory_list::Scroll;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Command {
        Hide,
        Show,
        Hover {
            object: object::Handle,
        },
        ActionMenu {
            object: object::Handle,
        },
        Action {
            object: object::Handle,
            action: Option<Action>,
        },
        Scroll(Scroll),
        ListDrop {
            pos: Point,
            object: object::Handle,
        },
    }
}


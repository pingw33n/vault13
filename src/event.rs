mod events;

use crate::asset::proto::TargetMap;
use crate::game::object;
use crate::graphics::{EPoint, Point};
use crate::graphics::geometry::hex::Direction;
use crate::ui;

pub use events::{Events, Sink};
pub use inventory::InventoryEvent;
pub use move_window::MoveWindowEvent;

#[derive(Clone, Copy)]
pub enum Event {
    MapExit {
        map: TargetMap,
        pos: EPoint,
        direction: Direction,
    },
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
    Scroll {
        source: ui::Handle,
    },
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
    UseSkill {
        skill: crate::asset::Skill,
        user: object::Handle,
        target: object::Handle,
    },
    Skilldex(SkilldexEvent),
    InventoryList(InventoryListEvent),
    Inventory(InventoryEvent),
    MoveWindow(MoveWindowEvent),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryListEvent {
    ActionMenu {
        object: object::Handle,
    },
    DefaultAction {
        object: object::Handle,
    },
    Drop {
        source: ui::Handle,
        pos: Point,
        object: object::Handle,
    },
    Hover {
        object: object::Handle,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectPickKind {
    Hover,
    DefaultAction,
    ActionMenu,
    Skill(crate::asset::Skill),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkilldexEvent {
    Cancel,
    Show,
    Skill {
        skill: crate::asset::Skill,
        target: Option<object::Handle>,
    },
}

mod inventory {
    use super::*;
    use crate::game::ui::action_menu::Action;
    use crate::game::ui::inventory_list::Scroll;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum InventoryEvent {
        Hide,
        Show,
        Hover {
            object: object::Handle,
        },
        Action {
            object: object::Handle,
            action: Action,
        },
        Scroll(Scroll),
        ToggleMouseMode,
    }
}

mod move_window {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum MoveWindowEvent {
        Hide {
            ok: bool
        },
        Inc,
        Dec,
        Max,
    }
}
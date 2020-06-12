use num_traits::clamp;
use std::cmp;

use crate::asset::frame::FrameId;
use crate::graphics::{Point, Rect};
use crate::graphics::geometry::hex::Direction;
use crate::graphics::sprite::Sprite;
use crate::ui::*;
use crate::ui::command::UiCommandData;

pub fn show(actions: Vec<Action>, win: Handle, ui: &mut Ui) -> Handle {
    assert!(!actions.is_empty());

    let (placement, saved_cursor) = {
        let win_base = ui.widget_base(win);
        let mut win_base = win_base.borrow_mut();
        let saved_cursor = win_base.cursor();
        let placement = Placement::new(actions.len() as u32, ui.cursor_pos(), win_base.rect());
        win_base.set_cursor(Some(placement.cursor));
        (placement, saved_cursor)
    };

    ui.show_cursor_ghost();
    ui.set_cursor_constraint(placement.rect.clone());
    ui.set_cursor_pos(placement.rect.top_left());

    let action_menu = ui.new_widget(win, placement.rect, Some(Cursor::Hidden), None,
        ActionMenu::new(actions, saved_cursor));
    ui.capture(action_menu);

    action_menu
}

pub fn hide(action_menu: Handle, ui: &mut Ui) {
    ui.hide_cursor_ghost();
    ui.clear_cursor_constraint();
    ui.remove(action_menu);
}

pub struct Placement {
    pub rect: Rect,
    pub cursor: Cursor,
}

impl Placement {
    pub fn new(action_count: u32, cursor_pos: Point, bounds: Rect) -> Self {
        let flipped = bounds.right - cursor_pos.x <
            Action::ICON_OFFSET_X + Action::ICON_WIDTH;

        let height = Action::ICON_HEIGHT * action_count as i32;
        let offset_x = if flipped {
            - (Action::ICON_OFFSET_X + Action::ICON_WIDTH - 1)
        } else {
            Action::ICON_OFFSET_X
        };
        let offset_y = cmp::min(bounds.bottom - cursor_pos.y - height, 0);
        let pos = cursor_pos + Point::new(offset_x, offset_y);
        let rect = Rect::with_size(pos.x, pos.y, Action::ICON_WIDTH, height);

        let cursor = if flipped {
            Cursor::ActionArrowFlipped
        } else {
            Cursor::ActionArrow
        };

        Self {
            rect,
            cursor,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Cancel,
    Drop,
    Inventory,
    Look,
    Push,
    Rotate,
    Talk,
    Unload,
    UseHand,
    UseSkill,
}

impl Action {
    pub const ICON_OFFSET_X: i32 = 29;
    pub const ICON_WIDTH: i32 = 40;
    pub const ICON_HEIGHT: i32 = 40;

    pub fn icons(self) -> (FrameId, FrameId) {
        use Action::*;
        use self::FrameId as F;
        match self {
            Cancel => (F::CANCELN, F::CANCELH),
            Drop => (F::DROPN, F::DROPH),
            Inventory => (F::INVENN, F::INVENH),
            Look => (F::LOOKN, F::LOOKH),
            Push => (F::PUSHN, F::PUSHH),
            Rotate => (F::ROTATEN, F::ROTATEH),
            Talk => (F::TALKN, F::TALKH),
            Unload => (F::UNLOADN, F::UNLOADH),
            UseHand => (F::USEGETN, F::USEGETH),
            UseSkill => (F::SKILLN, F::SKILLH),
        }
    }
}

pub struct ActionMenu {
    actions: Vec<Action>,
    selection: u32,
    saved_cursor: Option<Cursor>,
}

impl ActionMenu {
    fn new(actions: Vec<Action>, saved_cursor: Option<Cursor>) -> Self {
        Self {
            actions,
            selection: 0,
            saved_cursor,
        }
    }

    fn update_selection(&mut self, base: &Base, mouse_pos: Point) {
        let rel_y = mouse_pos.y - base.rect().top;
        // Apply speed up to mouse movement.
        let rel_y = (rel_y as f64 * 1.5) as i32;
        self.selection = clamp(rel_y / Action::ICON_HEIGHT, 0, self.actions.len() as i32 - 1) as u32;
    }
}

impl Widget for ActionMenu {
    fn handle_event(&mut self, mut ctx: HandleEvent) {

        match ctx.event {
            Event::MouseMove { pos } => {
                self.update_selection(ctx.base, pos);
            },
            Event::MouseUp { pos, .. } => {
                self.update_selection(ctx.base, pos);
                ctx.out(UiCommandData::Action { action: self.actions[self.selection as usize] });
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let mut pos = ctx.base.unwrap().rect().top_left();
        for (i, &icon) in self.actions.iter().enumerate() {
            let (normal, highl) = icon.icons();
            let fid = if i as u32 == self.selection {
                highl
            } else {
                normal
            };
            Sprite {
                pos,
                centered: false,
                fid,
                frame_idx: 0,
                direction: Direction::NE,
                light: 0x10000,
                effect: None,
            }.render(ctx.canvas, ctx.frm_db);

            pos.y += Action::ICON_HEIGHT;
        }
    }
}
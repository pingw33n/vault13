use bstring::bfmt::ToBString;
use bstring::BString;
use std::cmp;
use std::convert::TryFrom;
use std::time::{Duration, Instant};

use crate::asset::frame::FrameId;
use crate::graphics::{Rect, Point};
use crate::graphics::color::WHITE;
use crate::graphics::font::FontKey;
use crate::graphics::sprite::Sprite;
use crate::game::object;
use crate::game::ui::action_menu::{Action, Placement};
use crate::ui::*;
use crate::ui::command::UiCommandData;
use crate::ui::command::inventory::Command;

pub struct Item {
    pub object: object::Handle,
    pub fid: FrameId,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseMode {
    Action,
    Drag,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scroll {
    Down,
    Up,
}

pub struct InventoryList {
    item_height: i32,
    item_spacing: i32,
    items: Vec<Item>,
    scroll_idx: usize,
    dragging: Option<usize>,
    mouse_mode: MouseMode,
    last_hovered: Option<object::Handle>,
    default_action: Option<Action>,
    action_menu_state: Option<(Instant, usize)>,
    visible_items: usize,
}

impl InventoryList {
    pub fn new(item_height: i32, item_spacing: i32) -> Self {
        Self {
            item_height,
            item_spacing,
            items: Vec::new(),
            scroll_idx: 0,
            dragging: None,
            mouse_mode: MouseMode::Drag,
            last_hovered: None,
            default_action: None,
            action_menu_state: None,
            visible_items: 0,
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.scroll_idx = 0;
    }

    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn can_scroll(&self, scroll: Scroll) -> bool {
        match scroll {
            Scroll::Down => self.scroll_idx + 1 <= self.max_scroll_idx(),
            Scroll::Up => self.scroll_idx > 0,
        }
    }

    pub fn scroll_idx(&self) -> usize {
        self.scroll_idx
    }

    pub fn scroll(&mut self, scroll: Scroll) {
        self.set_scroll_idx(match scroll {
            Scroll::Down => self.scroll_idx + 1,
            Scroll::Up => self.scroll_idx.checked_sub(1).unwrap_or(0),
        });
    }

    pub fn set_scroll_idx(&mut self, scroll_idx: usize) {
        self.scroll_idx = cmp::min(scroll_idx, self.max_scroll_idx());
    }

    pub fn set_mouse_mode(&mut self, mouse_mode: MouseMode) {
        self.mouse_mode = mouse_mode;
        self.default_action = None;
        self.action_menu_state = None;
        self.last_hovered = None;
        self.dragging = None;
    }

    fn max_scroll_idx(&self) -> usize {
        self.items.len().checked_sub(self.visible_items).unwrap_or(0)
    }

    fn item_index_at(&self, rect: Rect, pos: Point) -> Option<usize> {
        if !rect.contains(pos) {
            return None;
        }
        let i = (pos.y - rect.top) / (self.item_height + self.item_spacing);
        let i = self.scroll_idx + usize::try_from(i).unwrap();
        if i < self.items.len() {
            Some(i)
        } else {
            None
        }
    }
}

impl Widget for InventoryList {
    fn init(&mut self, ctx: Init) {
        self.visible_items = (ctx.base.rect().height() / (self.item_height + self.item_spacing)) as usize;
    }

    fn handle_event(&mut self, mut ctx: HandleEvent) {
        match ctx.event {
            Event::MouseDown { pos: _, button} if button == MouseButton::Left => {
                if let Some(idx) = self.item_index_at(ctx.base.rect(), ctx.cursor_pos) {
                    match self.mouse_mode {
                        MouseMode::Action => {
                            self.action_menu_state = Some((ctx.now, idx));
                        }
                        MouseMode::Drag => {
                            ctx.base.set_cursor(Some(Cursor::Frame(self.items[idx].fid)));
                            ctx.capture();
                            self.dragging = Some(idx);
                        }
                    }
                }
            }
            Event::MouseUp { pos: _, button } if button == MouseButton::Left => {
                match self.mouse_mode {
                    MouseMode::Action => {
                        self.action_menu_state = None;
                        if let Some(idx) = self.item_index_at(ctx.base.rect(), ctx.cursor_pos) {
                            ctx.out(UiCommandData::Inventory(Command::Action {
                                action: None,
                                object: self.items[idx].object,
                            }));
                        }
                    }
                    MouseMode::Drag => if let Some(item_index) = self.dragging.take() {
                        ctx.base.set_cursor(None);
                        ctx.release();
                        let object = self.items[item_index].object;
                        ctx.out(UiCommandData::Inventory(Command::ListDrop {
                            pos: ctx.cursor_pos,
                            object,
                        }));
                    }
                }
            }
            Event::MouseMove { pos: _ } if self.mouse_mode == MouseMode::Action => {
                if let Some(idx) = self.item_index_at(ctx.base.rect(), ctx.cursor_pos) {
                    self.default_action = Some(Action::Look);
                    let object = self.items[idx].object;
                    if Some(object) != self.last_hovered {
                        self.last_hovered = Some(object);
                        ctx.out(UiCommandData::Inventory(Command::Hover {
                            object,
                        }));
                    }
                } else {
                    self.default_action = None;
                }
            }
            Event::MouseLeave => {
                self.default_action = None;
            }
            Event::Tick => {
                if let Some((start, item)) = self.action_menu_state {
                    if ctx.now - start >= Duration::from_millis(500) {
                        self.default_action = None;
                        self.action_menu_state = None;

                        ctx.out(UiCommandData::Inventory(Command::ActionMenu {
                            object: self.items[item].object,
                        }));
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let rect = ctx.base.unwrap().rect();

        let mut item_rect = rect.with_height(self.item_height);

        for item in self.items.iter().skip(self.scroll_idx as usize) {
            if !rect.contains_rect(item_rect) {
                break;
            }
            let frame = ctx.frm_db.get(item.fid).unwrap();
            let frame = frame.first();

            let draw_rect = fit(frame.bounds(), item_rect);
            ctx.canvas.draw_scaled(&frame.texture, draw_rect);

            if item.count > 1 {
                let s = BString::concat(&[&b"x"[..], item.count.to_bstring().as_bytes()]);
                ctx.canvas.draw_text(&s, item_rect.top_left(),
                    FontKey::antialiased(1), WHITE, &Default::default())
            }

            item_rect = item_rect.translate(Point::new(0, self.item_height + self.item_spacing));
        }

        if let Some(default_action) = self.default_action {
            let fid = default_action.icons().0;
            let mut sprite = Sprite::new(fid);
            sprite.pos = Placement::new(1, ctx.cursor_pos, Rect::full()).rect.top_left();
            sprite.render(ctx.canvas, ctx.frm_db);
        }
    }
}

fn fit(r1: Rect, r2: Rect) -> Rect {
    assert!(!r1.is_empty() && !r2.is_empty());
    let r = if r1.width() > r2.width() || r1.height() > r2.height() {
        let this_ar = r1.width() as f64 / r1.height() as f64;
        let r2_ar = r2.width() as f64 / r2.height() as f64;
        if this_ar >= r2_ar {
            r2.with_height((r2.width() as f64 / this_ar) as i32)
        } else {
            r2.with_width((r2.height() as f64 * this_ar) as i32)
        }
    } else {
        r2.with_width(r1.width())
            .with_height(r1.height())
    };
    r.translate(Point::new(
        (r2.width() - r.width()) / 2,
        (r2.height() - r.height()) / 2))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fit_() {
        fn r(l: i32, t: i32, w: i32, h: i32) -> Rect {
            Rect::with_size(l, t, w, h)
        }
        assert_eq!(fit(r(0, 0, 159, 39), r(100, 200, 56, 40)), r(100, 213, 56, 13));
        assert_eq!(fit(r(0, 0, 40, 53), r(100, 200, 56, 40)), r(113, 200, 30, 40));
        assert_eq!(fit(r(0, 0, 40, 20), r(100, 200, 56, 40)), r(108, 210, 40, 20));
    }
}
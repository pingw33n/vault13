use bstring::bfmt::ToBString;
use bstring::BString;
use std::cmp;
use std::convert::TryFrom;
use std::time::{Duration, Instant};

use crate::asset::frame::FrameId;
use crate::graphics::{Rect, Point};
use crate::graphics::color::WHITE;
use crate::graphics::font::FontKey;
use crate::graphics::sprite::{Sprite, Effect};
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
            Scroll::Down => self.scroll_idx < self.max_scroll_idx(),
            Scroll::Up => self.scroll_idx > 0,
        }
    }

    pub fn scroll_idx(&self) -> usize {
        self.scroll_idx
    }

    pub fn scroll(&mut self, scroll: Scroll) {
        self.set_scroll_idx(match scroll {
            Scroll::Down => self.scroll_idx + 1,
            Scroll::Up => self.scroll_idx.saturating_sub(1),
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
        self.items.len().saturating_sub(self.visible_items)
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
            UiEvent::MouseDown { pos: _, button} if button == MouseButton::Left => {
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
            UiEvent::MouseUp { pos: _, button } if button == MouseButton::Left => {
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
            UiEvent::MouseMove { pos: _ } if self.mouse_mode == MouseMode::Action => {
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
            UiEvent::MouseLeave => {
                self.default_action = None;
            }
            UiEvent::Tick => {
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
            let mut sprite = Sprite::new(item.fid);
            sprite.pos = item_rect.top_left();
            sprite.effect = Some(Effect::Fit {
                width: item_rect.width(),
                height: item_rect.height(),
            });
            sprite.render(ctx.canvas, ctx.frm_db);

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

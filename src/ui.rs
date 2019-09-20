pub mod button;
pub mod message_panel;
pub mod out;
pub mod panel;

pub use sdl2::mouse::MouseButton;

use downcast_rs::{Downcast, impl_downcast};
use enum_map_derive::Enum;
use sdl2::event::{Event as SdlEvent};
use slotmap::{SecondaryMap, SlotMap};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::time::Instant;

use crate::asset::frame::{FrameId, FrameDb};
use crate::graphics::{Point, Rect};
use crate::graphics::geometry::hex::Direction;
use crate::graphics::render::Canvas;
use crate::graphics::sprite::Sprite;
use crate::util::{SmKey, VecExt};
use crate::ui::out::OutEvent;
use crate::graphics::font::Fonts;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    MouseDown {
        pos: Point,
        button: MouseButton,
    },
    MouseLeave,
    MouseMove {
        pos: Point,
    },
    MouseUp {
        pos: Point,
        button: MouseButton,
    },
    Tick,
    #[doc(hidden)]
    __NonExhaustive,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd)]
pub enum Cursor {
    ActionArrow,
    ActionArrowFlipped,
    Arrow,
    ArrowDown,
    ArrowUp,
    Hidden,
}

impl Cursor {
    pub fn fid(self) -> FrameId {
        use Cursor::*;
        match self {
            ActionArrow => FrameId::ACTARROW,
            ActionArrowFlipped => FrameId::ACTARROM,
            Arrow => FrameId::STDARROW,
            ArrowDown => FrameId::SDNARROW,
            ArrowUp => FrameId::SUPARROW,
            Hidden => FrameId::BLANK,
        }
    }

    fn offset(self) -> Point {
        use Cursor::*;
        match self {
            // ACTARROM is not properly centered.
            ActionArrowFlipped => (-14, 22),
            _ => (0, 0)
        }.into()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Handle(SmKey);

pub struct HandleInput<'a> {
    pub now: Instant,
    pub event: &'a SdlEvent,
    pub out: &'a mut Vec<out::OutEvent>,
}

pub struct Ui {
    frm_db: Rc<FrameDb>,
    fonts: Rc<Fonts>,
    widget_handles: SlotMap<SmKey, ()>,
    widget_bases: SecondaryMap<SmKey, RefCell<Base>>,
    widgets: SecondaryMap<SmKey, RefCell<Box<Widget>>>,
    windows_order: Vec<Handle>,
    cursor_pos: Point,
    cursor_constraints: Vec<Rect>,
    cursor_ghost: Option<(Point, Cursor)>,
    cursor: Cursor,
    capture: Option<Handle>,
    /// If `true` the next `update()` will fire `MouseMove` to the `cursor_pos` unless
    /// this event was seen on previous `handle_input()`.
    simulate_mouse_move: bool,
    mouse_focus: Option<Handle>,
}

impl Ui {
    pub fn new(frm_db: Rc<FrameDb>, fonts: Rc<Fonts>, width: i32, height: i32) -> Self {
        Self {
            frm_db,
            fonts,
            widget_handles: SlotMap::with_key(),
            widget_bases: SecondaryMap::new(),
            widgets: SecondaryMap::new(),
            windows_order: Vec::new(),
            cursor_pos: Point::new(0, 0),
            cursor_constraints: vec![Rect::with_size(0, 0, width, height)],
            cursor_ghost: None,
            cursor: Cursor::Arrow,
            capture: None,
            simulate_mouse_move: false,
            mouse_focus: None,
        }
    }

    pub fn fonts(&self) -> &Rc<Fonts> {
        &self.fonts
    }

    pub fn new_window(&mut self, rect: Rect, background: Option<Sprite>) -> Handle {
        let h = self.insert_widget(None, Base {
            rect,
            cursor: None,
            background,
        }, Box::new(Window {
            widgets: Vec::new(),
        }));
        self.windows_order.push(h);

        self.simulate_mouse_move = true;

        h
    }

    pub fn remove(&mut self, handle: Handle) -> bool {
        let widg = if let Some(v) = self.widgets.remove(handle.0) {
            v
        } else {
            return false;
        };
        if self.capture == Some(handle) {
            self.capture = None;
        }
        if self.mouse_focus == Some(handle) {
            self.mouse_focus = None;
        }
        self.widget_bases.remove(handle.0);

        let widg = widg.borrow();
        if let Some(win) = widg.downcast_ref::<Window>() {
            self.windows_order.remove_first(&handle).unwrap();
            for &w in &win.widgets {
                self.remove(w);
            }
        } else if let Some(win) = self.window_of(handle) {
            // Remove widget from its window.
            let mut win = self.widgets[win.0].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            win.widgets.remove_first(&handle).unwrap();
        }

        self.simulate_mouse_move = true;

        true
    }

    pub fn new_widget(&mut self,
            window: Handle,
            rect: Rect,
            cursor: Option<Cursor>,
            background: Option<Sprite>,
            widget: impl 'static + Widget)
        -> Handle
    {
        let rect = {
            let win_base = self.widget_bases[window.0].borrow();
            let top_left = win_base.rect.top_left();
            rect.translate(top_left.x, top_left.y)
        };

        let h = self.insert_widget(Some(window), Base {
            rect,
            cursor,
            background,
        }, Box::new(widget));

        self.simulate_mouse_move = true;

        h
    }

    pub fn widget_base(&self, handle: Handle) -> &RefCell<Base> {
        &self.widget_bases[handle.0]
    }

    pub fn widget(&self, handle: Handle) -> &RefCell<Box<Widget>> {
        &self.widgets[handle.0]
    }

    pub fn widget_ref<T: Widget>(&self, handle: Handle) -> Ref<T> {
        Ref::map(self.widget(handle).borrow(), |w| w.downcast_ref::<T>().unwrap())
    }

    pub fn widget_mut<T: Widget>(&self, handle: Handle) -> RefMut<T> {
        RefMut::map(self.widget(handle).borrow_mut(), |w| w.downcast_mut::<T>().unwrap())
    }

    /// Creates "ghost" of the current cursor - a copy that will be drawn at the same position
    /// beneath the real cursor until hidden. Only one cursor ghost is possible.
    pub fn show_cursor_ghost(&mut self) {
        self.cursor_ghost = Some((self.cursor_pos, self.effective_cursor()));
    }

    /// Hides cursor ghost shown by `show_cursor_ghost()`.
    pub fn hide_cursor_ghost(&mut self) {
        if let Some((pos, _)) = self.cursor_ghost.take() {
            self.cursor_pos = pos;
            self.simulate_mouse_move = true;
        }
    }

    #[must_use]
    pub fn window_of(&self, handle: Handle) -> Option<Handle> {
        for &winh in &self.windows_order {
            let win = self.widget_ref::<Window>(winh);
            if win.widgets.contains(&handle) {
                return Some(winh);
            }
        }
        None
    }

    pub fn cursor_pos(&self) -> Point {
        self.cursor_pos
    }

    pub fn set_cursor_pos(&mut self, pos: Point) {
        self.update_cursor_pos_abs(pos);
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    pub fn capture(&mut self, widget: Handle) {
        self.capture = Some(widget);
    }

    pub fn release(&mut self) {
        self.capture = None;
    }

    /// Limits cursor position to the specified `rect`.
    pub fn set_cursor_constraint(&mut self, rect: Rect) {
        if self.cursor_constraints.len() == 1 {
            self.cursor_constraints.push(rect);
        } else {
            self.cursor_constraints[1] = rect;
        }
    }

    /// Removes constraint set by `set_cursor_constraint()`.
    pub fn clear_cursor_constraint(&mut self) {
        self.cursor_constraints.truncate(1);
    }

    fn widget_handle_event(&mut self,
        now: Instant,
        target: Handle,
        event: Event,
        out: &mut Vec<out::OutEvent>)
    {
        self.widgets[target.0].borrow_mut().handle_event(HandleEvent {
            now,
            this: target,
            base: &mut self.widget_bases[target.0].borrow_mut(),
            event,
            capture: &mut self.capture,
            out,
            cursor_pos: self.cursor_pos,
        });
    }

    pub fn handle_input(&mut self, ctx: HandleInput) -> bool {
        match ctx.event {
            SdlEvent::MouseButtonDown { mouse_btn, .. } => {
                let target = if let Some(h) = self.update_mouse_focus(ctx.now, ctx.out) {
                    h
                } else {
                    return false;
                };
                self.widget_handle_event(ctx.now, target,
                    Event::MouseDown { pos: self.cursor_pos, button: *mouse_btn }, ctx.out);
            }
            SdlEvent::MouseMotion { xrel, yrel, .. } => {
                self.simulate_mouse_move = false;

                self.update_cursor_pos_rel(Point::new(*xrel, *yrel));
                if !self.fire_mouse_move(ctx.now, ctx.out) {
                    return false;
                }
            }
            SdlEvent::MouseButtonUp { mouse_btn, .. } => {
                let target = if let Some(h) = self.update_mouse_focus(ctx.now, ctx.out) {
                    h
                } else {
                    return false;
                };
                self.widget_handle_event(ctx.now, target,
                    Event::MouseUp { pos: self.cursor_pos, button: *mouse_btn }, ctx.out);
            }
            _ => return false,
        }
        true
    }

    pub fn update(&mut self, now: Instant, out: &mut Vec<out::OutEvent>) {
        if self.simulate_mouse_move {
            self.simulate_mouse_move = false;
            self.fire_mouse_move(now, out);
        }

        // FIXME avoid copy/allocation
        let handles: Vec<_> = self.widgets.keys().collect();
        for h in handles {
            self.widget_handle_event(now, Handle(h), Event::Tick, out);
        }
    }

    pub fn sync(&mut self) {
        for (h, w) in &self.widgets {
            w.borrow_mut().sync(Sync {
                base: &mut self.widget_bases[h].borrow_mut(),
                cursor_pos: self.cursor_pos,
            });
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas) {
        for &winh in &self.windows_order {
            let mut win = self.widgets[winh.0].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            let has_mouse_focus = self.mouse_focus.is_some() &&
                win.widgets.contains(&self.mouse_focus.unwrap());
            self.widget_bases[winh.0].borrow_mut().render(Render {
                frm_db: &self.frm_db,
                canvas,
                base: None,
                cursor_pos: self.cursor_pos,
                has_mouse_focus,
            });
            win.render(Render {
                frm_db: &self.frm_db,
                canvas,
                base: Some(&self.widget_bases[winh.0].borrow()),
                cursor_pos: self.cursor_pos,
                has_mouse_focus,
            });
            for &widgh in &win.widgets {
                let has_mouse_focus = self.mouse_focus == Some(widgh);
                self.widget_bases[widgh.0].borrow_mut().render(Render {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[winh.0].borrow()),
                    cursor_pos: self.cursor_pos,
                    has_mouse_focus,
                });
                self.widgets[widgh.0].borrow_mut().render(Render {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[widgh.0].borrow()),
                    cursor_pos: self.cursor_pos,
                    has_mouse_focus,
                });
            }
        }

        if let Some((pos, cursor)) = self.cursor_ghost {
            self.draw_cursor(cursor, pos, canvas);
        }

        let cursor = self.effective_cursor();
        self.draw_cursor(cursor, self.cursor_pos, canvas);
    }

    fn effective_cursor(&self) -> Cursor {
        self.capture
            .or_else(|| self.widget_at(self.cursor_pos))
            .and_then(|h| self.widget_bases[h.0].borrow().cursor)
            .unwrap_or(self.cursor)
    }

    fn draw_cursor(&self, cursor: Cursor, pos: Point, canvas: &mut dyn Canvas) {
        let fid = cursor.fid();
        Sprite {
            pos: pos + cursor.offset(),
            centered: true,
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            light: 0x10000,
            effect: None,
        }.render(canvas, &self.frm_db);
    }

    fn widget_at(&self, point: Point) -> Option<Handle> {
        for &winh in self.windows_order.iter().rev() {
            let win_base = self.widget_bases[winh.0].borrow();
            if win_base.rect.contains(point.x, point.y) {
                let mut win = self.widgets[winh.0].borrow_mut();
                let win = win.downcast_mut::<Window>().unwrap();
                for &widgh in win.widgets.iter().rev() {
                    let widg_base = self.widget_bases[widgh.0].borrow();
                    if widg_base.rect.contains(point.x, point.y) {
                        return Some(widgh);
                    }
                }
                return Some(winh);
            }
        }
        None
    }

    fn find_mouse_event_target(&self) -> Option<Handle> {
        if self.capture.is_some() {
            self.capture
        } else {
            self.widget_at(self.cursor_pos)
        }
    }

    fn update_mouse_focus(&mut self, now: Instant, out: &mut Vec<OutEvent>)
        -> Option<Handle>
    {
        let target = self.find_mouse_event_target()?;
        if self.mouse_focus.is_some() && self.mouse_focus != Some(target) {
            self.widget_handle_event(now, self.mouse_focus.unwrap(), Event::MouseLeave, out);
        }
        self.mouse_focus = Some(target);
        Some(target)
    }

    fn fire_mouse_move(&mut self, now: Instant, out: &mut Vec<OutEvent>) -> bool {
        if let Some(target) = self.update_mouse_focus(now, out) {
            self.widget_handle_event(now, target,
                Event::MouseMove { pos: self.cursor_pos }, out);
            true
        } else {
            false
        }
    }

    fn update_cursor_pos_rel(&mut self, rel: Point) {
        let abs = self.cursor_pos + rel;
        self.update_cursor_pos_abs(abs);
    }

    fn update_cursor_pos_abs(&mut self, abs: Point) {
        let rect = *self.cursor_constraints.last().unwrap();
        self.cursor_pos = abs.clamp_in_rect(rect);
    }

    fn insert_widget(&mut self, window: Option<Handle>, base: Base, widget: Box<Widget>) -> Handle {
        let k = self.widget_handles.insert(());
        self.widget_bases.insert(k, RefCell::new(base));
        self.widgets.insert(k, RefCell::new(widget));

        let h = Handle(k);

        if let Some(winh) = window {
            let mut win = self.widgets[winh.0].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            win.widgets.push(h);
        }

        self.widgets[k].borrow_mut().init(Init {
            base: &mut self.widget_bases[k].borrow_mut(),
        });

        h
    }
}

pub struct Window {
    widgets: Vec<Handle>,
}

impl Widget for Window {
    fn handle_event(&mut self, _ctx: HandleEvent) {
    }

    fn render(&mut self, _ctx: Render) {
    }
}

pub struct Base {
    rect: Rect,
    cursor: Option<Cursor>,
    background: Option<Sprite>,
}

impl Base {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn cursor(&self) -> Option<Cursor> {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: Option<Cursor>) {
        self.cursor = cursor;
    }

    pub fn background(&self) -> Option<&Sprite> {
        self.background.as_ref()
    }

    pub fn background_mut(&mut self) -> Option<&mut Sprite> {
        self.background.as_mut()
    }
}

impl Widget for Base {
    fn handle_event(&mut self, _ctx: HandleEvent) {
    }

    fn render(&mut self, ctx: Render) {
        if let Some(background) = &mut self.background {
            background.pos = self.rect.top_left();
            background.render(ctx.canvas, ctx.frm_db);
        }
    }
}

pub struct HandleEvent<'a> {
    pub now: Instant,
    pub this: Handle,
    pub base: &'a mut Base,
    pub event: Event,
    capture: &'a mut Option<Handle>,
    pub out: &'a mut Vec<out::OutEvent>,
    pub cursor_pos: Point,
}

impl HandleEvent<'_> {
    pub fn is_captured(&self) -> bool {
        self.capture.is_some()
    }

    pub fn capture(&mut self) {
        *self.capture = Some(self.this);
    }

    pub fn release(&mut self) {
        *self.capture = None;
    }
}

pub struct Render<'a> {
    pub frm_db: &'a FrameDb,
    pub canvas: &'a mut Canvas,
    pub base: Option<&'a Base>,
    pub cursor_pos: Point,
    pub has_mouse_focus: bool,
}

pub struct Init<'a> {
    pub base: &'a mut Base,
}

pub struct Sync<'a> {
    pub base: &'a mut Base,
    pub cursor_pos: Point,
}

pub trait Widget: Downcast {
    fn init(&mut self, _ctx: Init) {}

    fn handle_event(&mut self, ctx: HandleEvent);

    /// Called after `hanled_event()` and before `render()`.
    /// Should be used to sync the directly altered widget state to the UI.
    fn sync(&mut self, _ctx: Sync) {}

    fn render(&mut self, ctx: Render);
}

impl_downcast!(Widget);
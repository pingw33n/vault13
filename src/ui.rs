pub mod button;
pub mod command;
pub mod image_text;
pub mod message_panel;
pub mod panel;

pub use sdl2::mouse::MouseButton;
pub use sdl2::keyboard::Keycode;

use downcast_rs::{Downcast, impl_downcast};
use enum_map_derive::Enum;
use sdl2::event::{Event as SdlEvent};
use slotmap::{SecondaryMap, SlotMap};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::time::Instant;

use crate::asset::frame::{FrameId, FrameDb};
use crate::graphics::{Point, Rect};
use crate::graphics::font::Fonts;
use crate::graphics::geometry::hex::Direction;
use crate::graphics::render::Canvas;
use crate::graphics::sprite::Sprite;
use crate::ui::command::UiCommand;
use crate::util::VecExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    KeyDown {
        keycode: Option<Keycode>,
    },
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cursor {
    ActionArrow,
    ActionArrowFlipped,
    Arrow,
    ArrowDown,
    ArrowUp,
    CrosshairUse,

    ScrollNorth,
    ScrollNorthEast,
    ScrollEast,
    ScrollSouthEast,
    ScrollSouth,
    ScrollSouthWest,
    ScrollWest,
    ScrollNorthWest,
    ScrollNorthX,
    ScrollNorthEastX,
    ScrollEastX,
    ScrollSouthEastX,
    ScrollSouthX,
    ScrollSouthWestX,
    ScrollWestX,
    ScrollNorthWestX,

    Hidden,

    Frame(FrameId),
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
            CrosshairUse => FrameId::CROSSHAIR_USE,

            ScrollNorth => FrameId::SCRNORTH,
            ScrollNorthEast => FrameId::SCRNEAST,
            ScrollEast => FrameId::SCREAST,
            ScrollSouthEast => FrameId::SCRSEAST,
            ScrollSouth => FrameId::SCRSOUTH,
            ScrollSouthWest => FrameId::SCRSWEST,
            ScrollWest => FrameId::SCRWEST,
            ScrollNorthWest => FrameId::SCRNWEST,
            ScrollNorthX => FrameId::SCRNX,
            ScrollNorthEastX => FrameId::SCRNEX,
            ScrollEastX => FrameId::SCREX,
            ScrollSouthEastX => FrameId::SCRSEX,
            ScrollSouthX => FrameId::SCRSX,
            ScrollSouthWestX => FrameId::SCRSWX,
            ScrollWestX => FrameId::SCRWX,
            ScrollNorthWestX => FrameId::SCRNWX,

            Hidden => FrameId::BLANK,
            Frame(v) => v,
        }
    }

    fn placement(self, frm_db: &FrameDb) -> (Point, bool) {
        use Cursor::*;
        let offset = match self {
            // ACTARROM is not properly centered.
            ActionArrowFlipped => (-14, 22),
            Frame(fid) => {
                return (-frm_db.get(fid).unwrap().first().size() / 2, false);
            }
            _ => (0, 0)
        }.into();
        (offset, true)
    }
}

new_handle_type! {
    pub struct Handle;
}

pub struct HandleInput<'a> {
    pub now: Instant,
    pub event: &'a SdlEvent,
    pub out: &'a mut Vec<command::UiCommand>,
}

pub struct Ui {
    frm_db: Rc<FrameDb>,
    fonts: Rc<Fonts>,
    widget_handles: SlotMap<Handle, ()>,
    widget_bases: SecondaryMap<Handle, RefCell<Base>>,
    widgets: SecondaryMap<Handle, RefCell<Box<dyn Widget>>>,
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
    keyboard_focus: Option<Handle>,
    modal_window: Option<Handle>,
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
            keyboard_focus: None,
            modal_window: None,
        }
    }

    pub fn fonts(&self) -> &Rc<Fonts> {
        &self.fonts
    }

    pub fn frm_db(&self) -> &Rc<FrameDb> {
        &self.frm_db
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
        let widg = if let Some(v) = self.widgets.remove(handle) {
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
        if self.keyboard_focus == Some(handle) {
            self.keyboard_focus = None;
        }
        if self.modal_window == Some(handle) {
            self.modal_window = None;
        }
        self.widget_bases.remove(handle);

        let widg = widg.borrow();
        if let Some(win) = widg.downcast_ref::<Window>() {
            self.windows_order.remove_first(&handle).unwrap();
            for &w in &win.widgets {
                self.remove(w);
            }
        } else if let Some(win) = self.window_of(handle) {
            // Remove widget from its window.
            let mut win = self.widgets[win].borrow_mut();
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
            let win_base = self.widget_bases[window].borrow();
            let top_left = win_base.rect.top_left();
            rect.translate(top_left)
        };

        let h = self.insert_widget(Some(window), Base {
            rect,
            cursor,
            background,
        }, Box::new(widget));

        self.simulate_mouse_move = true;

        h
    }

    pub fn widget_base_ref(&self, handle: Handle) -> Ref<Base> {
        self.widget_bases[handle].borrow()
    }

    pub fn widget_base_mut(&self, handle: Handle) -> RefMut<Base> {
        self.widget_bases[handle].borrow_mut()
    }

    pub fn widget(&self, handle: Handle) -> &RefCell<Box<dyn Widget>> {
        &self.widgets[handle]
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

    pub fn keyboard_focus(&self) -> Option<Handle> {
        self.keyboard_focus
    }

    pub fn set_keyboard_focus(&mut self, widget: Option<Handle>) {
        assert!(widget.is_none() || self.widgets.contains_key(widget.unwrap()));
        self.keyboard_focus = widget;
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

    pub fn is_window(&self, handle: Handle) -> bool {
        self.widget(handle).borrow().downcast_ref::<Window>().is_some()
    }

    pub fn set_modal_window(&mut self, win: Option<Handle>) {
        if let Some(win) = win {
            assert!(self.is_window(win));
        }
        self.modal_window = win;
    }

    fn widget_handle_event(&mut self,
        now: Instant,
        target: Handle,
        event: Event,
        out: &mut Vec<command::UiCommand>)
    {
        self.widgets[target].borrow_mut().handle_event(HandleEvent {
            now,
            this: target,
            base: &mut self.widget_bases[target].borrow_mut(),
            event,
            capture: &mut self.capture,
            out,
            cursor_pos: self.cursor_pos,
        });
    }

    fn keyboard_event_target(&self) -> Option<Handle> {
        self.capture.or(self.keyboard_focus)
    }

    pub fn handle_input(&mut self, ctx: HandleInput) -> bool {
        match *ctx.event {
            SdlEvent::KeyDown { keycode, .. } => {
                if let Some(target) = self.keyboard_event_target() {
                    self.widget_handle_event(ctx.now, target, Event::KeyDown { keycode }, ctx.out);
                } else {
                    return false;
                }
            }
            SdlEvent::MouseButtonDown { mouse_btn, .. } => {
                let target = if let Some(h) = self.update_mouse_focus(ctx.now, ctx.out) {
                    h
                } else {
                    return false;
                };
                self.widget_handle_event(ctx.now, target,
                    Event::MouseDown { pos: self.cursor_pos, button: mouse_btn }, ctx.out);
            }
            SdlEvent::MouseMotion { xrel, yrel, .. } => {
                self.simulate_mouse_move = false;

                self.update_cursor_pos_rel(Point::new(xrel, yrel));
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
                    Event::MouseUp { pos: self.cursor_pos, button: mouse_btn }, ctx.out);
            }
            _ => return false,
        }
        true
    }

    pub fn update(&mut self, now: Instant, out: &mut Vec<command::UiCommand>) {
        if self.simulate_mouse_move {
            self.simulate_mouse_move = false;
            self.fire_mouse_move(now, out);
        }

        // FIXME avoid copy/allocation
        let handles: Vec<_> = self.widgets.keys().collect();
        for h in handles {
            self.widget_handle_event(now, h, Event::Tick, out);
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

    pub fn render(&mut self, canvas: &mut dyn Canvas) {
        for &winh in &self.windows_order {
            let mut win = self.widgets[winh].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            let has_mouse_focus = self.mouse_focus.is_some() &&
                win.widgets.contains(&self.mouse_focus.unwrap());
            self.widget_bases[winh].borrow_mut().render(Render {
                frm_db: &self.frm_db,
                canvas,
                base: None,
                cursor_pos: self.cursor_pos,
                has_mouse_focus,
            });
            win.render(Render {
                frm_db: &self.frm_db,
                canvas,
                base: Some(&self.widget_bases[winh].borrow()),
                cursor_pos: self.cursor_pos,
                has_mouse_focus,
            });
            for &widgh in &win.widgets {
                let has_mouse_focus = self.mouse_focus == Some(widgh);
                self.widget_bases[widgh].borrow_mut().render(Render {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[winh].borrow()),
                    cursor_pos: self.cursor_pos,
                    has_mouse_focus,
                });
                self.widgets[widgh].borrow_mut().render(Render {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[widgh].borrow()),
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
        let widg_cursor = |h| self.widget_bases[h].borrow().cursor;
        if let Some(v) = self.capture.and_then(widg_cursor) {
            return v;
        }
        if let Some(v) = self.widget_at(self.cursor_pos)
            .and_then(|(widg, win)| widg_cursor(widg).or_else(|| widg_cursor(win)))
        {
            return v;
        }
        if let Some(v) = self.modal_window.and_then(widg_cursor) {
            return v;
        }
        self.cursor
    }

    fn draw_cursor(&self, cursor: Cursor, pos: Point, canvas: &mut dyn Canvas) {
        let fid = cursor.fid();
        let (offset, centered) = cursor.placement(&self.frm_db);
        Sprite {
            pos: pos + offset,
            centered,
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            light: 0x10000,
            effect: None,
        }.render(canvas, &self.frm_db);
    }

    fn widget_at(&self, point: Point) -> Option<(Handle, Handle)> {
        for &winh in self.windows_order.iter().rev() {
            let win_base = self.widget_bases[winh].borrow();
            if win_base.rect.contains(point) {
                let mut win = self.widgets[winh].borrow_mut();
                let win = win.downcast_mut::<Window>().unwrap();
                for &widgh in win.widgets.iter().rev() {
                    let widg_base = self.widget_bases[widgh].borrow();
                    if widg_base.rect.contains(point) {
                        return Some((widgh, winh));
                    }
                }
                return Some((winh, winh));
            }
            if Some(winh) == self.modal_window {
                break;
            }
        }
        None
    }

    fn find_mouse_event_target(&self) -> Option<Handle> {
        if self.capture.is_some() {
            self.capture
        } else {
            self.widget_at(self.cursor_pos).map(|(widg, _)| widg)
        }
    }

    fn update_mouse_focus(&mut self, now: Instant, out: &mut Vec<UiCommand>)
        -> Option<Handle>
    {
        let target = self.find_mouse_event_target()?;
        if self.mouse_focus.is_some() && self.mouse_focus != Some(target) {
            self.widget_handle_event(now, self.mouse_focus.unwrap(), Event::MouseLeave, out);
        }
        self.mouse_focus = Some(target);
        Some(target)
    }

    fn fire_mouse_move(&mut self, now: Instant, out: &mut Vec<UiCommand>) -> bool {
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

    fn insert_widget(&mut self, window: Option<Handle>, base: Base, widget: Box<dyn Widget>) -> Handle {
        let h = self.widget_handles.insert(());
        self.widget_bases.insert(h, RefCell::new(base));
        self.widgets.insert(h, RefCell::new(widget));

        if let Some(winh) = window {
            let mut win = self.widgets[winh].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            win.widgets.push(h);
        }

        self.widgets[h].borrow_mut().init(Init {
            base: &mut self.widget_bases[h].borrow_mut(),
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
    pub out: &'a mut Vec<command::UiCommand>,
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

    pub fn out(&mut self, event_data: command::UiCommandData) {
        self.out.push(UiCommand {
            source: self.this,
            data: event_data,
        });
    }
}

pub struct Render<'a> {
    pub frm_db: &'a FrameDb,
    pub canvas: &'a mut dyn Canvas,
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

    fn handle_event(&mut self, _ctx: HandleEvent) {}

    /// Called after `handle_event()` and before `render()`.
    /// Should be used to sync the directly altered widget state to the UI.
    fn sync(&mut self, _ctx: Sync) {}

    fn render(&mut self, ctx: Render);
}

impl_downcast!(Widget);
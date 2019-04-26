use downcast_rs::{Downcast, impl_downcast};
use enum_map_derive::Enum;
use sdl2::event::{Event as SdlEvent};
use slotmap::{SecondaryMap, SlotMap};
use std::cell::RefCell;
use std::rc::Rc;

use crate::asset::frm::{Fid, FrmDb};
use crate::graphics::{Point, Rect};
use crate::graphics::geometry::hex::Direction;
use crate::graphics::render::Canvas;
use crate::graphics::sprite::Sprite;
use crate::util::SmKey;
use sdl2::mouse::MouseButton;

pub mod button;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    MouseDown {
        pos: Point,
        button: MouseButton,
    },
    MouseMove {
        pos: Point,
    },
    MouseUp {
        pos: Point,
        button: MouseButton,
    },
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd)]
pub enum Cursor {
    Arrow,
    ArrowDown,
    ArrowUp,
    Hidden,
}

impl Cursor {
    pub fn fid(self) -> Fid {
        use Cursor::*;
        match self {
            Arrow => Fid::STDARROW,
            ArrowDown => Fid::SDNARROW,
            ArrowUp => Fid::SUPARROW,
            Hidden => Fid::BLANK,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Handle(SmKey);


pub struct Ui {
    frm_db: Rc<FrmDb>,
    widget_handles: SlotMap<SmKey, ()>,
    widget_bases: SecondaryMap<SmKey, RefCell<Base>>,
    widgets: SecondaryMap<SmKey, RefCell<Box<Widget>>>,
    windows_order: Vec<Handle>,
    cursor_pos: Point,
    cursor_frozen: bool,
    pub cursor: Cursor,
    capture: Option<Handle>,
}

impl Ui {
    pub fn new(frm_db: Rc<FrmDb>) -> Self {
        Self {
            frm_db,
            widget_handles: SlotMap::with_key(),
            widget_bases: SecondaryMap::new(),
            widgets: SecondaryMap::new(),
            windows_order: Vec::new(),
            cursor_pos: Point::new(0, 0),
            cursor_frozen: false,
            cursor: Cursor::Arrow,
            capture: None,
        }
    }

    pub fn new_window(&mut self, rect: Rect, background: Option<Sprite>) -> Handle {
        let k = self.widget_handles.insert(());
        self.widget_bases.insert(k, RefCell::new(Base {
            rect,
            cursor: None,
            background,
        }));
        self.widgets.insert(k, RefCell::new(Box::new(Window {
            widgets: Vec::new(),
        })));
        let h = Handle(k);
        self.windows_order.push(h);
        h
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

        let k = self.widget_handles.insert(());
        self.widget_bases.insert(k, RefCell::new(Base {
            rect,
            cursor,
            background,
        }));
        self.widgets.insert(k, RefCell::new(Box::new(widget)));
        let h = Handle(k);
        let mut win = self.widgets[window.0].borrow_mut();
        let win = win.downcast_mut::<Window>().unwrap();
        win.widgets.push(h);
        h
    }

    pub fn handle_input(&mut self, event: &SdlEvent) -> bool {
        match event {
            SdlEvent::MouseButtonDown { x, y, mouse_btn, .. } => {
                let pos = Point::new(*x, *y);

                self.maybe_update_cursor_pos(pos);

                let target = if let Some(capture) = self.capture {
                    capture
                } else if let Some(h) = self.widget_at(self.cursor_pos) {
                    h
                } else {
                    return false;
                };
                self.widgets[target.0].borrow_mut().handle_event(HandleEvent {
                    this: target,
                    base: &self.widget_bases[target.0].borrow(),
                    event: Event::MouseDown { pos, button: *mouse_btn },
                    capture: &mut self.capture,
                });
            }
            SdlEvent::MouseMotion { x, y, .. } => {
                let pos = Point::new(*x, *y);

                self.maybe_update_cursor_pos(pos);

                let target = if let Some(capture) = self.capture {
                    capture
                } else if let Some(h) = self.widget_at(self.cursor_pos) {
                    h
                } else {
                    return false;
                };
                self.widgets[target.0].borrow_mut().handle_event(HandleEvent {
                    this: target,
                    base: &self.widget_bases[target.0].borrow(),
                    event: Event::MouseMove { pos },
                    capture: &mut self.capture,
                });
            }
            SdlEvent::MouseButtonUp { x, y, mouse_btn, .. } => {
                let pos = Point::new(*x, *y);

                self.maybe_update_cursor_pos(pos);

                let target = if let Some(capture) = self.capture {
                    capture
                } else if let Some(h) = self.widget_at(self.cursor_pos) {
                    h
                } else {
                    return false;
                };
                self.widgets[target.0].borrow_mut().handle_event(HandleEvent {
                    this: target,
                    base: &self.widget_bases[target.0].borrow(),
                    event: Event::MouseUp { pos, button: *mouse_btn },
                    capture: &mut self.capture,
                });
            }
            _ => return false,
        }
        true
    }

    pub fn render(&mut self, canvas: &mut Canvas) {
        for &winh in &self.windows_order {
            self.widget_bases[winh.0].borrow_mut().render(RenderContext {
                frm_db: &self.frm_db,
                canvas,
                base: None,
            });
            let mut win = self.widgets[winh.0].borrow_mut();
            let win = win.downcast_mut::<Window>().unwrap();
            win.render(RenderContext {
                frm_db: &self.frm_db,
                canvas,
                base: Some(&self.widget_bases[winh.0].borrow()),
            });
            for &widgh in &win.widgets {
                self.widget_bases[widgh.0].borrow_mut().render(RenderContext {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[winh.0].borrow()),
                });
                self.widgets[widgh.0].borrow_mut().render(RenderContext {
                    frm_db: &self.frm_db,
                    canvas,
                    base: Some(&self.widget_bases[widgh.0].borrow()),
                });
            }
        }

        let cursor = self.effective_cursor();
        let fid = cursor.fid();
        Sprite {
            pos: self.cursor_pos,
            centered: true,
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            light: 0x10000,
            effect: None,
        }.render(canvas, &self.frm_db);
    }

    fn effective_cursor(&self) -> Cursor {
        self.widget_at(self.cursor_pos)
            .and_then(|h| self.widget_bases[h.0].borrow().cursor)
            .unwrap_or(self.cursor)
    }

    fn widget_at(&self, point: Point) -> Option<Handle> {
        for &winh in &self.windows_order {
            let win_base = self.widget_bases[winh.0].borrow();
            if win_base.rect.contains(point.x, point.y) {
                let mut win = self.widgets[winh.0].borrow_mut();
                let win = win.downcast_mut::<Window>().unwrap();
                for &widgh in &win.widgets {
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

    fn maybe_update_cursor_pos(&mut self, pos: Point) {
        if !self.cursor_frozen {
            self.cursor_pos = pos;
        }
    }
}

pub struct Window {
    widgets: Vec<Handle>,
}

impl Widget for Window {
    fn handle_event(&mut self, _ctx: HandleEvent) {
    }

    fn render(&mut self, _ctx: RenderContext) {
    }
}

pub struct Base {
    rect: Rect,
    cursor: Option<Cursor>,
    background: Option<Sprite>,
}

impl Widget for Base {
    fn handle_event(&mut self, _ctx: HandleEvent) {
    }

    fn render(&mut self, ctx: RenderContext) {
        if let Some(background) = &mut self.background {
            background.pos = self.rect.top_left();
            background.render(ctx.canvas, ctx.frm_db);
        }
    }
}

pub struct HandleEvent<'a> {
    this: Handle,
    base: &'a Base,
    event: Event,
    capture: &'a mut Option<Handle>,
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

pub struct RenderContext<'a> {
    frm_db: &'a FrmDb,
    canvas: &'a mut Canvas,
    base: Option<&'a Base>,
}

pub trait Widget: Downcast {
    fn handle_event(&mut self, ctx: HandleEvent);

    fn render(&mut self, ctx: RenderContext);
}

impl_downcast!(Widget);
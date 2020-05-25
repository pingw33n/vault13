use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant};

use crate::asset::Flag;
use crate::asset::frame::FrameId;
use crate::game::world::World;
use crate::game::object::{self, Object};
use crate::graphics::{EPoint, Point};
use crate::graphics::color;
use crate::graphics::font::*;
use crate::graphics::geometry::TileGridView;
use crate::graphics::geometry::hex::Direction;
use crate::graphics::render;
use crate::graphics::sprite::{OutlineStyle, Sprite};
use crate::ui::*;
use crate::ui::command::{UiCommand, UiCommandData, ObjectPickKind};

use super::action_menu::{Action, Placement};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PickMode {
    Hex,
    Object,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HexCursorStyle {
    Normal,
    Blocked,
}

#[derive(Clone, Copy, Debug)]
enum PickState {
    Idle,
    Pending {
        start: Instant,
        pos: Point,
    },
}

pub struct WorldView {
    world: Rc<RefCell<World>>,
    pick_mode: PickMode,
    hex_cursor: object::Handle,
    pub hex_cursor_style: HexCursorStyle,
    pub roof_visible: bool,
    pick_state: PickState,
    action_menu_state: Option<(Instant, object::Handle)>,

    /// Icon displayed near the cursor in object pick mode.
    pub default_action_icon: Option<Action>,
}

impl WorldView {
    pub fn new(world: Rc<RefCell<World>>) -> Self {
        let hex_cursor = Self::insert_hex_cursor(&mut world.borrow_mut());

        Self {
            world,
            pick_mode: PickMode::Hex,
            hex_cursor,
            hex_cursor_style: HexCursorStyle::Normal,
            roof_visible: false,
            pick_state: PickState::Idle,
            action_menu_state: None,
            default_action_icon: None,
        }
    }

    pub fn hex_cursor_pos(&self) -> Option<EPoint> {
        if self.pick_mode == PickMode::Hex {
            let world = self.world.borrow();
            let cursor = world.objects().get(self.hex_cursor);
            cursor.pos
        } else {
            None
        }
    }

    pub fn ensure_hex_cursor(&mut self) {
        let world = &mut self.world.borrow_mut();
        if !world.objects().contains(self.hex_cursor) {
            self.hex_cursor = Self::insert_hex_cursor(world);
        }
    }

    fn insert_hex_cursor(world: &mut World) -> object::Handle {
        let mut hex_cursor = Object::new(FrameId::MOUSE_HEX_OUTLINE, None,
            Some(Default::default()));
        hex_cursor.flags = Flag::WalkThru | Flag::Flat | Flag::NoBlock | Flag::Temp |
            Flag::LightThru | Flag::ShootThru;
        hex_cursor.outline = Some(object::Outline {
            style: OutlineStyle::Red,
            translucent: true,
            disabled: false,
        });
        world.insert_object(hex_cursor)
    }

    fn update_hex_cursor_pos(&mut self, screen_pos: Point) -> (EPoint, bool) {
        let mut world = self.world.borrow_mut();
        let hex_pos = world.camera().hex().from_screen(screen_pos);
        let pos = EPoint::new(world.elevation(), hex_pos);
        let old_pos = world.objects().get(self.hex_cursor).pos;
        let changed = if Some(pos) != old_pos {
            world.set_object_pos(self.hex_cursor, pos);
            true
        } else {
            false
        };
        (pos, changed)
    }

    fn update_hex_cursor_visibility(&mut self, force_visible: Option<bool>) {
        let mut world = self.world.borrow_mut();
        let mut cursor = world.objects_mut().get_mut(self.hex_cursor);
        let visible = force_visible.unwrap_or(self.pick_mode == PickMode::Hex);
        if visible {
            cursor.flags.remove(Flag::TurnedOff);
        } else {
            cursor.flags.insert(Flag::TurnedOff);
        }
    }
}

impl Widget for WorldView {
    fn init(&mut self, ctx: Init) {
        ctx.base.set_cursor(Some(Cursor::Hidden));
    }

    fn handle_event(&mut self, ctx: HandleEvent) {
        match ctx.event {
            Event::MouseMove { pos } => {
                match self.pick_mode {
                    PickMode::Hex => {
                        let (pos, changed) = self.update_hex_cursor_pos(pos);
                        if changed {
                            ctx.out.push(UiCommand {
                                source: ctx.this,
                                data: UiCommandData::HexPick { action: false, pos },
                            });
                        }
                    }
                    PickMode::Object => {
                        self.pick_state = PickState::Pending { start: ctx.now, pos };
                        self.default_action_icon = None;
                    }
                }
                self.update_hex_cursor_visibility(None);
            }
            Event::MouseDown { pos, button } => {
                if button == MouseButton::Left && self.pick_mode == PickMode::Object {
                    let world = self.world.borrow();
                    if let Some(obj) = world.pick_object(pos, true) {
                        self.action_menu_state = Some((ctx.now, obj));
                    }
                }
            }
            Event::MouseUp { pos, button } => {
                self.action_menu_state = None;
                match button {
                    MouseButton::Left => {
                        match self.pick_mode {
                            PickMode::Hex => {
                                let (pos, _) = self.update_hex_cursor_pos(pos);
                                ctx.out.push(UiCommand {
                                    source: ctx.this,
                                    data: UiCommandData::HexPick { action: true, pos },
                                });
                            }
                            PickMode::Object => {
                                let world = self.world.borrow();
                                if let Some(obj) = world.pick_object(pos, true) {
                                    ctx.out.push(UiCommand {
                                        source: ctx.this,
                                        data: UiCommandData::ObjectPick {
                                            kind: ObjectPickKind::DefaultAction,
                                            obj,
                                        },
                                    });
                                }
                            }
                        }
                    }
                    MouseButton::Right => {
                        self.pick_mode = match self.pick_mode {
                            PickMode::Hex => {
                                ctx.base.set_cursor(Some(Cursor::ActionArrow));
                                PickMode::Object
                            }
                            PickMode::Object => {
                                ctx.base.set_cursor(Some(Cursor::Hidden));
                                let (pos, changed) = self.update_hex_cursor_pos(pos);
                                if changed {
                                    ctx.out.push(UiCommand {
                                        source: ctx.this,
                                        data: UiCommandData::HexPick { action: false, pos },
                                    });
                                }
                                PickMode::Hex
                            }
                        };
                        self.update_hex_cursor_visibility(None);
                        self.default_action_icon = None;
                    }
                    _ => {}
                }
            }
            Event::MouseLeave => {
                self.update_hex_cursor_visibility(Some(false));
                self.action_menu_state = None;
                self.default_action_icon = None;
                self.pick_state = PickState::Idle;
            }
            Event::Tick => {
                if let Some((time, obj)) = self.action_menu_state {
                    if ctx.now - time >= Duration::from_millis(500) {
                        self.action_menu_state = None;
                        self.default_action_icon = None;

                        ctx.out.push(UiCommand {
                            source: ctx.this,
                            data: UiCommandData::ObjectPick {
                                kind: ObjectPickKind::ActionMenu,
                                obj,
                            },
                        });
                    }
                }

                match self.pick_state {
                    PickState::Idle => {}
                    PickState::Pending { start, pos } => if ctx.now - start >= Duration::from_millis(500) {
                        let world = self.world.borrow();
                        if let Some(obj) = world.pick_object(pos, true) {
                            ctx.out.push(UiCommand {
                                source: ctx.this,
                                data: UiCommandData::ObjectPick {
                                    kind: ObjectPickKind::Hover,
                                    obj,
                                },
                            });
                        }
                        self.pick_state = PickState::Idle;
                    }
                }
            }
            _ => {}
        }
    }

    fn sync(&mut self, ctx: Sync) {
        if ctx.base.cursor() != Some(Cursor::Hidden) {
            ctx.base.set_cursor(Some(
                if self.default_action_icon.is_some() {
                    Placement::new(1, ctx.cursor_pos, ctx.base.rect()).cursor
                } else {
                    Cursor::ActionArrow
                }))
        }
    }

    fn render(&mut self, ctx: Render) {
        let world = self.world.borrow();

        world.render(ctx.canvas, self.roof_visible);

        match self.pick_mode {
            PickMode::Hex => if self.hex_cursor_style == HexCursorStyle::Blocked {
                let hex_cursor = world.objects().get(self.hex_cursor);
                let pos = hex_cursor.pos.unwrap();
                if !hex_cursor.flags.contains(Flag::TurnedOff) && pos.elevation == world.elevation() {
                    let center = world.camera().hex().to_screen(pos.point) + Point::new(16, 8);
                    ctx.canvas.draw_text(b"X".as_ref().into(), center, FontKey::antialiased(1),
                        color::RED, &DrawOptions {
                            horz_align: HorzAlign::Center,
                            vert_align: VertAlign::Middle,
                            dst_color: Some(color::BLACK),
                            outline: Some(render::Outline::Fixed {
                                color: color::BLACK,
                                trans_color: None,
                            }),
                            ..Default::default()
                        });
                }
            }
            PickMode::Object => if let Some(action) = self.default_action_icon {
                let fid = action.icons().0;
                let pos = Placement::new(1, ctx.cursor_pos, ctx.base.unwrap().rect()).rect.top_left();
                Sprite {
                    pos,
                    centered: false,
                    fid,
                    frame_idx: 0,
                    direction: Direction::NE,
                    light: 0x10000,
                    effect: None,
                }.render(ctx.canvas, ctx.frm_db);
            }
        }
    }
}
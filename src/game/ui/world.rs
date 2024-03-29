use matches::matches;
use std::time::{Duration, Instant};

use crate::asset::Flag;
use crate::asset::frame::FrameId;
use crate::game::world::{World, WorldRef};
use crate::game::object;
use crate::graphics::{EPoint, Point};
use crate::graphics::color;
use crate::graphics::font::*;
use crate::graphics::geometry::TileGridView;
use crate::graphics::render;
use crate::graphics::sprite::{OutlineStyle, Sprite};
use crate::ui::*;
use crate::ui::command::{UiCommandData, ObjectPickKind};

use super::action_menu::{Action, Placement};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PickMode {
    Hex,
    Object(ObjectPickMode),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectPickMode {
    Action,
    Skill(crate::asset::Skill),
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
    world: WorldRef,
    pick_mode: PickMode,
    saved_pick_mode: Option<PickMode>,
    hex_cursor: object::Handle,
    pub hex_cursor_style: HexCursorStyle,
    pub roof_visible: bool,
    pick_state: PickState,
    action_menu_state: Option<(Instant, object::Handle)>,

    /// Icon displayed near the cursor in object pick mode.
    pub default_action_icon: Option<Action>,
}

impl WorldView {
    pub fn new(world: WorldRef) -> Self {
        let hex_cursor = Self::insert_hex_cursor(&mut world.borrow_mut());

        Self {
            world,
            pick_mode: PickMode::Hex,
            saved_pick_mode: None,
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
            Some(cursor.pos())
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

    pub fn enter_skill_target_pick_mode(&mut self, skill: crate::asset::Skill) {
        self.saved_pick_mode = Some(self.pick_mode);
        self.pick_mode = PickMode::Object(ObjectPickMode::Skill(skill));
    }

    fn insert_hex_cursor(world: &mut World) -> object::Handle {
        let mut hex_cursor = world.objects_mut().create(
            Some(FrameId::MOUSE_HEX_OUTLINE), None, Some(Default::default()), None);
        hex_cursor.flags = Flag::WalkThru | Flag::Flat | Flag::NoBlock | Flag::Temp |
            Flag::LightThru | Flag::ShootThru;
        hex_cursor.outline = Some(object::Outline {
            style: OutlineStyle::Red,
            translucent: true,
            disabled: false,
        });
        hex_cursor.handle()
    }

    fn update_hex_cursor_pos(&mut self, screen_pos: Point) -> (EPoint, bool) {
        let mut world = self.world.borrow_mut();
        let hex_pos = world.camera().hex().screen_to_tile(screen_pos);
        let pos = EPoint::new(world.elevation(), hex_pos);
        let old_pos = world.objects().get(self.hex_cursor).pos();
        let changed = pos != old_pos;
        if changed {
            world.objects_mut().set_pos(self.hex_cursor, Some(pos));
        }
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
    fn handle_event(&mut self, mut ctx: HandleEvent) {
        match ctx.event {
            Event::MouseMove { pos } => {
                match self.pick_mode {
                    PickMode::Hex => {
                        let (pos, changed) = self.update_hex_cursor_pos(pos);
                        if changed {
                            ctx.out(UiCommandData::HexPick { action: false, pos });
                        }
                    }
                    PickMode::Object(ObjectPickMode::Action) => {
                        self.pick_state = PickState::Pending { start: ctx.now, pos };
                        self.default_action_icon = None;
                    }
                    PickMode::Object(ObjectPickMode::Skill(_)) => {}
                }
                self.update_hex_cursor_visibility(None);
            }
            Event::MouseDown { pos, button } => {
                if button == MouseButton::Left &&
                    matches!(self.pick_mode, PickMode::Object(ObjectPickMode::Action))
                {
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
                                ctx.out(UiCommandData::HexPick { action: true, pos });
                            }
                            PickMode::Object(mode) => {
                                let picked_obj = self.world.borrow().pick_object(pos, true);
                                if let Some(obj) = picked_obj {
                                    let kind = match mode {
                                        ObjectPickMode::Action => ObjectPickKind::DefaultAction,
                                        ObjectPickMode::Skill(skill) => {
                                            self.pick_mode = self.saved_pick_mode.take().unwrap();
                                            ObjectPickKind::Skill(skill)
                                        }
                                    };
                                    ctx.out(UiCommandData::ObjectPick { kind, obj });
                                    if self.pick_mode == PickMode::Hex {
                                        self.update_hex_cursor_visibility(None);
                                        let (pos, changed) = self.update_hex_cursor_pos(pos);
                                        if changed {
                                            ctx.out(UiCommandData::HexPick { action: false, pos });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MouseButton::Right => {
                        self.pick_mode = match self.pick_mode {
                            PickMode::Hex => {
                                PickMode::Object(ObjectPickMode::Action)
                            }
                            PickMode::Object(_) => {
                                let (pos, changed) = self.update_hex_cursor_pos(pos);
                                if changed {
                                    ctx.out(UiCommandData::HexPick { action: false, pos });
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

                        ctx.out(UiCommandData::ObjectPick {
                            kind: ObjectPickKind::ActionMenu,
                            obj,
                        });
                    }
                }

                match self.pick_state {
                    PickState::Idle => {}
                    PickState::Pending { start, pos } => if ctx.now - start >= Duration::from_millis(500) {
                        let world = self.world.borrow();
                        if let Some(obj) = world.pick_object(pos, true) {
                            ctx.out(UiCommandData::ObjectPick {
                                kind: ObjectPickKind::Hover,
                                obj,
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
        ctx.base.set_cursor(Some(
            if self.default_action_icon.is_some() {
                Placement::new(1, ctx.cursor_pos, ctx.base.rect()).cursor
            } else {
                match self.pick_mode {
                    PickMode::Hex => Cursor::Hidden,
                    PickMode::Object(ObjectPickMode::Action) => Cursor::ActionArrow,
                    PickMode::Object(ObjectPickMode::Skill(_)) => Cursor::CrosshairUse,
                }
            }));
    }

    fn render(&mut self, ctx: Render) {
        let world = self.world.borrow();

        world.render(ctx.canvas, self.roof_visible);

        match self.pick_mode {
            PickMode::Hex => if self.hex_cursor_style == HexCursorStyle::Blocked {
                let hex_cursor = world.objects().get(self.hex_cursor);
                let pos = hex_cursor.pos();
                if !hex_cursor.flags.contains(Flag::TurnedOff) && pos.elevation == world.elevation() {
                    let center = world.camera().hex().center_to_screen(pos.point);
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
            PickMode::Object(ObjectPickMode::Action) => if let Some(action) = self.default_action_icon {
                let fid = action.icons().0;
                let pos = Placement::new(1, ctx.cursor_pos, ctx.base.unwrap().rect()).rect.top_left();
                Sprite::new_with_pos(fid, pos).render(ctx.canvas, ctx.frm_db);
            }
            PickMode::Object(ObjectPickMode::Skill(_)) => {}
        }
    }
}

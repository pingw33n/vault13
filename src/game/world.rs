pub mod floating_text;

use bstring::{bstr, BString};
use enum_map::Enum;
use if_chain::if_chain;
use log::debug;
use std::cmp;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::asset::{EntityKind, Flag, ExactEntityKind, Stat};
use crate::asset::frame::{FrameId, FrameDb};
use crate::asset::map::ELEVATION_COUNT;
use crate::asset::message::Messages;
use crate::asset::proto::{ProtoDb, ProtoId, ProtoRef, SubProto, SubItem, SubScenery};
use crate::game::GameTime;
use crate::game::object::{self, *};
use crate::graphics::{EPoint, Point, Rect};
use crate::graphics::font::Fonts;
use crate::graphics::geometry::TileGridView;
use crate::graphics::geometry::camera::Camera;
use crate::graphics::geometry::hex::{self, Direction};
use crate::graphics::lighting::light_grid::LightGrid;
use crate::graphics::map::*;
use crate::graphics::render::Canvas;
use crate::util::VecExt;
use crate::util::array2d::Array2d;

use floating_text::FloatingText;
use crate::game::rpg::Rpg;

// scr_game_init()
const START_GAME_TIME: GameTime = GameTime::from_decis(302400);

const MAX_FLOATING_TEXTS: usize = 19;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum ScrollDirection {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl ScrollDirection {
    fn go(self, p: Point) -> Point {
        use ScrollDirection::*;
        let dir = match self {
            N => return hex::go_vert(p, -1),
            S => return hex::go_vert(p, 1),
            NE => Direction::NE,
            E => Direction::E,
            SE => Direction::SE,
            SW => Direction::SW,
            W => Direction::W,
            NW => Direction::NW,
        };
        hex::go(p, dir, 1)
    }
}

pub struct World {
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrameDb>,
    critter_names: Messages,
    hex_grid: hex::TileGrid,
    camera: Camera,
    sqr_tiles: Vec<Option<Array2d<(u16, u16)>>>,
    objects: Objects,
    light_grid: LightGrid,
    floating_texts: Vec<FloatingText>,
    dude_obj: Option<object::Handle>,
    update_time: Instant,
    fonts: Rc<Fonts>,

    pub dude_name: BString,
    pub game_time: GameTime,
    pub ambient_light: u32,
}

impl World {
    pub fn new(
        proto_db: Rc<ProtoDb>,
        frm_db: Rc<FrameDb>,
        critter_names: Messages,
        hex_grid: hex::TileGrid,
        viewport: Rect,
        update_time: Instant,
        fonts: Rc<Fonts>,
    ) -> Self {
        let light_grid = LightGrid::new(
            hex_grid.width(),
            hex_grid.height(),
            ELEVATION_COUNT);
        let objects = Objects::new(hex_grid.clone(), ELEVATION_COUNT, frm_db.clone());
        Self {
            proto_db,
            frm_db,
            critter_names,
            hex_grid,
            camera: Camera {
                origin: Point::new(0, 0),
                viewport,
            },
            sqr_tiles: Vec::with_default(ELEVATION_COUNT as usize),
            objects,
            light_grid,
            floating_texts: Vec::new(),
            dude_obj: None,
            update_time,
            fonts,
            dude_name: BString::new(),
            game_time: START_GAME_TIME,
            ambient_light: 0x10000,
        }
    }

    pub fn proto_db(&self) -> &ProtoDb {
        &self.proto_db
    }

    pub fn frm_db(&self) -> &FrameDb {
        &self.frm_db
    }

    pub fn hex_grid(&self) -> &hex::TileGrid {
        &self.hex_grid
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn objects(&self) -> &Objects {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut Objects {
        &mut self.objects
    }

    pub fn light_grid(&self) -> &LightGrid {
        &self.light_grid
    }

    pub fn clear(&mut self) {
        for v in &mut self.sqr_tiles {
            *v = None;
        }
        self.objects.clear();
        self.floating_texts.clear();
        self.dude_obj = None;
        self.light_grid.clear();
    }

    pub fn set_sqr_tiles(&mut self, sqr_tiles: Vec<Option<Array2d<(u16, u16)>>>) {
        assert_eq!(sqr_tiles.len(), ELEVATION_COUNT as usize);
        self.sqr_tiles = sqr_tiles;
    }

    // obj_new but without script initialization.
    pub fn new_object(&mut self,
        fid: FrameId,
        proto: Option<ProtoRef>,
        pos: Option<EPoint>,
        rpg: &Rpg,
    ) -> object::Handle {
        let mut obj = Object::new(fid, proto, pos, SubObject::None);
        if let Some(proto_flags) = obj.proto().map(|p| p.flags) {
            let flags = proto_flags & (
                Flag::Flat |
                Flag::NoBlock |
                Flag::MultiHex |
                Flag::LightThru |
                Flag::ShootThru |
                Flag::WallTransEnd);
            obj.flags.insert(flags);

            for &flag in &[
                Flag::TransNone,
                Flag::TransEnergy,
                Flag::TransGlass,
                Flag::TransRed,
                Flag::TransSteam,
                Flag::TransWall]
            {
                if proto_flags.contains(flag) {
                    obj.flags.insert(flag);
                    break;
                }
            }
        }
        let objh = self.insert_object(obj);
        let obj = &mut self.objects().get_mut(objh);
        let sub = if let Some(proto) = obj.proto() {
            if proto.kind() == ExactEntityKind::SqrTile {
                return objh;
            }
            // proto_update_init
            match &proto.sub {
                SubProto::Critter(p) => {
                    SubObject::Critter(object::Critter {
                        hit_points: rpg.stat(Stat::HitPoints, obj, self.objects()),
                        radiation: 0,
                        poison: 0,
                        combat: CritterCombat {
                            damage_flags: Default::default(),
                            ai_packet: p.ai_packet,
                            team_id: p.team_id,
                            who_hit_me: 0,
                        },
                    })
                }
                // proto_update_gen
                SubProto::Item(p) => {
                    match &p.sub {
                        SubItem::Ammo(p) => {
                            SubObject::Item(object::Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: None,
                            })
                        }
                        SubItem::Key(p) => {
                            SubObject::Key(object::Key {
                                id: p.id,
                            })
                        }
                        SubItem::Misc(p) => {
                            SubObject::Item(object::Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: p.ammo_proto_id.map(|pid| self.proto_db().proto(pid).unwrap()),
                            })
                        }
                        SubItem::Weapon(p) => {
                            SubObject::Item(object::Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: p.ammo_proto_id.map(|pid| self.proto_db().proto(pid).unwrap()),
                            })
                        }
                        | SubItem::Armor(_)
                        | SubItem::Container(_)
                        | SubItem::Drug(_)
                        => SubObject::None,
                    }
                }
                SubProto::Scenery(p) => {
                    match &p.sub {
                        SubScenery::Door(proto) => {
                            SubObject::Scenery(object::Scenery::Door(object::Door {
                                flags: proto.flags,
                            }))
                        }
                        SubScenery::Stairs(proto) => {
                            SubObject::Scenery(object::Scenery::Stairs(proto.exit.clone().unwrap()))
                        }
                        SubScenery::Elevator(proto) => {
                            SubObject::Scenery(object::Scenery::Elevator(object::Elevator {
                                kind: proto.kind,
                                level: proto.level,
                            }))
                        }
                        SubScenery::Ladder(proto) => {
                            SubObject::Scenery(object::Scenery::Ladder(proto.exit.clone().unwrap()))
                        }
                        SubScenery::Misc => {
                            SubObject::None
                        }
                    }
                }
                SubProto::Wall(_)
                | SubProto::SqrTile(_)
                | SubProto::Misc
                => SubObject::None,
            }
        } else {
            SubObject::None
        };
        obj.sub = sub;
        if obj.sub.as_critter().is_some() {
            rpg.recalc_derived_stats(obj, self.objects());
        }
        // Not initializing script here.
        objh
    }

    pub fn insert_object(&mut self, object: Object) -> object::Handle {
        let h = self.objects.insert(object);

        Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);

        h
    }

    pub fn dude_obj(&self) -> Option<object::Handle> {
        self.dude_obj
    }

    pub fn insert_dude_obj(&mut self, object: object::Object) -> object::Handle {
        assert!(self.dude_obj.is_none());
        let h = self.insert_object(object);
        debug!("dude obj: {:?}", h);
        self.dude_obj = Some(h);
        h
    }

    pub fn remove_dude_obj(&mut self) -> Option<object::Object> {
        let h = self.dude_obj?;
        self.dude_obj = None;
        Some(self.objects_mut().remove(h).unwrap())
    }

    pub fn set_dude_name(&mut self, dude_name: BString) {
        self.dude_name = dude_name;
    }

    pub fn elevation(&self) -> u32 {
        self.objects.get(self.dude_obj.expect("no dude_obj"))
            .pos.expect("dude_obj has no pos")
            .elevation
    }

    pub fn has_elevation(&self, elevation: u32) -> bool {
        self.sqr_tiles[elevation as usize].is_some()
    }

    pub fn set_object_pos(&mut self, h: object::Handle, pos: EPoint) {
        Self::update_light_grid(&self.objects, &mut self.light_grid, h, -1);

        self.objects.set_pos(h, pos);

        Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
    }

    pub fn make_object_standing(&mut self, h: object::Handle) {
        self.objects.make_standing(h, &self.frm_db);
    }

    pub fn path_for_object(&self,
        obj: object::Handle,
        to: PathTo,
        smooth: bool,
    ) -> Option<Vec<Direction>> {
        self.objects.path(obj, to, smooth)
    }

    pub fn rebuild_light_grid(&mut self) {
        self.light_grid.clear();
        for h in self.objects.iter() {
            Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
        }
    }

    pub fn object_bounds(&self, obj: object::Handle, include_outline: bool) -> Rect {
        self.objects.bounds(obj, &self.camera.hex(), include_outline)
    }

    pub fn is_object_in_camera(&self, obj: object::Handle) -> bool {
        let bounds = self.object_bounds(obj, true);
        self.camera.viewport.intersects(bounds)
    }

    pub fn object_hit_test(&self, p: Point) -> Vec<(object::Handle, object::Hit)> {
        self.objects.hit_test(p.elevated(self.elevation()), self.camera.viewport,
            &self.camera.hex(), self.egg())
    }

    // object_under_mouse()
    pub fn pick_object(&self, pos: Point, include_dude: bool) -> Option<object::Handle> {
        let filter_dude = |oh: &&(object::Handle, object::Hit)| -> bool {
            include_dude || Some(oh.0) != self.dude_obj
        };
        let hits = self.object_hit_test(pos);
        let r = hits
            .iter()
            .filter(filter_dude)
            .filter(|(_, h)| !h.translucent && !h.with_egg)
            .filter(|&&(o, _)| {
                let obj = self.objects.get(o);
                if let SubObject::Critter(critter) = &obj.sub {
                    !critter.combat.damage_flags.intersects(DamageFlag::Dead | DamageFlag::KnockedOut)
                } else {
                    true
                }
            })
            .map(|&(o, _)| o)
            .next();
        if r.is_none() {
            hits.iter()
                .filter(filter_dude)
                .map(|&(o, _)| o)
                .next()
        } else {
            r
        }
    }

    // object_name()
    pub fn object_name(&self, obj: object::Handle) -> Option<BString> {
        if Some(obj) == self.dude_obj {
            Some(self.dude_name.clone())
        } else {
            let obj = self.objects.get(obj);
            if_chain! {
                if obj.kind() == EntityKind::Critter;
                if let Some((_, prg_id)) = obj.script;
                if let Some(msg) = self.critter_names.get(prg_id.index() as i32 + 101);
                then {
                    Some(msg.text.clone())
                } else {
                    obj.proto().and_then(|s| s.name().map(|s| s.to_owned()))
                }
            }
        }
    }

    pub fn show_floating_text(&mut self,
        obj: Option<object::Handle>,
        text: &bstr,
        options: floating_text::Options,
    ) -> bool {
        self.hide_floating_text(obj);
        if self.floating_texts.len() < MAX_FLOATING_TEXTS {
            self.floating_texts.push(FloatingText::new(
                obj, text, &self.fonts, options, self.update_time));
            true
        } else {
            false
        }
    }

    pub fn hide_floating_text(&mut self, obj: Option<object::Handle>) {
        self.floating_texts.retain(|ft| ft.obj != obj);
    }

    pub fn update(&mut self, time: Instant) {
        self.update_time = time;
        self.expire_floating_texts();
    }

    pub fn render(&self, canvas: &mut dyn Canvas, draw_roof: bool) {
        let elevation = self.elevation();
        render_floor(canvas, &self.camera.sqr(), self.camera.viewport,
            |p| {
                let fid = FrameId::new_generic(EntityKind::SqrTile,
                    self.sqr_tiles[elevation as usize].as_ref().unwrap().get(p.x as usize, p.y as usize).unwrap().0).unwrap();
                let frms = self.frm_db.get(fid).unwrap();
                Some(frms.frame_lists[Direction::NE].frames[0].texture.clone())
            },
            |point| {
                let l = self.light_grid().get_clipped(EPoint { elevation, point });
                cmp::max(l, self.ambient_light)
            }
        );

        self.objects().render(canvas, elevation, self.camera.viewport, &self.camera.hex(),
            self.egg().as_ref(),
            |pos| if let Some(pos) = pos {
                cmp::max(self.light_grid().get_clipped(pos), self.ambient_light)
            } else {
                self.ambient_light
            });

        if draw_roof {
            render_roof(canvas, &self.camera.sqr(), self.camera.viewport,
                |p| {
                    let id = self.sqr_tiles[elevation as usize].as_ref().unwrap()
                        .get(p.x as usize, p.y as usize).unwrap().1;
                    let fid = FrameId::new_generic(EntityKind::SqrTile, id).unwrap();
                    Some(self.frm_db.get(fid).unwrap().first().texture.clone())
                });
        }

        self.objects().render_outlines(canvas, elevation, self.camera.viewport, &self.camera.hex());

        self.render_floating_texts(canvas);
    }

    pub fn scroll(&mut self, dir: ScrollDirection, amount: u32) -> u32 {
        if amount == 0 {
            return 0;
        }

        let mut scrolled = 0;
        let mut pos = self.camera.hex().from_screen(self.camera.viewport.center());
        // Original doesn't use tile centers when measuring screen distance between dude and camera.
        let dude_pos_scr = hex::to_screen(
            self.objects.get(self.dude_obj.unwrap()).pos.unwrap().point) + hex::TILE_CENTER;
        let elevation = self.elevation();
        while scrolled < amount {
            let new_pos = dir.go(pos);
            if !self.hex_grid.is_in_bounds(new_pos) {
                break;
            }

            let new_pos_scr = hex::to_screen(new_pos) + hex::TILE_CENTER;
            let distance = dude_pos_scr - new_pos_scr;
            if distance.x.abs() >= 480 || distance.y.abs() >= 400 { // TODO make configurable
                break;
            }

            let blocker = self.objects.at(new_pos.elevated(elevation))
                .iter()
                .any(|&h| self.objects.get(h)
                    .proto_id() == Some(ProtoId::SCROLL_BLOCKER));
            if blocker {
                break;
            }

            pos = new_pos;
            scrolled += 1;
        }

        self.camera.look_at(pos);

        scrolled
    }

    fn update_light_grid(objects: &Objects, light_grid: &mut LightGrid, h: object::Handle,
            factor: i32) {
        let obj = objects.get(h);
        if let Some(pos) = obj.pos {
            light_grid.update(pos,
                obj.light_emitter.radius,
                factor * obj.light_emitter.intensity as i32,
                |lt| objects.light_test(lt));
        }
    }

    fn egg(&self) -> Option<Egg> {
        if let Some(dude_obj) = self.dude_obj {
            Some(Egg {
                pos: self.objects().get(dude_obj).pos.unwrap().point,
                fid: FrameId::EGG,
            })
        } else {
            None
        }
    }

    fn render_floating_texts(&self, canvas: &mut dyn Canvas) {
        for floating_text in &self.floating_texts {
            let screen_pos = if let Some(obj) = floating_text.obj {
                let pos = if let Some(pos) = self.objects.get(obj).pos {
                    pos
                } else {
                    debug!("not showing floating text for {:?} because it is not on the hex grid",
                        floating_text.obj);
                    continue;
                };
                if pos.elevation != self.elevation() {
                    return;
                }
                self.camera.hex().center_to_screen(pos.point) - Point::new(0, 60)
            } else {
                self.camera.viewport.center()
            };
            floating_text.render(screen_pos, self.camera.viewport, canvas);
        }
    }

    fn expire_floating_texts(&mut self) {
        let update_time = self.update_time;
        self.floating_texts.retain(|ft| {
            let expires_at = ft.expires_at(Duration::from_millis(3_500),
                Duration::from_millis(1_400));
            expires_at > update_time
        })
    }
}




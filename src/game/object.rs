use enumflags::BitFlags;
use enumflags_derive::EnumFlags;
use enum_primitive_derive::Primitive;
use if_chain::if_chain;
use slotmap::{SecondaryMap, SlotMap};
use std::cell::{Ref, RefCell};
use std::cmp;
use std::mem;
use std::rc::Rc;

use crate::asset::{CritterAnim, EntityKind, Flag, FlagExt, WeaponKind};
use crate::asset::frame::{FrameId, FrameDb};
use crate::asset::proto::{self, CritterKillKind, ProtoId, ProtoDb};
use crate::game::script::Sid;
use crate::graphics::{EPoint, Point, Rect};
use crate::graphics::geometry::hex::*;
use crate::graphics::lighting::light_grid::{LightTest, LightTestResult};
use crate::graphics::render::Canvas;
use crate::graphics::sprite::*;
use crate::sequence::cancellable::Cancel;
use crate::util::{self, EnumExt, SmKey};
use crate::util::array2d::Array2d;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Outline {
    pub style: OutlineStyle,
    pub translucent: bool,
    pub disabled: bool,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    pub capacity: usize,
    pub items: Vec<InventoryItem>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            capacity: 0,
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InventoryItem {
    pub object: Handle,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LightEmitter {
    pub intensity: u32,
    pub radius: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub pos: Point,
    pub fid: FrameId,
}

impl Egg {
    #[must_use]
    pub fn hit_test(&self, p: Point, tile_grid: &TileGrid, frm_db: &FrameDb) -> bool {
        let screen_pos = tile_grid.to_screen(self.pos) + Point::new(16, 8);
        let frms = frm_db.get(self.fid);
        let frml = &frms.frame_lists[Direction::NE];
        let frm = &frml.frames[0];

        let bounds = frm.bounds_centered(screen_pos, frml.center);
        if !bounds.contains(p.x, p.y) {
            return false;
        }
        let p = p - bounds.top_left();
        frm.mask.test(p)
    }
}

#[derive(Clone, Debug)]
pub struct Hit {
    /// Hit a translucent object.
    pub translucent: bool,

    /// Hit a `Wall` or `Scenery` object at point which is masked by the Egg.
    pub with_egg: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Handle(SmKey);

impl Handle {
    #[cfg(test)]
    pub fn null() -> Self {
        use slotmap::Key;
        Handle(Key::null())
    }
}

#[derive(Debug)]
pub struct Object {
    pub flags: BitFlags<Flag>,
    pub pos: Option<EPoint>,
    pub screen_pos: Point,
    pub screen_shift: Point,
    pub fid: FrameId,
    pub frame_idx: usize,
    pub direction: Direction,
    pub light_emitter: LightEmitter,
    pub pid: Option<ProtoId>,
    pub inventory: Inventory,
    pub outline: Option<Outline>,
    pub sequence: Option<Cancel>,
    pub sid: Option<Sid>,
    pub sub: Option<SubObject>,
}

impl Object {
    pub fn new(fid: FrameId, pid: Option<ProtoId>, pos: Option<EPoint>) -> Self {
        Self {
            pos,
            screen_pos: Point::new(0, 0),
            screen_shift: Point::new(0, 0),
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            flags: BitFlags::empty(),
            pid,
            inventory: Inventory::new(),
            light_emitter: LightEmitter {
                intensity: 0,
                radius: 0,
            },
            outline: None,
            sequence: None,
            sid: None,
            sub: match fid.kind() {
                EntityKind::Critter => Some(SubObject::Critter(Default::default())),
                _ => None,
            }
        }
    }

    pub fn kind(&self) -> EntityKind {
        self.fid.kind()
    }

    pub fn has_running_sequence(&self) -> bool {
        self.sequence.as_ref().map(|seq| seq.is_running()).unwrap_or(false)
    }

    pub fn render(&mut self, canvas: &mut Canvas, light: u32,
            frm_db: &FrameDb, proto_db: &ProtoDb, tile_grid: &TileGrid,
            egg: Option<&Egg>) {
        if self.flags.contains(Flag::TurnedOff) {
            return;
        }

        let light = if self.fid.kind() == EntityKind::Interface {
            0x10000
        } else {
            light
        };

        let effect = self.get_effect(proto_db, tile_grid, egg);
        let sprite = self.create_sprite(light, effect, tile_grid);

        self.screen_pos = sprite.render(canvas, frm_db).top_left();
    }

    pub fn render_outline(&self, canvas: &mut Canvas, frm_db: &FrameDb, tile_grid: &TileGrid) {
        if self.flags.contains(Flag::TurnedOff) {
            return;
        }
        if let Some(outline) = self.outline {
            if outline.disabled {
                return;
            }
            let effect = Effect::Outline {
                style: outline.style,
                translucent: outline.translucent,
            };
            let sprite = self.create_sprite(0x10000, Some(effect), tile_grid);
            sprite.render(canvas, frm_db);
        }
    }

    // obj_bound()
    pub fn bounds(&self, frm_db: &FrameDb, tile_grid: &TileGrid) -> Rect {
        let (frame_list, frame) = self.frame_list(frm_db);
        self.bounds0(frame_list.center, frame.size(), tile_grid)
    }

    // critter_is_dead()
    pub fn is_critter_dead(&self) -> bool {
        // FIXME
        false
    }

    // obj_intersects_with
    #[must_use]
    pub fn hit_test(&self, p: Point, frm_db: &FrameDb, tile_grid: &TileGrid) -> Option<Hit> {
        if self.flags.contains(Flag::TurnedOff) {
            return None;
        }

        let bounds = self.bounds(frm_db, tile_grid);
        if !bounds.contains(p.x, p.y) {
            return None;
        }

        let p = p - bounds.top_left();
        if !self.frame(frm_db).mask.test(p) {
            return None;
        }

        let translucent = self.has_trans() && !self.flags.contains(Flag::TransNone);
        Some(Hit {
            translucent,
            with_egg: false,
        })
    }

    fn bounds0(&self, frame_center: Point, frame_size: Point, tile_grid: &TileGrid) -> Rect {
        let mut r = if let Some(pos) = self.pos {
            let top_left =
                tile_grid.to_screen(pos.point)
                + Point::new(16, 8)
                + frame_center
                + self.screen_shift
                - Point::new(frame_size.x / 2, frame_size.y - 1);
            let bottom_right = top_left + frame_size;
            Rect::with_points(top_left, bottom_right)
        } else {
            Rect::with_points(self.screen_pos, self.screen_pos + frame_size)
        };

        let has_outline = self.outline.map(|o| !o.disabled).unwrap_or(false);
        if has_outline {
            // Include 1-pixel outline.
            r.left -= 1;
            r.top -= 1;
            r.right += 1;
            r.bottom += 1;
        }

        r
    }

    fn create_sprite(&self, light: u32, effect: Option<Effect>, tile_grid: &TileGrid) -> Sprite {
        let (pos, centered) = if let Some(EPoint { point: hex_pos, .. }) = self.pos {
            (tile_grid.to_screen(hex_pos) + self.screen_shift + Point::new(16, 8), true)
        } else {
            (self.screen_pos, false)
        };
        Sprite {
            pos,
            centered,
            fid: self.fid,
            frame_idx: self.frame_idx,
            direction: self.direction,
            light,
            effect,
        }
    }

    fn get_effect(&self, proto_db: &ProtoDb, tile_grid: &TileGrid, egg: Option<&Egg>)
            -> Option<Effect> {
        let kind = self.fid.kind();

        if kind == EntityKind::Interface {
            return None;
        }

        let with_egg =
            egg.is_some()
            // Doesn't have any translucency flags.
            && !self.has_trans()
            // Scenery or wall with position and proto.
            && (kind == EntityKind::Scenery || kind == EntityKind::Wall)
                && self.pos.is_some() && self.pid.is_some();

        if !with_egg {
            return self.get_trans_effect();
        }

        let egg = egg.unwrap();

        let pos = self.pos.unwrap().point;
        let proto_flags_ext = proto_db.proto(self.pid.unwrap()).unwrap().flags_ext;

        let with_egg = if proto_flags_ext.intersects(
                FlagExt::WallEastOrWest | FlagExt::WallWestCorner) {
            tile_grid.is_in_front_of(pos, egg.pos)
                && (!tile_grid.is_to_right_of(egg.pos, pos)
                    || !self.flags.contains(Flag::WallTransEnd))
        } else if proto_flags_ext.contains(FlagExt::WallNorthCorner) {
            tile_grid.is_in_front_of(pos, egg.pos)
                || tile_grid.is_to_right_of(pos, egg.pos)
        } else if proto_flags_ext.contains(FlagExt::WallSouthCorner) {
            tile_grid.is_in_front_of(pos, egg.pos)
                && tile_grid.is_to_right_of(pos, egg.pos)
        } else if tile_grid.is_to_right_of(pos, egg.pos) {
            !tile_grid.is_in_front_of(egg.pos, pos)
                && !self.flags.contains(Flag::WallTransEnd)
        } else {
            false
        };

        if with_egg {
            let mask_pos = tile_grid.to_screen(egg.pos) + Point::new(16, 8)/*+ self.screen_shift ??? */;
            Some(Effect::Masked { mask_fid: egg.fid, mask_pos })
        } else {
            self.get_trans_effect()
        }
    }

    fn get_trans_effect(&self) -> Option<Effect> {
        match () {
            _ if self.flags.contains(Flag::TransEnergy) => Some(Translucency::Energy),
            _ if self.flags.contains(Flag::TransGlass) => Some(Translucency::Glass),
            _ if self.flags.contains(Flag::TransRed) => Some(Translucency::Red),
            _ if self.flags.contains(Flag::TransSteam) => Some(Translucency::Steam),
            _ if self.flags.contains(Flag::TransWall) => Some(Translucency::Wall),
            _ => None,
        }.map(Effect::Translucency)
    }

    fn frame_list<'a>(&self, frm_db: &'a FrameDb) -> (Ref<'a, FrameList>, Ref<'a, Frame>) {
        let direction = self.direction;
        let frame_idx = self.frame_idx;
        let frms = frm_db.get(self.fid);
        Ref::map_split(frms, |frms| {
            let frml = &frms.frame_lists[direction];
            (frml, &frml.frames[frame_idx])
        })
    }

    fn frame<'a>(&self, frm_db: &'a FrameDb) -> Ref<'a, Frame> {
        self.frame_list(frm_db).1
    }

    fn has_trans(&self) -> bool {
        self.flags.intersects(
            Flag::TransEnergy | Flag::TransGlass | Flag::TransRed | Flag::TransSteam |
            Flag::TransWall | Flag::TransNone)
    }
}

pub struct Objects {
    tile_grid: TileGrid,
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrameDb>,
    handles: SlotMap<SmKey, ()>,
    objects: SecondaryMap<SmKey, RefCell<Object>>,
    // Objects attached to tile (Object::pos is Some).
    by_pos: Box<[Array2d<Vec<Handle>>]>,
    // Objects not attached to tile (Object::pos is None).
    detached: Vec<Handle>,
    empty_object_handle_vec: Vec<Handle>,
    path_finder: RefCell<PathFinder>,
}

impl Objects {
    pub fn new(tile_grid: TileGrid, elevation_count: u32, proto_db: Rc<ProtoDb>,
            frm_db: Rc<FrameDb>) -> Self {
        let path_finder = RefCell::new(PathFinder::new(tile_grid.clone(), 5000));
        let by_pos = util::vec_with_func(elevation_count as usize,
            |_| Array2d::with_default(tile_grid.width() as usize, tile_grid.height() as usize))
            .into_boxed_slice();
        Self {
            tile_grid,
            proto_db,
            frm_db,
            handles: SlotMap::with_key(),
            objects: SecondaryMap::new(),
            by_pos,
            detached: Vec::new(),
            empty_object_handle_vec: Vec::new(),
            path_finder,
        }
    }

    pub fn contains(&self, obj: Handle) -> bool {
        self.objects.contains_key(obj.0)
    }

    pub fn insert(&mut self, obj: Object) -> Handle {
        let pos = obj.pos;

        let k = self.handles.insert(());
        let h = Handle(k);
        self.objects.insert(k, RefCell::new(obj));

        self.insert_into_tile_grid(h, pos, true);

        h
    }

    pub fn at(&self, pos: EPoint) -> &Vec<Handle> {
        self.by_pos[pos.elevation as usize]
            .get(pos.point.x as usize, pos.point.y as usize)
            .unwrap()
    }

    pub fn get(&self, h: Handle) -> &RefCell<Object> {
        &self.objects[h.0]
    }

    pub fn light_test(&self, light_test: LightTest) -> LightTestResult {
        let mut update = true;

        let dir = light_test.direction;

        for &objh in self.at(light_test.point) {
            let obj = self.get(objh).borrow();
            if obj.flags.contains(Flag::TurnedOff) {
                continue;
            }
            let block = !obj.flags.contains(Flag::LightThru);

            if obj.fid.kind() == EntityKind::Wall {
                if !obj.flags.contains(Flag::Flat) {
                    let flags_ext = self.proto_db.proto(obj.pid.unwrap()).unwrap().flags_ext;
                    if flags_ext.contains(FlagExt::WallEastOrWest) ||
                            flags_ext.contains(FlagExt::WallEastCorner) {
                        if dir != Direction::W
                                && dir != Direction::NW
                                && (dir != Direction::NE || light_test.i >= 8)
                                && (dir != Direction::SW || light_test.i <= 15) {
                            update = false;
                        }
                    } else if flags_ext.contains(FlagExt::WallNorthCorner) {
                        if dir != Direction::NE && dir != Direction::NW {
                            update = false;
                        }
                    } else if flags_ext.contains(FlagExt::WallSouthCorner) {
                        if dir != Direction::NE
                                && dir != Direction::E
                                && dir != Direction::W
                                && dir != Direction::NW
                                && (dir != Direction::SW || light_test.i <= 15) {
                            update = false;
                        }
                    } else if dir != Direction::NE
                            && dir != Direction::E
                            && (dir != Direction::NW || light_test.i <= 7) {
                        update = false;
                    }
                }
            } else if block && dir >= Direction::E && dir <= Direction::SW {
                update = false;
            }

            if block {
                return LightTestResult {
                    block,
                    update,
                }
            }
        }

        LightTestResult {
            block: false,
            update,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, elevation: u32, screen_rect: &Rect,
            tile_grid: &TileGrid, egg: Option<&Egg>,
            get_light: impl Fn(Option<EPoint>) -> u32) {
        let ref get_light = get_light;
        self.render0(canvas, elevation, screen_rect, tile_grid, egg, get_light, true);
        self.render0(canvas, elevation, screen_rect, tile_grid, egg, get_light, false);
    }

    pub fn render_outlines(&self, canvas: &mut Canvas, elevation: u32, screen_rect: &Rect,
            tile_grid: &TileGrid) {
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in hex_rect.top..hex_rect.bottom {
            for x in (hex_rect.left..hex_rect.right).rev() {
                let pos = EPoint {
                    elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos) {
                    let obj = self.get(objh).borrow_mut();
                    obj.render_outline(canvas, &self.frm_db, tile_grid);
                }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=Handle> + '_ {
        // FIXME this should come from by_pos.
        self.handles.keys().map(|k| Handle(k))
    }

    pub fn set_pos(&mut self, h: Handle, pos: impl Into<EPoint>) {
        self.remove_from_tile_grid(h);
        self.insert_into_tile_grid(h, Some(pos.into()), true);
    }

    pub fn set_screen_shift(&mut self, h: Handle, shift: impl Into<Point>) {
        let pos = self.remove_from_tile_grid(h);
        self.get(h).borrow_mut().screen_shift = shift.into();
        self.insert_into_tile_grid(h, pos, false);
    }

    pub fn add_screen_shift(&mut self, h: Handle, shift: impl Into<Point>) -> Point {
        let pos = self.remove_from_tile_grid(h);
        let new_shift = {
            let mut obj = self.get(h).borrow_mut();
            obj.screen_shift += shift.into();
            obj.screen_shift
        };
        self.insert_into_tile_grid(h, pos, false);
        new_shift
    }

    pub fn reset_screen_shift(&mut self, h: Handle) {
        let pos = self.remove_from_tile_grid(h);
        self.insert_into_tile_grid(h, pos, true);
    }

    // dude_stand()
    pub fn make_standing(&mut self, h: Handle, frm_db: &FrameDb) {
        let shift = {
            let mut obj = self.get(h).borrow_mut();
            let mut shift = Point::new(0, 0);
            let fid = if let FrameId::Critter(critter_fid) = obj.fid {
                if critter_fid.weapon() != WeaponKind::Unarmed {
                    let fid = critter_fid
                        .with_direction(Some(obj.direction))
                        .with_anim(CritterAnim::TakeOut)
                        .into();
                    let frame_set = frm_db.get(fid);
                    for frame in &frame_set.frame_lists[obj.direction].frames {
                        shift += frame.shift;
                    }

                    let fid = critter_fid
                        .with_direction(Some(obj.direction))
                        .with_anim(CritterAnim::Stand)
                        .with_weapon(WeaponKind::Unarmed)
                        .into();
                    shift += frm_db.get(fid).frame_lists[obj.direction].center;
                }
                let anim = if critter_fid.anim() == CritterAnim::FireDance {
                    CritterAnim::FireDance
                } else {
                    CritterAnim::Stand
                };
                critter_fid
                    .with_direction(Some(obj.direction))
                    .with_anim(anim)
                    .into()
            } else {
                obj.fid
            };
            obj.fid = fid;
            obj.frame_idx = 0;
            shift
        };
        self.set_screen_shift(h, shift);
    }

    // obj_blocking_at()
    pub fn blocker_at(&self, p: impl Into<EPoint>, mut filter: impl FnMut(Handle, &Object) -> bool)
            -> Option<&RefCell<Object>> {
        let p = p.into();
        let mut check = |h| {
            let obj = self.get(h);
            let o = obj.borrow();
            match o.fid.kind() {
                | EntityKind::Critter
                | EntityKind::Scenery
                | EntityKind::Wall
                => {},
                _ => return None,
            }
            if o.flags.contains(Flag::TurnedOff) || o.flags.contains(Flag::NoBlock) {
                return None;
            }
            if !filter(h, &*o) {
                return None;
            }
            Some(obj)
        };
        for &objh in self.at(p.into()) {
            let r = check(objh);
            if r.is_some() {
                return r;
            }
        }
        for dir in Direction::iter() {
            if let Some(near) = self.tile_grid.go(p.point, dir, 1) {
                for &objh in self.at(near.elevated(p.elevation)) {
                    if self.get(objh).borrow().flags.contains(Flag::MultiHex) {
                        let r = check(objh);
                        if r.is_some() {
                            return r;
                        }
                    }
                }
            }
        }

        None
    }

    pub fn blocker_for_object_at(&self, obj: Handle, p: impl Into<EPoint>)
            -> Option<&RefCell<Object>> {
        self.blocker_at(p, |h, _| h != obj)
    }

    pub fn path_for_object(&self, obj: Handle, to: impl Into<Point>, smooth: bool, proto_db: &ProtoDb)
            -> Option<Vec<Direction>> {
        let o = self.get(obj).borrow();
        let from = o.pos?;
        self.path_finder.borrow_mut().find(from.point, to, smooth,
            |p| {
                let p = EPoint::new(from.elevation, p);
                if self.blocker_for_object_at(obj, p).is_some() {
                    TileState::Blocked
                } else if let Some(pid) = o.pid {
                    let radioacive_goo = self.at(p)
                        .iter()
                        .any(|&h| self.get(h).borrow().pid
                            .map(|pid| pid.is_radioactive_goo())
                            .unwrap_or(false));
                    let cost = if radioacive_goo {
                        let gecko = if let proto::Variant::Critter(ref c) = proto_db.proto(pid).unwrap().proto {
                            c.kill_kind == CritterKillKind::Gecko
                        } else {
                            false
                        };
                        if gecko {
                            100
                        } else {
                            400
                        }
                    } else {
                        0
                    };

                    TileState::Passable(cost)
                } else {
                    TileState::Passable(0)
                }
            })
    }

    pub fn bounds(&self, obj: Handle, tile_grid: &TileGrid) -> Rect {
        self.get(obj).borrow().bounds(&self.frm_db, tile_grid)
    }

    pub fn hit_test(&self, p: EPoint, screen_rect: &Rect, tile_grid: &TileGrid,
        egg: Option<Egg>) -> Vec<(Handle, Hit)>
    {
        let mut r = Vec::new();
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in (hex_rect.top..hex_rect.bottom).rev() {
            for x in hex_rect.left..hex_rect.right {
                let pos = EPoint {
                    elevation: p.elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos).iter().rev() {
                    let obj = self.get(objh).borrow();

                    let mut hit = if let Some(hit) = obj.hit_test(p.point, &self.frm_db, tile_grid) {
                        hit
                    } else {
                        continue;
                    };

                    if let Some(egg) = egg {
                        if self.is_egg_hit(p.point, &*obj, egg, tile_grid) {
                            hit.with_egg = true;
                        }
                    }

                    r.push((objh, hit));
                }
            }
        }
        r
    }

    // obj_intersects_with()
    #[must_use]
    fn is_egg_hit(&self, p: Point, obj: &Object, egg: Egg, tile_grid: &TileGrid) -> bool {
        if_chain! {
            if let Some(obj_pos) = obj.pos;
            let obj_pos = obj_pos.point;
            if let Some(pid) = obj.pid;
            if pid.kind() == EntityKind::Wall || pid.kind() == EntityKind::Scenery;
            then {
                if !egg.hit_test(p, tile_grid, &self.frm_db) {
                    return false;
                }

                let proto = self.proto_db.proto(pid).unwrap();
                let masked = if proto.flags_ext.intersects(
                    FlagExt::WallEastOrWest | FlagExt::WallWestCorner)
                {
                    tile_grid.is_in_front_of(obj_pos, egg.pos)
                } else if proto.flags_ext.contains(FlagExt::WallNorthCorner) {
                    tile_grid.is_in_front_of(obj_pos, egg.pos) ||
                        tile_grid.is_to_right_of(obj_pos, egg.pos)
                } else if proto.flags_ext.contains(FlagExt::WallSouthCorner) {
                    tile_grid.is_in_front_of(obj_pos, egg.pos) &&
                        tile_grid.is_to_right_of(obj_pos, egg.pos)
                } else {
                    tile_grid.is_to_right_of(obj_pos, egg.pos)
                };
                masked
            } else {
                false
            }
        }
    }

    fn get_render_hex_rect(screen_rect: &Rect, tile_grid: &TileGrid) -> Rect {
        tile_grid.from_screen_rect(&Rect {
            left: -320,
            top: -190,
            right: screen_rect.width() + 320,
            bottom: screen_rect.height() + 190
        }, false)
    }

    fn render0(&self, canvas: &mut Canvas, elevation: u32,
            screen_rect: &Rect, tile_grid: &TileGrid, egg: Option<&Egg>,
            get_light: impl Fn(Option<EPoint>) -> u32,
            flat: bool) {
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in hex_rect.top..hex_rect.bottom {
            for x in (hex_rect.left..hex_rect.right).rev() {
                let pos = EPoint {
                    elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos) {
                    let mut obj = self.get(objh).borrow_mut();
                    if flat && !obj.flags.contains(Flag::Flat) {
                        break;
                    } else if !flat && obj.flags.contains(Flag::Flat) {
                        continue;
                    }
                    let light = get_light(obj.pos);
                    assert!(light <= 0x10000);
                    obj.render(canvas, light, &self.frm_db, &self.proto_db, tile_grid, egg);
                }
            }
        }
    }

    fn at_mut(&mut self, pos: EPoint) -> &mut Vec<Handle> {
        self.by_pos[pos.elevation as usize]
            .get_mut(pos.point.x as usize, pos.point.y as usize)
            .unwrap()
    }

    fn cmp_objs(&self, o1: &Object, o2: &Object) -> cmp::Ordering {
        assert_eq!(o1.pos.unwrap().elevation, o2.pos.unwrap().elevation);

        // By flatness, flat first.
        let flat = o1.flags.contains(Flag::Flat);
        let other_flat = o2.flags.contains(Flag::Flat);
        if flat != other_flat {
            return if flat {
                cmp::Ordering::Less
            } else {
                cmp::Ordering::Greater
            };
        }


        let shift = o1.screen_shift + o1.frame(&self.frm_db).shift;
        let other_shift = o2.screen_shift + o2.frame(&self.frm_db).shift;

        // By shift_y, less first.
        if shift.y < other_shift.y {
            return cmp::Ordering::Less;
        }
        if shift.y > other_shift.y {
            return cmp::Ordering::Greater;
        }

        // By shift_x, less first.
        shift.x.cmp(&other_shift.x)
    }

    fn insert_into_tile_grid(&mut self, h: Handle, pos: Option<EPoint>, reset_screen_shift: bool) {
        if let Some(pos) = pos {
            {
                let mut obj = self.get(h).borrow_mut();
                obj.pos = Some(pos);
                if reset_screen_shift {
                    obj.screen_shift = Point::new(0, 0);
                }
            }

            let i = {
                let list = self.at(pos);
                let obj = self.get(h).borrow();
                match list.binary_search_by(|&h| {
                    let o = self.get(h).borrow();
                    self.cmp_objs(&o, &obj)
                }) {
                    Ok(mut i) =>  {
                        // Append to the current group of equal objects.
                        while i < list.len()
                            && self.cmp_objs(&obj, &self.get(list[i]).borrow()) == cmp::Ordering::Equal
                        {
                            i += 1;
                        }
                        i
                    }
                    Err(i) => i,
                }
            };
            self.at_mut(pos).insert(i, h);
        } else {
            self.detached.push(h);
        }
    }

    fn remove_from_tile_grid(&mut self, h: Handle) -> Option<EPoint> {
        let old_pos = mem::replace(&mut self.get(h).borrow_mut().pos, None);
        let list = if let Some(old_pos) = old_pos {
            self.at_mut(old_pos)
        } else {
            &mut self.detached
        };
        // TODO maybe use binary_search for detaching.
        list.retain(|&hh| hh != h);
        old_pos
    }
}

#[derive(Debug)]
pub enum SubObject {
    Critter(Critter),
}

#[derive(Debug, Default)]
pub struct Critter {
    pub health: i32,
    pub radiation: i32,
    pub poison: i32,
    pub combat: CritterCombat,
}

#[derive(Debug)]
pub struct CritterCombat {
    pub damage_flags: BitFlags<DamageFlag>,
}

impl Default for CritterCombat {
    fn default() -> Self {
        Self {
            damage_flags: BitFlags::empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, EnumFlags, Primitive)]
#[repr(u32)]
pub enum DamageFlag {
  KnockedOut = 0x1,
  KnockedDown = 0x2,
  CripLegLeft = 0x4,
  CripLegRight = 0x8,
  CripArmLeft = 0x10,
  CripArmRight = 0x20,
  Blind = 0x40,
  Dead = 0x80,
  Hit = 0x100,
  Critical = 0x200,
  OnFire = 0x400,
  Bypass = 0x800,
  Explode = 0x1000,
  Destroy = 0x2000,
  Drop = 0x4000,
  LoseTurn = 0x8000,
  HitSelf = 0x10000,
  LoseAmmo = 0x20000,
  Dud = 0x40000,
  HurtSelf = 0x80000,
  RandomHit = 0x100000,
  CripRandom = 0x200000,
  Backwash = 0x400000,
  PerformReverse = 0x800000,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bounds() {
        let screen_shift = Point::new(10, 20);
        let base = Point::new(2384, 468) + screen_shift;

        let tg = TileGrid::default();
        let mut obj = Object::new(FrameId::BLANK, None, Some(EPoint::new(0, (55, 66))));
        obj.screen_shift = screen_shift;
        assert_eq!(obj.bounds0(Point::new(-1, 3), Point::new(29, 63), &tg),
            Rect::with_points((1, -51), (30, 12))
                .translate(base.x, base.y));
    }
}
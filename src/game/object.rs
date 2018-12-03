use asset::{EntityKind, Flag, FlagExt};
use asset::frm::{Fid, FrmDb};
use asset::proto::{Pid, ProtoDb};
use graphics::{ElevatedPoint, Point, Rect};
use graphics::frm::{Effect, Frame, Sprite, Translucency};
use graphics::geometry::Direction;
use graphics::geometry::hex::TileGrid;
use graphics::lighting::light_grid::{LightTest, LightTestResult};
use graphics::render::Render;
use util;
use util::two_dim_array::Array2d;

use enumflags::BitFlags;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::cell::{Ref, RefCell};
use std::cmp;
use std::mem;
use std::rc::Rc;

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

pub struct Egg {
    pub pos: Point,
    pub fid: Fid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Handle(DefaultKey);

#[derive(Clone, Debug)]
pub struct Object {
    pub handle: Option<Handle>,
//    pub id: u32,
    pub flags: BitFlags<Flag>,
    pub pos: Option<ElevatedPoint>,
    pub screen_pos: Point,
    pub screen_shift: Point,
    pub fid: Fid,
    pub frame_idx: usize,
    pub direction: Direction,
    pub light_emitter: LightEmitter,
    pub pid: Option<Pid>,
    pub inventory: Inventory,
//  //  int updated_flags;
////  GameObject::ItemOrCritter _;
//    int cid;

//  int outline;
//  int script_id;
//  GameObject *owner;
//  int script_idx;
}

impl Object {
    pub fn render(&mut self, render: &mut Render, rect: &Rect, light: u32,
            frm_db: &FrmDb, proto_db: &ProtoDb, tile_grid: &TileGrid,
            egg: Option<&Egg>) {
        if self.flags.contains(Flag::TurnedOff) {
            return;
        }
        let (pos, centered) = if let Some(ElevatedPoint { point: hex_pos, .. }) = self.pos {
            (tile_grid.to_screen(hex_pos) + self.screen_shift + Point::new(16, 8), true)
        } else {
            (self.screen_pos, false)
        };

        let effect = self.get_effect(proto_db, tile_grid, egg);

        let sprite = Sprite {
            pos,
            centered,
            fid: self.fid,
            frame_idx: self.frame_idx,
            direction: self.direction,
            light,
            effect,
        };
        self.screen_pos = sprite.render(render, rect, frm_db).top_left();
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
            && !self.flags.intersects(
                Flag::TransEnergy | Flag::TransGlass | Flag::TransRed | Flag::TransSteam |
                Flag::TransWall | Flag::TransNone)
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

    fn frame<'a>(&self, frm_db: &'a FrmDb) -> Ref<'a, Frame> {
        let direction = self.direction;
        let frame_idx = self.frame_idx;
        let frms = frm_db.get(self.fid);
        Ref::map(frms, |frms| {
            let frml = &frms.frame_lists[direction];
            &frml.frames[frame_idx]
        })
    }
}

pub struct Objects {
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrmDb>,
    handles: SlotMap<DefaultKey, ()>,
    objects: SecondaryMap<DefaultKey, RefCell<Object>>,
    by_pos: Box<[Array2d<Vec<Handle>>]>,
    detached: Vec<Handle>,
    empty_object_handle_vec: Vec<Handle>,
}

impl Objects {
    pub fn insert(&mut self, mut obj: Object) -> Handle {
        assert!(obj.handle.is_none());

        let pos = obj.pos;

        let k = self.handles.insert(());
        let h = Handle(k);
        obj.handle = Some(h.clone());
        self.objects.insert(k, RefCell::new(obj));

        self.attach(&h, pos);

        h
    }

    pub fn at(&self, pos: ElevatedPoint) -> &Vec<Handle> {
        self.by_pos[pos.elevation]
            .get(pos.point.x as usize, pos.point.y as usize)
            .unwrap()
    }

    pub fn get(&self, h: &Handle) -> &RefCell<Object> {
        &self.objects[h.0]
    }

    pub fn light_test(&self, light_test: LightTest) -> LightTestResult {
        let mut update = true;

        let dir = light_test.direction;

        for objh in self.at(light_test.point) {
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

    pub fn render(&self, render: &mut Render, elevation: usize, screen_rect: &Rect,
            tile_grid: &TileGrid, egg: Option<&Egg>,
            get_light: impl Fn(Option<ElevatedPoint>) -> u32) {
        let ref get_light = get_light;
        self.render0(render, elevation, screen_rect, tile_grid, egg, get_light, true);
        self.render0(render, elevation, screen_rect, tile_grid, egg, get_light, false);
    }

    fn render0(&self, render: &mut Render, elevation: usize,
            screen_rect: &Rect, tile_grid: &TileGrid, egg: Option<&Egg>,
            get_light: impl Fn(Option<ElevatedPoint>) -> u32,
            flat: bool) {
        let hex_rect = tile_grid.from_screen_rect(&Rect {
            left: -320,
            top: -190,
            right: screen_rect.width() + 320,
            bottom: screen_rect.height() + 190
        }, false);
        for y in hex_rect.top..hex_rect.bottom {
            for x in (hex_rect.left..hex_rect.right).rev() {
                let pos = ElevatedPoint {
                    elevation,
                    point: Point::new(x, y),
                };
                for objh in self.at(pos) {
                    let mut obj = self.get(objh).borrow_mut();
                    if flat && !obj.flags.contains(Flag::Flat) {
                        break;
                    } else if !flat && obj.flags.contains(Flag::Flat) {
                        continue;
                    }
                    let light = get_light(obj.pos);
                    assert!(light <= 0x10000);
                    obj.render(render, &screen_rect, light, &self.frm_db, &self.proto_db, tile_grid,
                        egg);
                }
            }
        }
    }

    fn at_mut(&mut self, pos: ElevatedPoint) -> &mut Vec<Handle> {
        self.by_pos[pos.elevation]
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

    fn attach(&mut self, h: &Handle, pos: Option<ElevatedPoint>) {
        if let Some(pos) = pos {
            let pos = pos.into();
            self.get(&h).borrow_mut().pos = Some(pos);

            let i = {
                let list = self.at(pos);
                let obj = self.get(&h).borrow();
                match list.binary_search_by(|h| {
                    let o = self.get(h).borrow();
                    self.cmp_objs(&obj, &o)
                }) {
                    Ok(i) => i,
                    Err(i) => i,
                }
            };
            self.at_mut(pos).insert(i, h.clone());
        } else {
            self.detached.push(h.clone());
        }
    }

    fn detach(&mut self, h: &Handle){
        let old_pos = mem::replace(&mut self.get(h).borrow_mut().pos, None);
        let list = if let Some(old_pos) = old_pos {
            self.at_mut(old_pos)
        } else {
            &mut self.detached
        };
        // TODO maybe use binary_search for detaching.
        list.retain(|hh| hh != h);
    }
}

impl Objects {
    pub fn new(tile_grid: &TileGrid, elevation_count: usize, proto_db: Rc<ProtoDb>,
            frm_db: Rc<FrmDb>) -> Self {
        Self {
            proto_db,
            frm_db,
            handles: SlotMap::new(),
            objects: SecondaryMap::new(),
            by_pos: util::vec_with_func(elevation_count,
                |_| Array2d::with_default(tile_grid.width() as usize, tile_grid.height() as usize))
                .into_boxed_slice(),
            detached: Vec::new(),
            empty_object_handle_vec: Vec::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=Handle> + '_ {
        // FIXME this should come from by_pos.
        self.handles.keys().map(|k| Handle(k))
    }

    pub fn set_pos(&mut self, h: &Handle, pos: impl Into<ElevatedPoint>) {
        self.detach(h);
        self.attach(h, Some(pos.into()));
    }
}
pub mod db;

use byteorder::{BigEndian, ReadBytesExt};
use enumflags2::{bitflags, BitFlags};
use log::*;
use measure_time::*;
use num_traits::FromPrimitive;
use std::cmp;
use std::convert::{TryFrom, TryInto};
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::asset::*;
use crate::asset::frame::{FrameId, FrameDb};
use crate::asset::proto::{MapExit, ProtoId, ProtoDb, SubItem, TargetMap};
use crate::asset::script::ProgramId;
use crate::game::object::{self, *};
use crate::game::script::*;
use crate::graphics::{EPoint, Point};
use crate::graphics::geometry::hex::{Direction, TileGrid};
use crate::graphics::sprite::OutlineStyle;
use crate::util::EnumExt;
use crate::util::array2d::Array2d;

pub const ELEVATION_COUNT: u32 = 3;

struct ScriptInfo {
    sid: ScriptIid,
    program_id: ProgramId,
    local_var_count: usize,
    local_var_offset: usize,
}

#[bitflags]
#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
#[repr(u32)]
pub enum OutlineFlag {
    GlowingRed      = 0x1,
    Red             = 0x2,
    Gray            = 0x4,
    GlowingGreen    = 0x8,
    Yellow          = 0x10,
    Brown           = 0x20,
    Disabled        = 0x80,
    Translucent     = 0x40000000,
}

pub type SqrTiles = Vec<Option<Array2d<(u16, u16)>>>;

/// Unique map ID as defined in `maps.txt`.
pub type MapId = u32;

pub struct Map {
    pub id: MapId,
    pub savegame: bool,
    pub entrance: EPoint,
    pub entrance_direction: Direction,
    pub sqr_tiles: SqrTiles,
    pub map_vars: Box<[i32]>,
}

pub struct MapReader<'a, R: 'a> {
    pub reader: &'a mut R,
    pub objects: &'a mut Objects,
    pub proto_db: &'a ProtoDb,
    pub frm_db: &'a FrameDb,
    pub scripts: &'a mut Scripts,
}

impl<'a, R: 'a + Read> MapReader<'a, R> {
    pub fn read(&mut self) -> io::Result<Map> {
        debug_time!("MapReader::read()");
        // header

        let version = self.reader.read_u32::<BigEndian>()?;

        let mut name = [0; 16];
        self.reader.read_exact(&mut name[..]).unwrap();

        let entrance_pos_lin = self.reader.read_i32::<BigEndian>()?;
        let entrance_pos = TileGrid::default().linear_to_rect_inv(entrance_pos_lin as u32);
        debug!("entrance_pos={} ({:?})", entrance_pos_lin, entrance_pos);
        let entrance_elevation = self.reader.read_u32::<BigEndian>()?;
        assert!(entrance_elevation <= ELEVATION_COUNT);
        let entrance_direction = Direction::from_u32(self.reader.read_u32::<BigEndian>()?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid entrance direction"))?;
        let local_var_count = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;

        let program_id = self.read_program_id(0)?;
        debug!("map program_id: {:?}", program_id);

        let flags = self.reader.read_u32::<BigEndian>()?;
        debug!("flags: {:04b}", flags);
        let savegame = flags & 0x1 != 0;

        let _ = self.reader.read_i32::<BigEndian>()?;
        let map_var_count = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let id = self.reader.read_i32::<BigEndian>()?.try_into().unwrap();
        let _time = self.reader.read_u32::<BigEndian>()?;

        self.reader.read_exact(&mut [0; 44 * 4][..])?;

        // map global vars

        let mut map_vars = Vec::with_capacity(map_var_count);
        for _ in 0..map_var_count {
            map_vars.push(self.reader.read_i32::<BigEndian>()?);
        }

        // map local vars

        let mut local_vars = Vec::with_capacity(local_var_count);
        for _ in 0..local_var_count {
            local_vars.push(self.reader.read_i32::<BigEndian>()?);
        }

        let sqr_tiles = self.read_sqr_tiles(flags)?;
        self.read_scripts(&local_vars, savegame)?;

        if let Some(program_id) = program_id {
            self.make_map_script(program_id)?;
        }

        self.read_objects(version)?;

        Ok(Map {
            id,
            savegame,
            entrance: EPoint {
                elevation: entrance_elevation,
                point: entrance_pos,
            },
            entrance_direction,
            sqr_tiles,
            map_vars: map_vars.into(),
        })
    }

    fn read_scripts(&mut self, local_vars: &[i32], savegame: bool) -> io::Result<()> {
        for script_kind in ScriptKind::iter() {
            debug!("reading {:?} scripts", script_kind);
            let script_count = self.reader.read_i32::<BigEndian>()?;
            debug!("script_count: {}", script_count);
            if script_count > 0 {
                let script_count = script_count as usize;
                const NODE_LEN: usize = 16;
                let node_count = script_count / NODE_LEN + (script_count % NODE_LEN != 0) as usize;
                debug!("node_count: {}", node_count);
                let mut scripts = Vec::new();
                for _ in 0..node_count {
                    scripts.clear();
                    for _ in 0..NODE_LEN {
                        if let Some(script) = self.read_script()? {
                            scripts.push(script);
                        }
                    }

                    let node_script_count = self.reader.read_i32::<BigEndian>()?;
                    debug!("node_script_count: {}", node_script_count);
                    let _ = self.reader.read_i32::<BigEndian>()?;

                    scripts.truncate(node_script_count as usize);

                    for script in &scripts {
                        let local_vars = if savegame && script.local_var_count > 0 {
                            let end = script.local_var_offset + script.local_var_count;
                            Some(local_vars[script.local_var_offset..end].into())
                        } else {
                            None
                        };
                        self.scripts.instantiate(script.sid, script.program_id, local_vars)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn read_script(&mut self) -> io::Result<Option<ScriptInfo>> {
        // Maps contain garbage in unused slots but the exact size of the data to skip depends
        // on the script kinds.

        let sid = ScriptIid::read(self.reader);
        let sid = match sid {
            Ok(sid) => sid,
            Err(ref e) if e.kind() == ErrorKind::InvalidData => {
                self.reader.read_exact(&mut [0; 15 * 4][..])?;
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        trace!("sid: {:?}", sid);

        let _ = self.reader.read_i32::<BigEndian>()?;

        match sid.kind() {
            ScriptKind::Spatial => {
                let _elevation_and_tile = self.reader.read_i32::<BigEndian>()?;
                let _spatial_radius = self.reader.read_i32::<BigEndian>()?;
            }
            ScriptKind::Time => {
                let _elevation_and_tile = self.reader.read_i32::<BigEndian>()?;
            }
            _ => {}
        }

        let _flags = self.reader.read_i32::<BigEndian>()?;

        let program_id = self.read_program_id(1)?;
        trace!("program_id: {:?}", program_id);

        let _ = self.reader.read_i32::<BigEndian>()?;
        let self_obj_id = self.reader.read_i32::<BigEndian>()?;
        trace!("self_obj_id: {}", self_obj_id);
        let local_var_offset = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let local_var_count = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let _return_value = self.reader.read_i32::<BigEndian>()?;
        let _action = self.reader.read_i32::<BigEndian>()?;
        let _ext_param = self.reader.read_i32::<BigEndian>()?;
        let _action_num = self.reader.read_i32::<BigEndian>()?;
        let _script_overrides = self.reader.read_i32::<BigEndian>()?;
        let _unk1 = self.reader.read_i32::<BigEndian>()?;
        let _how_much = self.reader.read_i32::<BigEndian>()?;
        let _unk2 = self.reader.read_i32::<BigEndian>()?;

        if let Some(program_id) = program_id {
            Ok(Some(ScriptInfo {
                sid,
                program_id,
                local_var_count,
                local_var_offset,
            }))
        } else {
            Ok(None)
        }
    }

    fn read_objects(&mut self, version: u32) -> io::Result<()> {
        let total_obj_count = self.reader.read_i32::<BigEndian>()?;
        debug!("object count: {}", total_obj_count);
        for elev in 0..ELEVATION_COUNT {
            let obj_count = self.reader.read_u32::<BigEndian>()?;
            debug!("object count at elevation {}: {}", elev, obj_count);

            for _ in 0..obj_count {
                let obj = self.read_object(version != 19)?;
                let script = obj.script;
                let objh = self.objects.insert(obj);
                if let Some((sid, _)) = script {
                    self.scripts.attach_to_object(sid, objh);
                }
            }
        }
        Ok(())
    }

    fn read_object(&mut self, f2: bool) -> io::Result<Object> {
        let id = self.reader.read_u32::<BigEndian>()?;

        trace!("object ID {}", id);
        let pos_lin = self.reader.read_i32::<BigEndian>()?;
        let pos = u32::try_from(pos_lin).ok()
            .map(|v| TileGrid::default().linear_to_rect_inv(v));
        trace!("hex_pos={} {:?}", pos_lin, pos);

        let screen_shift = Point::new(
            self.reader.read_i32::<BigEndian>()?,
            self.reader.read_i32::<BigEndian>()?);
        let screen_pos = Point::new(
            self.reader.read_i32::<BigEndian>()?,
            self.reader.read_i32::<BigEndian>()?);
        let frame_idx = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let direction = self.reader.read_u32::<BigEndian>()?;
        let direction = Direction::from_u32(direction)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid object direction: {}", direction)))?;
        let fid = FrameId::read(self.reader)?;
        trace!("{:?}", fid);

        self.frm_db.get(fid)?;

        let flags = self.reader.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(flags)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("unknown object flags: {:x}", flags)))?;

        let elevation = self.reader.read_u32::<BigEndian>()?;
        let pid = ProtoId::read(self.reader)?;
        let proto = self.proto_db.proto(pid)?;
        trace!("{:?} {:?}", pid, proto.borrow().name());
        let _cid = self.reader.read_u32::<BigEndian>()?;
        let light_emitter = LightEmitter {
            radius: self.reader.read_i32::<BigEndian>()? as u32,
            intensity: self.reader.read_i32::<BigEndian>()? as u32,
        };
        let outline = self.read_outline()?;
        trace!("outline: {:?}", outline);

        let script = self.read_obj_script()?;

        // proto update data

        let inventory_len = usize::try_from(self.reader.read_u32::<BigEndian>()?).unwrap();
        let _inventory_capacity = self.reader.read_i32::<BigEndian>()? as usize;
        let _ = self.reader.read_u32::<BigEndian>()?;

        let updated_flags = self.reader.read_u32::<BigEndian>()?;
        let updated_flags = BitFlags::from_bits(updated_flags)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                    format!("unknown updated flags: {:x}", updated_flags)))?;
        trace!("updated_flags: {:?}", updated_flags);

        let sub = if pid.kind() == EntityKind::Critter {
            // combat data
            let _damage_last_turn = self.reader.read_u32::<BigEndian>()?;
            let _combat_state = self.reader.read_u32::<BigEndian>()?;
            let _action_points = self.reader.read_u32::<BigEndian>()?;

            let damage_flags = self.reader.read_u32::<BigEndian>()?;
            let damage_flags = BitFlags::from_bits(damage_flags)
                .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                    format!("unknown damage flags: {:x}", damage_flags)))?;

            let ai_packet = self.reader.read_i32::<BigEndian>()?;
            let team_id = self.reader.read_i32::<BigEndian>()?;
            let who_hit_me = self.reader.read_i32::<BigEndian>()?;

            let health = self.reader.read_i32::<BigEndian>()?;
            let radiation = self.reader.read_i32::<BigEndian>()?;
            let poison = self.reader.read_i32::<BigEndian>()?;
            SubObject::Critter(object::Critter {
                hit_points: health,
                radiation,
                poison,
                combat: CritterCombat {
                    damage_flags,
                    ai_packet,
                    team_id,
                    who_hit_me,
                },
                dude: None,
            })
        } else {
    //            let update_flags = if updated_flags == 0xcccccccc {
    //                0
    //            } else {
    //                updated_flags
    //            };
            match pid.kind() {
                EntityKind::Item => {
                    let proto = proto.borrow();
                    match proto.sub.as_item().unwrap().sub {
                        SubItem::Weapon(ref proto) => {
                            let _ammo_count = self.reader.read_i32::<BigEndian>()?;
                            let ammo_proto_id = ProtoId::from_packed(self.reader.read_u32::<BigEndian>()?);

                            // object_fix_weapon_ammo()
                            let ammo_count = proto.max_ammo_count;
                            let ammo_proto_id = ammo_proto_id.or(proto.ammo_proto_id);
                            let ammo_proto = if let Some(pid) = ammo_proto_id {
                                Some(self.proto_db.proto(pid)?)
                            } else {
                                None
                            };
                            SubObject::Item(object::Item { ammo_count, ammo_proto })
                        }
                        SubItem::Ammo(_) => {
                            let ammo_count = self.reader.read_i32::<BigEndian>()?.try_into().unwrap();
                            SubObject::Item(object::Item { ammo_count, ammo_proto: None })
                        }
                        SubItem::Misc(ref proto) => {
                            let ammo_count = self.reader.read_i32::<BigEndian>()?;

                            // object_fix_weapon_ammo()
                            let ammo_count = ammo_count.try_into()
                                .unwrap_or(proto.max_ammo_count);
                            SubObject::Item(object::Item { ammo_count, ammo_proto: None })
                        }
                        SubItem::Key(_) => {
                            let _key_code = self.reader.read_i32::<BigEndian>()?;
                            SubObject::None
                        }
                        _ => SubObject::None
                    }
                }
                EntityKind::Scenery => {
                    let k = proto.borrow().sub.kind();
                    let kind = k.scenery().unwrap();
                    match kind {
                        SceneryKind::Door => {
                            let flags = self.reader.read_u32::<BigEndian>()?;
                            let flags = BitFlags::from_bits(flags)
                                .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                                    format!("unknown door flags: {:x}", flags)))?;
                            trace!("door flags: {:?}", flags);
                            SubObject::Scenery(object::Scenery::Door(object::Door { flags }))
                        }
                        SceneryKind::Stairs => {
                            let location = self.reader.read_u32::<BigEndian>()?;
                            let map = self.reader.read_i32::<BigEndian>()?;
                            let exit = MapExit::decode(map, location)
                                .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                                    format!("invalid map exit: map={} location={}", map, location)))?;
                            SubObject::Scenery(object::Scenery::Stairs(exit))
                        }
                        SceneryKind::Elevator => {
                            let kind = self.reader.read_u32::<BigEndian>()?;
                            let level = self.reader.read_u32::<BigEndian>()?;
                            SubObject::Scenery(object::Scenery::Elevator(object::Elevator { kind, level }))
                        }
                        SceneryKind::LadderDown | SceneryKind::LadderUp => {
                            let map = if f2 {
                                self.reader.read_i32::<BigEndian>()?
                            } else {
                                0
                            };
                            let location = self.reader.read_u32::<BigEndian>()?;
                            let exit = MapExit::decode(map, location)
                                .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                                    format!("invalid map exit: map={} location={}", map, location)))?;
                            SubObject::Scenery(object::Scenery::Ladder(exit))
                        }
                        SceneryKind::Misc => SubObject::None,
                    }
                }
                EntityKind::Misc => {
                    if pid.is_exit_area() {
                        // Exit area.
                        let map = self.reader.read_i32::<BigEndian>()?;
                        trace!("map={}", map);
                        assert!(map >= 0 || fid.idx() >= 33);
                        let map = TargetMap::decode(map)
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                                format!("invalid exit target map: {}", map)))?;
                        /* if charges <= 0
    //          {
    //            v7 = obj->art_fid & 0xFFF;
    //            if ( v7 < 33 )
    //              obj->art_fid = art_id_(OBJ_TYPE_MISC, v7 + 16, (obj->art_fid & 0xFF0000) >> 16, 0);
    //          }*/
                        let pos = self.reader.read_u32::<BigEndian>()?;
                        let pos = TileGrid::default().linear_to_rect_inv(pos);
                        let elevation = self.reader.read_u32::<BigEndian>()?;
                        let pos = pos.elevated(elevation);
                        let direction = Direction::from_u32(self.reader.read_u32::<BigEndian>()?)
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid exit direction"))?;
                        let exit = MapExit {
                            map,
                            pos,
                            direction,
                        };
                        trace!("{:?}", exit);
                        SubObject::MapExit(exit)
                    } else {
                        SubObject::None
                    }
                }
                _ => SubObject::None
            }
        };

        // inventory

        let mut inventory = Inventory {
            items: Vec::with_capacity(inventory_len),
        };
        for i in 0..inventory_len {
            trace!("loading inventory item {}/{}", i, inventory_len);
            let count = self.reader.read_i32::<BigEndian>()?.try_into().unwrap();
            trace!("item count: {}", count);
            let object = self.read_object(f2)?;
            let object = self.objects.insert(object);
            inventory.items.push(InventoryItem {
                object,
                count,
            });
        }

        let mut r = Object::new(fid, proto.into(), pos.map(|p| p.elevated(elevation)), sub);
        r.flags = flags;
        r.updated_flags = updated_flags;
        r.screen_pos = screen_pos;
        r.screen_shift = screen_shift;
        r.fid = fid;
        r.frame_idx = frame_idx;
        r.direction = direction;
        r.set_light_emitter(light_emitter);
        r.inventory = inventory;
        r.outline = outline;
        r.script = script;

        Ok(r)
    }

    fn read_obj_script(&mut self) -> io::Result<Option<(ScriptIid, ProgramId)>> {
        let sid = ScriptIid::read_opt(self.reader)?;
        trace!("sid: {:?}", sid);

        let program_id = self.read_program_id(1)?;
        trace!("program_id: {:?}", program_id);

        if sid.is_some() != program_id.is_some() {
            warn!("bad sid/program_id pair in object");
            return Ok(None);
        }

        Ok(if let (Some(sid), Some(program_id)) = (sid, program_id) {
            Some((sid, program_id))
        } else {
            None
        })
    }

    fn read_outline(&mut self) -> io::Result<Option<Outline>> {
        let flags_u32 = self.reader.read_u32::<BigEndian>()?;
        let flags = &mut BitFlags::from_bits(flags_u32)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("unknown object outline flags: {:x}", flags_u32)))?;

        fn take_bit(flags: &mut BitFlags<OutlineFlag>, flag: OutlineFlag) -> bool {
            let r = flags.contains(flag);
            flags.remove(flag);
            r
        }

        let translucent = take_bit(flags, OutlineFlag::Translucent);
        let disabled = take_bit(flags, OutlineFlag::Translucent);

        let style =
            if take_bit(flags, OutlineFlag::GlowingRed) { OutlineStyle::GlowingRed }
            else if take_bit(flags, OutlineFlag::Red) { OutlineStyle::Red }
            else if take_bit(flags, OutlineFlag::Gray) { OutlineStyle::Gray }
            else if take_bit(flags, OutlineFlag::GlowingGreen) { OutlineStyle::GlowingGreen }
            else if take_bit(flags, OutlineFlag::Yellow) { OutlineStyle::Yellow }
            else if take_bit(flags, OutlineFlag::Brown) { OutlineStyle::Brown }
            else { return Ok(None) };
        if !flags.is_empty() {
            warn!("mutually exclusive outline flags present: 0x{:x}", flags_u32);
            return Ok(Some(Outline {
                style: OutlineStyle::Purple,
                translucent: false,
                disabled: false,
            }));
        }

        Ok(Some(Outline {
            style,
            translucent,
            disabled,
        }))
    }

    fn make_map_script(&mut self, program_id: ProgramId) -> io::Result<()> {
        let sid = self.scripts.instantiate_map_script(program_id)?;
        let mut obj = Object::new(FrameId::MAPMK, None, Some(Default::default()), SubObject::None);
        obj.flags = BitFlags::from(Flag::LightThru)
            | Flag::WalkThru
            | Flag::TurnedOff;
        let objh = self.objects.insert(obj);
        self.scripts.attach_to_object(sid, objh);
        Ok(())
    }

    fn read_program_id(&mut self, offset: i32) -> io::Result<Option<ProgramId>> {
        Ok(self.reader.read_i32::<BigEndian>()?
            .checked_add(offset)
            .and_then(|v| v.try_into().ok())
            .and_then(ProgramId::new))
    }

    fn read_sqr_tiles(&mut self, flags: u32) -> io::Result<SqrTiles> {
        let mut sqr_tiles: Vec<Option<_>> = Vec::with_capacity(ELEVATION_COUNT as usize);
        for i in 0..ELEVATION_COUNT {
            if flags & (1 << (i + 1)) != 0 {
                debug!("no {} elevation", i);
                sqr_tiles.push(None);
                continue;
            }
            let mut tiles = Array2d::with_default(100, 100);
            for y in 0..tiles.height() {
                for x in (0..tiles.width()).rev() {
                    let roof_id = self.reader.read_u16::<BigEndian>()?;
                    let floor_id = self.reader.read_u16::<BigEndian>()?;
                    *tiles.get_mut(x, y).unwrap() = (floor_id, roof_id);
                }
            }
            sqr_tiles.push(Some(tiles));
        }
        Ok(sqr_tiles)
    }
}

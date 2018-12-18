use byteorder::{BigEndian, ReadBytesExt};
use enumflags::BitFlags;
use num_traits::FromPrimitive;
use std::cmp;
use std::io::{self, Error, ErrorKind, prelude::*};

use asset::*;
use asset::frm::{Fid, FrmDb};
use asset::proto::{ItemVariant, Pid, ProtoDb};
use asset::script::ScriptKind;
use game::object::*;
use graphics::{ElevatedPoint, Point};
use graphics::frm::OutlineStyle;
use graphics::geometry::Direction;
use graphics::geometry::hex::TileGrid;
use graphics::geometry::map::ELEVATION_COUNT;
use graphics::render::TextureFactory;
use util::EnumExt;

#[derive(Clone, Copy, Debug, Enum, EnumFlags, Eq, PartialEq)]
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

pub struct Map {
    pub entrance: ElevatedPoint,
    pub entrance_direction: Direction,
    pub sqr_tiles: Vec<Option<Vec<(u16, u16)>>>,
}

pub struct MapReader<'a, R: 'a> {
    pub reader: &'a mut R,
    pub objects: &'a mut Objects,
    pub proto_db: &'a ProtoDb,
    pub frm_db: &'a FrmDb,
    pub tile_grid: &'a TileGrid,
    pub texture_factory: &'a TextureFactory,
}

impl<'a, R: 'a + Read> MapReader<'a, R> {
    pub fn read(&mut self) -> io::Result<Map> {
        // header

        let version = self.reader.read_u32::<BigEndian>()?;

        let mut name = [0; 16];
        self.reader.read_exact(&mut name[..]).unwrap();

        let entrance_pos_lin = self.reader.read_i32::<BigEndian>()?;
        let entrance_pos = self.tile_grid.from_linear_inv(entrance_pos_lin as u32);
        debug!("entrance_pos={} ({:?})", entrance_pos_lin, entrance_pos);
        let entrance_elevation = self.reader.read_u32::<BigEndian>()? as usize;
        assert!(entrance_elevation <= ELEVATION_COUNT);
        let entrance_direction = Direction::from_u32(self.reader.read_u32::<BigEndian>()?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid entrance direction"))?;
        let local_var_count = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let _script_id = self.reader.read_i32::<BigEndian>()?;
        let flags = self.reader.read_i32::<BigEndian>()?;
        println!("flags={}", flags);
        let _ = self.reader.read_i32::<BigEndian>()?;
        let global_var_count = cmp::max(self.reader.read_i32::<BigEndian>()?, 0) as usize;
        let _map_id = self.reader.read_i32::<BigEndian>()?;
        let _time = self.reader.read_u32::<BigEndian>()?;

        self.reader.read_exact(&mut [0; 44 * 4][..])?;

        // global vars

        let mut global_vars = Vec::with_capacity(global_var_count);
        for _ in 0..global_var_count {
            global_vars.push(self.reader.read_i32::<BigEndian>()?);
        }

        // local vars

        let mut local_vars = Vec::with_capacity(local_var_count);
        for _ in 0..local_var_count {
            local_vars.push(self.reader.read_i32::<BigEndian>()?);
        }

        // tiles

        let mut sqr_tiles: Vec<Option<_>> = Vec::with_capacity(ELEVATION_COUNT);

        for i in 0..ELEVATION_COUNT {
            if flags & (1 << (i as u32 + 1)) != 0 {
                debug!("no {} elevation", i);
                sqr_tiles.push(None);
                continue;
            }
            let mut tiles = Vec::with_capacity(10000);
            for _ in 0..tiles.capacity() {
                let roof_id = self.reader.read_u16::<BigEndian>()?;
                let floor_id = self.reader.read_u16::<BigEndian>()?;
                tiles.push((floor_id, roof_id));
            }
            sqr_tiles.push(Some(tiles));
        }

        // scripts

        for _ in ScriptKind::iter() {
            let script_count = self.reader.read_i32::<BigEndian>()?;
            println!("script_count {}", script_count);
            if script_count > 0 {
                let script_count = script_count as u32;
                let node_count = script_count / 16 + (script_count % 16 != 0) as u32;
                println!("node_count {}", node_count);
                for _ in 0..node_count {
                    for _ in 0..16 {
                        // Maps contain garbage at unused slots at [len..16) and
                        // len's position depend on the script kinds of preceding data.

                        let sid = self.reader.read_u32::<BigEndian>()?;

                        let _ = self.reader.read_i32::<BigEndian>()?;

                        if let Some(script_kind) = ScriptKind::from_u32(sid >> 24) {
                            match script_kind {
                                ScriptKind::Spatial => {
                                    let _elevation_and_tile = self.reader.read_i32::<BigEndian>()?;
                                    let _spatial_radius = self.reader.read_i32::<BigEndian>()?;
                                }
                                ScriptKind::Time => {
                                    let _elevation_and_tile = self.reader.read_i32::<BigEndian>()?;
                                }
                                _ => {}
                            }
                        }

                        let _flags = self.reader.read_i32::<BigEndian>()?;
                        let _script_idx = self.reader.read_i32::<BigEndian>()?;
                        let _ = self.reader.read_i32::<BigEndian>()?;
                        let _self_obj_id = self.reader.read_i32::<BigEndian>()?;
                        let _local_var_offset = self.reader.read_i32::<BigEndian>()?;
                        let num_local_vars = self.reader.read_i32::<BigEndian>()?;
                        let _return_value = self.reader.read_i32::<BigEndian>()?;
                        let _action = self.reader.read_i32::<BigEndian>()?;
                        let _ext_param = self.reader.read_i32::<BigEndian>()?;
                        let _action_num = self.reader.read_i32::<BigEndian>()?;
                        let _script_overrides = self.reader.read_i32::<BigEndian>()?;
                        let _unk1 = self.reader.read_i32::<BigEndian>()?;
                        let _how_much = self.reader.read_i32::<BigEndian>()?;
                        let _unk2 = self.reader.read_i32::<BigEndian>()?;

                        let _num_local_vars = if flags & 1 == 0 {
                            0
                        } else {
                            num_local_vars
                        };
                    }

                    let _len = self.reader.read_i32::<BigEndian>()?;
                    let _ = self.reader.read_i32::<BigEndian>()?;
                }
            }
        }

        // objects

        let total_obj_count = self.reader.read_i32::<BigEndian>()?;
        debug!("object count: {}", total_obj_count);
        for elev in 0..ELEVATION_COUNT {
            let obj_count = self.reader.read_u32::<BigEndian>()?;
            debug!("object count at elevation {}: {}", elev, obj_count);

            for _ in 0..obj_count {
                let obj = self.read_obj(version != 19)?;
                self.objects.insert(obj);
            }
        }

        Ok(Map {
            entrance: ElevatedPoint {
                elevation: entrance_elevation,
                point: entrance_pos,
            },
            entrance_direction,
            sqr_tiles,
        })
    }

    fn read_obj(&mut self, f2: bool) -> io::Result<Object> {
        let id = self.reader.read_u32::<BigEndian>()?;
        trace!("object ID {}", id);
        let pos = self.reader.read_i32::<BigEndian>()?;
        trace!("hex_pos={}", pos);
        let screen_shift = Point::new(
            self.reader.read_i32::<BigEndian>()?,
            self.reader.read_i32::<BigEndian>()?);
        let screen_pos = Point::new(
            self.reader.read_i32::<BigEndian>()?,
            self.reader.read_i32::<BigEndian>()?);
        let _frm_idx = self.reader.read_i32::<BigEndian>()?;
        let direction = self.reader.read_u32::<BigEndian>()?;
        let direction = Direction::from_u32(direction)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid object direction: {}", direction)))?;
        let fid = Fid::read(self.reader)?;
        trace!("{:?}", fid);

        self.frm_db.get_or_load(fid, self.texture_factory)?;

        let flags = self.reader.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(flags)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("unknown object flags: {:x}", flags)))?;

        let elevation = self.reader.read_u32::<BigEndian>()? as usize;
        let pid = Pid::read(self.reader)?;
        trace!("{:?} {:?}", pid, self.proto_db.name(pid));
        let _cid = self.reader.read_u32::<BigEndian>()?;
        let light_emitter = LightEmitter {
            radius: self.reader.read_i32::<BigEndian>()? as u32,
            intensity: self.reader.read_i32::<BigEndian>()? as u32,
        };
        let outline = self.read_outline()?;
        trace!("outline: {:?}", outline);
        let _sid = self.reader.read_u32::<BigEndian>()?;
        let _script_idx = self.reader.read_u32::<BigEndian>()?;

        // proto update data

        let inventory_len = self.reader.read_i32::<BigEndian>()?;
        let inventory_capacity = self.reader.read_i32::<BigEndian>()? as usize;
        let _ = self.reader.read_u32::<BigEndian>()?;

        let updated_flags = self.reader.read_u32::<BigEndian>()?;

        if pid.kind() == EntityKind::Critter {
            // combat data
            let _damage_last_turn = self.reader.read_u32::<BigEndian>()?;
            let _combat_state = self.reader.read_u32::<BigEndian>()?;
            let _action_points = self.reader.read_u32::<BigEndian>()?;
            let _damage_flags = self.reader.read_u32::<BigEndian>()?;
            let _ai_packet = self.reader.read_u32::<BigEndian>()?;
            let _team_num = self.reader.read_u32::<BigEndian>()?;
            let _who_hit_me = self.reader.read_u32::<BigEndian>()?;

            let _health = self.reader.read_i32::<BigEndian>()?;
            let _radiation = self.reader.read_i32::<BigEndian>()?;
            let _poison = self.reader.read_i32::<BigEndian>()?;
        } else {
            assert!(updated_flags != 0xcccccccc);
    //            let update_flags = if updated_flags == 0xcccccccc {
    //                0
    //            } else {
    //                updated_flags
    //            };
            match pid.kind() {
                EntityKind::Item => {
                    let proto = self.proto_db.proto(pid).unwrap();
                    match proto.proto.item().unwrap().item {
                        ItemVariant::Weapon(ref proto) => {
                            let _charges = self.reader.read_i32::<BigEndian>()?;
                            let ammo_pid = Pid::from_packed(self.reader.read_u32::<BigEndian>()?);

                            // object_fix_weapon_ammo()
                            let _charges = proto.max_ammo;
                            let _ammo_pid = if ammo_pid.is_none() {
                                proto.ammo_pid
                            } else {
                                ammo_pid
                            };
                        }
                        ItemVariant::Ammo(_) => {
                            let _charges = self.reader.read_i32::<BigEndian>()?;
                        }
                        ItemVariant::Misc(ref proto) => {
                            let charges = self.reader.read_i32::<BigEndian>()?;

                            // object_fix_weapon_ammo()
                            let _charges = if charges < 0 {
                                proto.max_charges
                            } else {
                                charges
                            };
                        }
                        ItemVariant::Key(_) => {
                            let _key_code = self.reader.read_i32::<BigEndian>()?;
                        }
                        _ => {}
                    }
                }
                EntityKind::Scenery => {
                    let k = self.proto_db.kind(pid);
                    println!("{:?} {:?} {:?}", pid, pid.kind(), k);
                    let kind = k.scenery().unwrap();
                    match kind {
                        SceneryKind::Door => {
                            let _walk_thru = self.reader.read_i32::<BigEndian>()?;
                        }
                        SceneryKind::Stairs => {
                            let _dest_map_id = self.reader.read_u32::<BigEndian>()?;
                            let _dest_pos_and_elevation = self.reader.read_u32::<BigEndian>()?;
                        }
                        SceneryKind::Elevator => {
                            let _elevator_kind = self.reader.read_u32::<BigEndian>()?;
                            let _level = self.reader.read_u32::<BigEndian>()?;
                        }
                        SceneryKind::LadderDown | SceneryKind::LadderUp => {
                            if f2 {
                                let _dest_pos_and_elevation = self.reader.read_u32::<BigEndian>()?;
                            }
                            let _dest_map_id = self.reader.read_u32::<BigEndian>()?;
                        }
                        _ => {}
                    }
                }
                EntityKind::Misc => {
                    if pid.is_exit_area() {
                        // Exit area.
                        let map_id = self.reader.read_i32::<BigEndian>()?;
                        trace!("map_id={}", map_id);
                        assert!(map_id >= 0 || fid.id() >= 33);
                        /* if charges <= 0
    //          {
    //            v7 = obj->art_fid & 0xFFF;
    //            if ( v7 < 33 )
    //              obj->art_fid = art_id_(OBJ_TYPE_MISC, v7 + 16, (obj->art_fid & 0xFF0000) >> 16, 0);
    //          }*/
                        let _dude_pos = self.reader.read_u32::<BigEndian>()?;
                        let _elevation = self.reader.read_u32::<BigEndian>()?;
                        let _direction = Direction::from_u32(self.reader.read_u32::<BigEndian>()?)
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid exit direction"));
                    }
                }
                _ => {}
            }
        }

        // inventory

        let mut inventory = Inventory {
            capacity: inventory_capacity,
            items: Vec::with_capacity(inventory_capacity),
        };
        for i in 0..inventory_len {
            trace!("loading inventory item {}/{}", i, inventory_len);
            let count = self.reader.read_i32::<BigEndian>()? as usize;
            trace!("item count: {}", count);
            let object = self.read_obj(f2)?;
            let object = self.objects.insert(object);
            inventory.items.push(InventoryItem {
                object,
                count,
            });
        }

        let pos = if pos >= 0 {
            Some(ElevatedPoint {
                elevation,
                point: self.tile_grid.from_linear_inv(pos as u32),
            })
        } else {
            None
        };
        Ok(Object::new(
            flags,
            pos,
            screen_pos,
            screen_shift,
            fid,
            direction,
            light_emitter,
            Some(pid),
            inventory,
            outline,
        ))
    }

    fn read_outline(&mut self) -> io::Result<Option<Outline>> {
        let flags_u32 = self.reader.read_u32::<BigEndian>()?;
        let ref mut flags: BitFlags<OutlineFlag> = BitFlags::from_bits(flags_u32)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
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
}




#![allow(proc_macro_derive_resolution_fallback)]
#![allow(unused)]
#![deny(non_snake_case)]

extern crate bstring;
extern crate byteorder;
extern crate enumflags;
extern crate env_logger;
#[macro_use] extern crate enumflags_derive;
#[macro_use] extern crate enum_map;
#[macro_use] extern crate enum_primitive_derive;
extern crate flate2;
#[macro_use] extern crate icecream;
#[macro_use] extern crate if_chain;
#[macro_use] extern crate log;
extern crate num_traits;
extern crate png;
extern crate sdl2;
extern crate slotmap;

use byteorder::{BigEndian, ReadBytesExt};
use std::cmp;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, prelude::*};
use std::ops;
use std::path::Path;

mod asset;
mod fs;
mod graphics;
mod util;

use asset::*;
use asset::frm::*;
use asset::proto::*;
use fs::FileSystem;
use graphics::*;
use graphics::color::*;
use graphics::geometry::*;
use graphics::geometry::{hex, sqr};
use graphics::lightmap::LightMap;
use graphics::render::*;
use num_traits::FromPrimitive;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use std::time::Instant;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use graphics::map::render_floor;
use graphics::map::render_roof;
use std::io::Error;
use std::io::ErrorKind;
use std::rc::Rc;
use enumflags::BitFlags;
use asset::Flag;
use asset::message::Messages;
use util::EnumExt;
use graphics::frm::*;
use graphics::geometry::map::MapGrid;

fn read_color_pal(rd: &mut impl Read) -> io::Result<Palette> {
    let mut color_idx_to_rgb18 = [Rgb::black(); 256];
    let mut mapped_colors = [false; 256];
    for i in 0..256 {
        let r = rd.read_u8()?;
        let g = rd.read_u8()?;
        let b = rd.read_u8()?;
        let valid = r < 64 && b < 64 && g < 64;
        if valid {
            color_idx_to_rgb18[i] = Rgb::new(r, g, b);
        }
        mapped_colors[i] = valid;
    }

    let mut rgb15_to_color_idx = [0; 32768];
    rd.read_exact(&mut rgb15_to_color_idx[..])?;

    Ok(Palette::new(color_idx_to_rgb18, rgb15_to_color_idx, mapped_colors))
}

//fn read_frm(rd: &mut impl Read) -> io::Result<Vec<Texture>> {
//    let version = rd.read_u32::<BigEndian>()?;
//    let fps = rd.read_u16::<BigEndian>()?;
//    let action_frame = rd.read_u16::<BigEndian>()?;
//    let frame_count = rd.read_u16::<BigEndian>()?;
//
//    let mut centers = [(0, 0); 6];
//    for i in 0..6 {
//        centers[i].0 = rd.read_i16::<BigEndian>()? as i32;
//    }
//    for i in 0..6 {
//        centers[i].1 = rd.read_i16::<BigEndian>()? as i32;
//    }
//
//    let mut direction_count: usize = 1;
//    {
//        let mut data_offsets = [0; 6];
//        for i in 0..data_offsets.len() {
//            let offset = rd.read_u32::<BigEndian>()?;
//            let count = if i > 0 {
//                data_offsets[0..i - 1].iter().filter(|&&e| e == offset).count()
//            } else {
//                0
//            };
//            assert!(offset == 0 || count == 0);
//            if offset != 0 && count == 0 {
//                direction_count += 1;
//            }
//            data_offsets[i] = offset;
//        }
//        // println!("data_offsets={:?}", data_offsets);
//    }
//    // println!("direction_count={}", direction_count);
//    assert!(direction_count == 1 || direction_count == 6);
//
//    let total_data_len = rd.read_u32::<BigEndian>()?;
//
//    let mut frames = Vec::with_capacity(direction_count * frame_count as usize);
//    for direction in 0..direction_count {
//        for _ in 0..frame_count {
//            let width = rd.read_u16::<BigEndian>()? as i32;
//            let height = rd.read_u16::<BigEndian>()? as i32;
//            let size = (width as u32, height as u32);
//            let _len = rd.read_u32::<BigEndian>()?;
//            let x = rd.read_i16::<BigEndian>()?;
//            let y = rd.read_i16::<BigEndian>()?;
//
//            let len = (width * height) as usize;
//            let mut data = vec![0; len].into_boxed_slice();
//            rd.read_exact(&mut data[..])?;
//            frames.push(Texture {
//                data,
//                width,
//                height,
//            });
//        }
//    }
//
//    Ok(frames)
//}

fn read_frm_<R: Render + ?Sized>(render: &mut R, rd: &mut impl Read) -> io::Result<Vec<TextureHandle>> {
    let version = rd.read_u32::<BigEndian>()?;
    let fps = rd.read_u16::<BigEndian>()?;
    let action_frame = rd.read_u16::<BigEndian>()?;
    let frame_count = rd.read_u16::<BigEndian>()?;

    let mut centers = [(0, 0); 6];
    for i in 0..6 {
        centers[i].0 = rd.read_i16::<BigEndian>()? as i32;
    }
    for i in 0..6 {
        centers[i].1 = rd.read_i16::<BigEndian>()? as i32;
    }

    let mut direction_count: usize = 1;
    {
        let mut data_offsets = [0; 6];
        for i in 0..data_offsets.len() {
            let offset = rd.read_u32::<BigEndian>()?;
            let count = if i > 0 {
                data_offsets[0..i - 1].iter().filter(|&&e| e == offset).count()
            } else {
                0
            };
            assert!(offset == 0 || count == 0);
            if offset != 0 && count == 0 {
                direction_count += 1;
            }
            data_offsets[i] = offset;
        }
        // println!("data_offsets={:?}", data_offsets);
    }
    // println!("direction_count={}", direction_count);
    assert!(direction_count == 1 || direction_count == 6);

    let total_data_len = rd.read_u32::<BigEndian>()?;

    let mut frames = Vec::with_capacity(direction_count * frame_count as usize);
    for direction in 0..direction_count {
        for _ in 0..frame_count {
            let width = rd.read_u16::<BigEndian>()? as i32;
            let height = rd.read_u16::<BigEndian>()? as i32;
            let size = (width as u32, height as u32);
            let _len = rd.read_u32::<BigEndian>()?;
            let x = rd.read_i16::<BigEndian>()?;
            let y = rd.read_i16::<BigEndian>()?;

            let len = (width * height) as usize;
            let mut data = vec![0; len].into_boxed_slice();
            rd.read_exact(&mut data[..])?;
            frames.push(render.new_texture(width, height, data));
        }
    }

    Ok(frames)
}

//fn write_tex_png<P: AsRef<Path>>(path: P, tex: &Texture, pal: &Palette) {
//    use png::HasParameters;
//    use std::io::BufWriter;;
//
//    let file = File::create(path).unwrap();
//    let ref mut w = BufWriter::new(file);
//
//    let mut encoder = png::Encoder::new(w, tex.width as u32, tex.height as u32);
//    encoder.set(png::ColorType::RGB).set(png::BitDepth::Eight);
//     let mut writer = encoder.write_header().unwrap();
//
//    let data = tex.data.iter()
//        .flat_map(|&c| {
//            let Rgb { r, g, b } = pal.rgb24(c);
//            vec![r, g, b].into_iter()
//        })
//        .collect::<Vec<_>>();
//    writer.write_image_data(&data).unwrap();
//}

fn read_light_map<P: AsRef<Path>>(path: P) -> io::Result<LightMap> {
    use std::io::BufReader;
    let ref mut rd = BufReader::new(File::open(path).unwrap());
    use byteorder::LittleEndian;
    let mut r = vec![0u32; (LightMap::WIDTH * LightMap::HEIGHT) as usize].into_boxed_slice();
    for i in 0..r.len() {
        let v = rd.read_i32::<LittleEndian>()?;
        let v = cmp::min(cmp::max(v, 0), 0x10000) as u32;
        r[i] = v;
    }
    Ok(LightMap::with_data(r))
}

fn write_light_map_png<P: AsRef<Path>>(path: P, light_map: &LightMap) {
     use png::HasParameters;
    use std::io::BufWriter;

    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, LightMap::WIDTH as u32, LightMap::HEIGHT as u32);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
     let mut writer = encoder.write_header().unwrap();

    let data = light_map.data().iter()
        .map(|&c| c as f64 / 65536.0 * 255.0)
        .map(|c| c as u8)
        .collect::<Vec<_>>();
    writer.write_image_data(&data).unwrap();
}

const MAX_ELEVATIONS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum ScriptKind {
    System = 0x0,
    Spatial = 0x1,
    Time = 0x2,
    Item = 0x3,
    Critter = 0x4,
}

impl ScriptKind {
    pub const LEN: usize = 5;

    pub fn values() -> &'static [Self] {
        use ScriptKind::*;
        &[System, Spatial, Time, Item, Critter]
    }
}

struct Map {
    entrance: ElevatedPoint,
    sqr_tiles: Vec<Option<Vec<(u16, u16)>>>,
    objs: Vec<Object>,
}

fn load_map(rd: &mut impl Read, proto_db: &ProtoDb, htg: &hex::TileGrid) -> io::Result<Map> {
    // header

    let version = rd.read_u32::<BigEndian>()?;

    let mut name = [0; 16];
    rd.read_exact(&mut name[..]).unwrap();

    let entrance_pos_lin = rd.read_i32::<BigEndian>()?;
    let entrance_pos = htg.from_linear(entrance_pos_lin);
    debug!("entrance_pos={} ({:?})", entrance_pos_lin, entrance_pos);
    let entrance_elevation = rd.read_u32::<BigEndian>()? as usize;
//    assert!(entrance_elevation <= MAX_ELEVATIONS);
    let entrance_direction = Direction::from_u32(rd.read_u32::<BigEndian>()?)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid entrance direction"));
    let local_var_count = cmp::max(rd.read_i32::<BigEndian>()?, 0) as usize;
    let script_id = rd.read_i32::<BigEndian>()?;
    let flags = rd.read_i32::<BigEndian>()?;
    println!("flags={}", flags);
    let _ = rd.read_i32::<BigEndian>()?;
    let global_var_count = cmp::max(rd.read_i32::<BigEndian>()?, 0) as usize;
    let map_id = rd.read_i32::<BigEndian>()?;
    let time = rd.read_u32::<BigEndian>()?;

    rd.read_exact(&mut [0; 44 * 4][..])?;

    // global vars

    let mut global_vars = Vec::with_capacity(global_var_count);
    for _ in 0..global_var_count {
        global_vars.push(rd.read_i32::<BigEndian>()?);
    }

    // local vars

    let mut local_vars = Vec::with_capacity(local_var_count);
    for _ in 0..local_var_count {
        local_vars.push(rd.read_i32::<BigEndian>()?);
    }

    // tiles

    let mut sqr_tiles: Vec<Option<_>> = Vec::with_capacity(MAX_ELEVATIONS);

    for i in 0..MAX_ELEVATIONS {
        if flags & (1 << (i as u32 + 1)) != 0 {
            debug!("no {} elevation", i);
            sqr_tiles.push(None);
            continue;
        }
        let mut tiles = Vec::with_capacity(10000);
        for _ in 0..tiles.capacity() {
            let roof_id = rd.read_u16::<BigEndian>()?;
            let floor_id = rd.read_u16::<BigEndian>()?;
            tiles.push((floor_id, roof_id));
        }
        sqr_tiles.push(Some(tiles));
    }

    // scripts

    for _ in 0..ScriptKind::LEN {
        let script_count = rd.read_i32::<BigEndian>()?;
        println!("script_count {}", script_count);
        if script_count > 0 {
            let script_count = script_count as u32;
            let node_count = script_count / 16 + (script_count % 16 != 0) as u32;
            println!("node_count {}", node_count);
            for _ in 0..node_count {
                for _ in 0..16 {
                    // Maps contain garbage at unused slots at [len..16) and
                    // len's position depend on the script kinds of preceding data.

                    let sid = rd.read_u32::<BigEndian>()?;

                    let _ = rd.read_i32::<BigEndian>()?;

                    if let Some(script_kind) = ScriptKind::from_u32(sid >> 24) {
                        match script_kind {
                            ScriptKind::Spatial => {
                                let elevation_and_tile = rd.read_i32::<BigEndian>()?;
                                let spatial_radius = rd.read_i32::<BigEndian>()?;
                            }
                            ScriptKind::Time => {
                                let elevation_and_tile = rd.read_i32::<BigEndian>()?;
                            }
                            _ => {}
                        }
                    }

                    let _flags = rd.read_i32::<BigEndian>()?;
                    let script_idx = rd.read_i32::<BigEndian>()?;
                    let _ = rd.read_i32::<BigEndian>()?;
                    let self_obj_id = rd.read_i32::<BigEndian>()?;
                    let local_var_offset = rd.read_i32::<BigEndian>()?;
                    let num_local_vars = rd.read_i32::<BigEndian>()?;
                    let return_value = rd.read_i32::<BigEndian>()?;
                    let action = rd.read_i32::<BigEndian>()?;
                    let ext_param = rd.read_i32::<BigEndian>()?;
                    let action_num = rd.read_i32::<BigEndian>()?;
                    let script_overrides = rd.read_i32::<BigEndian>()?;
                    let unk1 = rd.read_i32::<BigEndian>()?;
                    let how_much = rd.read_i32::<BigEndian>()?;
                    let unk2 = rd.read_i32::<BigEndian>()?;

                    let num_local_vars = if flags & 1 == 0 {
                        0
                    } else {
                        num_local_vars
                    };
                }

                let len = rd.read_i32::<BigEndian>()?;
                let _ = rd.read_i32::<BigEndian>()?;
            }
        }
    }

    // objects

    let total_obj_count = rd.read_i32::<BigEndian>()?;
    debug!("object count: {}", total_obj_count);
    let mut objs = Vec::with_capacity(total_obj_count as usize);
    for elev in 0..MAX_ELEVATIONS {
        let obj_count = rd.read_u32::<BigEndian>()?;
        debug!("object count at elevation {}: {}", elev, obj_count);

        for _ in 0..obj_count {
            let obj = read_obj(rd, proto_db, version != 19, htg)?;
            objs.push(obj)
        }
    }

    Ok(Map {
        entrance: ElevatedPoint {
            elevation: entrance_elevation,
            point: entrance_pos,
        },
        sqr_tiles,
        objs,
    })
}

fn read_obj(rd: &mut impl Read, proto_db: &ProtoDb, f2: bool, htg: &hex::TileGrid) -> io::Result<Object> {
    fn do_read(rd: &mut impl Read, proto_db: &ProtoDb, f2: bool, htg: &hex::TileGrid) -> io::Result<Object> {
        let id = rd.read_u32::<BigEndian>()?;
        trace!("object ID {}", id);
        let hex_pos = rd.read_i32::<BigEndian>()?;
        trace!("hex_pos={}", hex_pos);
        let scr_shift = Point::new(
            rd.read_i32::<BigEndian>()?,
            rd.read_i32::<BigEndian>()?);
        let scr_pos = Point::new(
            rd.read_i32::<BigEndian>()?,
            rd.read_i32::<BigEndian>()?);
        let frm_idx = rd.read_i32::<BigEndian>()?;
        let direction = rd.read_u32::<BigEndian>()?;
        let direction = Direction::from_u32(direction)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid object direction: {}", direction)))?;
        let fid = Fid::read(rd)?;
        trace!("{:?}", fid);

        let flags = rd.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(flags)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("unknown object flags: {:x}", flags)))?;

        let elevation = rd.read_u32::<BigEndian>()? as usize;
        let pid = Pid::read(rd)?;
        trace!("{:?} {:?}", pid, proto_db.name(pid));
        let cid = rd.read_u32::<BigEndian>()?;
        let light_radius = rd.read_i32::<BigEndian>()?;
        let light_intensity = rd.read_i32::<BigEndian>()?;
        let _outline_color = rd.read_u32::<BigEndian>()?;
        let sid = rd.read_u32::<BigEndian>()?;
        let script_idx = rd.read_u32::<BigEndian>()?;

        // proto update data

        let inventory_len = rd.read_i32::<BigEndian>()?;
        let inventory_max = rd.read_i32::<BigEndian>()?;
        let _ = rd.read_u32::<BigEndian>()?;

        let updated_flags = rd.read_u32::<BigEndian>()?;

        if pid.kind() == EntityKind::Critter {
            // combat data
            let damage_last_turn = rd.read_u32::<BigEndian>()?;
            let combat_state = rd.read_u32::<BigEndian>()?;
            let action_points = rd.read_u32::<BigEndian>()?;
            let damage_flags = rd.read_u32::<BigEndian>()?;
            let ai_packet = rd.read_u32::<BigEndian>()?;
            let team_num = rd.read_u32::<BigEndian>()?;
            let who_hit_me = rd.read_u32::<BigEndian>()?;

            let health = rd.read_i32::<BigEndian>()?;
            let radiation = rd.read_i32::<BigEndian>()?;
            let poison = rd.read_i32::<BigEndian>()?;
        } else {
            assert!(updated_flags != 0xcccccccc);
//            let update_flags = if updated_flags == 0xcccccccc {
//                0
//            } else {
//                updated_flags
//            };
            match pid.kind() {
                EntityKind::Item => {
                    let proto = proto_db.proto(pid).unwrap();
                    match proto.proto.item().unwrap().item {
                        ItemVariant::Weapon(ref proto) => {
                            let charges = rd.read_i32::<BigEndian>()?;
                            let ammo_pid = Pid::from_packed(rd.read_u32::<BigEndian>()?);

                            // object_fix_weapon_ammo()
                            let charges = proto.max_ammo;
                            let ammo_pid = if ammo_pid.is_none() {
                                proto.ammo_pid
                            } else {
                                ammo_pid
                            };
                        }
                        ItemVariant::Ammo(_) => {
                            let charges = rd.read_i32::<BigEndian>()?;
                        }
                        ItemVariant::Misc(ref proto) => {
                            let charges = rd.read_i32::<BigEndian>()?;

                            // object_fix_weapon_ammo()
                            let charges = if charges < 0 {
                                proto.max_charges
                            } else {
                                charges
                            };
                        }
                        ItemVariant::Key(_) => {
                            let key_code = rd.read_i32::<BigEndian>()?;
                        }
                        _ => {}
                    }
                }
                EntityKind::Scenery => {
                    let k = proto_db.kind(pid);
                    println!("{:?} {:?} {:?}", pid, pid.kind(), k);
                    let kind = k.scenery().unwrap();
                    match kind {
                        SceneryKind::Door => {
                            let walk_thru = rd.read_i32::<BigEndian>()?;
                        }
                        SceneryKind::Stairs => {
                            let dest_map_id = rd.read_u32::<BigEndian>()?;
                            let dest_pos_and_elevation = rd.read_u32::<BigEndian>()?;
                        }
                        SceneryKind::Elevator => {
                            let elevator_kind = rd.read_u32::<BigEndian>()?;
                            let level = rd.read_u32::<BigEndian>()?;
                        }
                        SceneryKind::LadderDown | SceneryKind::LadderUp => {
                            if f2 {
                                let dest_pos_and_elevation = rd.read_u32::<BigEndian>()?;
                            }
                            let dest_map_id = rd.read_u32::<BigEndian>()?;
                        }
                        _ => {}
                    }
                }
                EntityKind::Misc => {
                    if pid.is_exit_area() {
                        // Exit area.
                        let map_id = rd.read_i32::<BigEndian>()?;
                        trace!("map_id={}", map_id);
                        assert!(map_id >= 0 || fid.id0() >= 33);
                        /* if charges <= 0
//          {
//            v7 = obj->art_fid & 0xFFF;
//            if ( v7 < 33 )
//              obj->art_fid = art_id_(OBJ_TYPE_MISC, v7 + 16, (obj->art_fid & 0xFF0000) >> 16, 0);
//          }*/
                        let dude_pos = rd.read_u32::<BigEndian>()?;
                        let elevation = rd.read_u32::<BigEndian>()?;
                        let direction = Direction::from_u32(rd.read_u32::<BigEndian>()?)
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid exit direction"));
                    }
                }
                _ => {}
            }
        }

        // inventory

        for i in 0..inventory_len {
            trace!("loading inventory item {}/{}", i, inventory_len);
            let item_count = rd.read_i32::<BigEndian>()?;
            trace!("item count: {}", item_count);
            do_read(rd, proto_db, f2, htg)?;
        }

        let hex_pos = if hex_pos >= 0 {
            Some(ElevatedPoint {
                elevation,
                point: htg.from_linear(hex_pos),
            })
        } else {
            None
        };
        Ok(Object {
            hex_pos,
            scr_shift,
            scr_pos,
            fid,
            frame_idx: 0,
            direction,
            flags,
            pid: Some(pid),
        })
    }

    do_read(rd, proto_db, f2, htg)
}

fn all_fids(fid: Fid) -> Vec<Fid> {
    let mut r = vec![fid];
    match fid.kind() {
        EntityKind::Critter => {
            for wk in WeaponKind::iter() {
                for anim in CritterAnim::iter() {
                    for variant in 0..=5 {
                        r.push(Fid::new(EntityKind::Critter, variant as u8, anim as u8, wk as u8, fid.id0()).unwrap());
                    }
                }
            }
        }
        _ => {}
    }
    r
}

struct Object {
//    id: i32,
    hex_pos: Option<ElevatedPoint>,
    scr_pos: Point,
    scr_shift: Point,
    fid: Fid,
    frame_idx: usize,
    direction: Direction,
    flags: BitFlags<Flag>,
//    elevation: usize,
//      Inventory inventory;
//  int updated_flags;
//  GameObject::ItemOrCritter _;
    pid: Option<Pid>,
//  int cid;
//  int light_radius;
//  int light_intensity;
//  int outline;
//  int script_id;
//  GameObject *owner;
//  int script_idx;
}

//fn render_obj(render: &mut Render, rect: &Rect, obj: &mut Object, frm_db: &FrmDb,
//        hex_tg: &hex::TileGrid, light: u32) {
//    let frms = frm_db.get(obj.fid);
//    let frml = &frms.frame_lists[obj.direction];
//    let frm = &frml.frames[obj.frame_idx];
//
//    let bounds = if let Some(ElevatedPoint { point: hex_pos, .. }) = obj.hex_pos {
//        let scr_pos = hex_tg.to_screen(hex_pos);
//        let frm_center = frml.center;
//        let p = scr_pos + frm_center + obj.scr_shift + Point::new(16, 8);
//        Rect {
//            left: p.x - frm.width / 2,
//            top: p.y - frm.height + 1,
//            right: p.x + frm.width / 2,
//            bottom: p.y + 1,
//        }
//    } else {
//        Rect::with_size(obj.scr_pos.x, obj.scr_pos.y, frm.width, frm.height - 1)
//    };
//    obj.scr_pos = bounds.top_left();
//
//    if rect.intersect(&bounds).is_empty() {
//        return;
//    }
//
//    if obj.fid.kind() == EntityKind::Interface {
//        render.draw(&frm.texture, bounds.left, bounds.right, 0x10000);
//        return;
//    }
//
//    match obj.fid.kind() {
//        EntityKind::Scenery | EntityKind::Wall => {
//            // TODO handle egg
//            render.draw(&frm.texture, bounds.left, bounds.top, light);
//        }
//        _ => {
//            // TODO handle transparency TRANS_*
//
//            render.draw(&frm.texture, bounds.left, bounds.top, light);
//        }
//    }
//}

impl Object {
    fn render(&mut self, render: &mut Render, rect: &Rect, light: u32,
            frm_db: &FrmDb, proto_db: &ProtoDb, hex_tg: &hex::TileGrid,
            egg_hex_pos: Point, egg_fid: Fid) {
        let (pos, centered) = if let Some(ElevatedPoint { point: hex_pos, .. }) = self.hex_pos {
            (hex_tg.to_screen(hex_pos) + self.scr_shift + Point::new(16, 8), true)
        } else {
            (self.scr_pos, false)
        };

        let effect = match self.fid.kind() {
            EntityKind::Interface => None,
            EntityKind::Scenery | EntityKind::Wall if self.hex_pos.is_some() && self.pid.is_some() => {
                let flags_ext = proto_db.proto(self.pid.unwrap()).unwrap().flags_ext;
                let mask_pos = hex_tg.to_screen(egg_hex_pos) + Point::new(16, 8)/*+ self.scr_shift */;
                Some(Effect::Masked { mask_fid: egg_fid, mask_pos })
            }
            _ => match () {
                _ if self.flags.contains(Flag::TransEnergy) => Some(Translucency::Energy),
                _ if self.flags.contains(Flag::TransGlass) => Some(Translucency::Glass),
                _ if self.flags.contains(Flag::TransRed) => Some(Translucency::Red),
                _ if self.flags.contains(Flag::TransSteam) => Some(Translucency::Steam),
                _ if self.flags.contains(Flag::TransWall) => Some(Translucency::Wall),
                _ => None,
            }.map(Effect::Translucency)
        };

        let sprite = Sprite {
            pos,
            centered,
            fid: self.fid,
            frame_idx: self.frame_idx,
            direction: self.direction,
            light,
            effect,
        };
        self.scr_pos = sprite.render(render, rect, frm_db).top_left();
    }
}

fn main() {
    env_logger::init();

    let master_dat = "../../Dropbox/f2/MASTER.DAT";
    let critter_dat = "../../Dropbox/f2/CRITTER.DAT";
    let mut fs = fs::FileSystem::new();
    fs.register_provider(Box::new(fs::dat::v2::Dat::new(master_dat).unwrap()));
    fs.register_provider(Box::new(fs::dat::v2::Dat::new(critter_dat).unwrap()));
    let fs = Rc::new(fs);
    let proto_db = ProtoDb::new(fs.clone(), "english").unwrap();
    let frm_db = FrmDb::new(fs.clone(), "english").unwrap();

//    let messages = Messages::read(&mut fs.reader("text/english/game/skill.msg").unwrap()).unwrap();
//    println!("{:#?}", messages);
//    return;

    let pal = read_color_pal(&mut fs.reader("color.pal").unwrap()).unwrap();

    let sdl = sdl2::init().unwrap();
    let mut event_pump = sdl.event_pump().unwrap();
    let video = sdl.video().unwrap();

    let window = video.window("Vault 13", 640, 480)
        .position_centered()
        .allow_highdpi()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    {
        use graphics::render::software::SoftwareRender;
        let mut overlay = PaletteOverlay::standard();
        let mut render: Box<Render> = Box::new(SoftwareRender::new(canvas, Box::new(pal.clone()),
            overlay)) as Box<Render>;
        let mut render = render.as_mut();

//        let kind = EntityKind::Critter;
//        for i in 1..proto_db.len(kind)+1 {
//            let pid = Pid::new(kind, i as u32);
//            println!("{:?}", proto_db.name(pid));
//            println!("{:?}", proto_db.description(pid));
//            if let Ok(proto) = proto_db.proto(pid) {
////                let path = frm_db.name(proto.fid).unwrap();
////                println!("{:?} {:?} {:?}", proto.fid, path, fs.metadata(&path).unwrap());
////                let frm = frm_db.get_or_load(proto.fid, render);
////                println!("{} {:#?}", i, frm);
//                println!("{:?}", proto.fid);
//                for fid in all_fids(proto.fid) {
//                    if frm_db.exists(fid) {
//                        println!("  {:?}", fid);
//                        println!("  {:?}", frm_db.name(fid));
//                    }
//                }
//
//            } else {
//                println!("{:?}", proto_db.proto(pid));
//            }
//    //        println!("{:#?}", proto_db.proto(pid));
//            println!();
//        }
//        return;

        let mut mg = MapGrid::new(640, 380);
        
        /*let maps = vec![
            "maps/arbridge.map",
            "maps/arcaves.map",
            "maps/ardead.map",
            "maps/argarden.map",
            "maps/artemple.map",
            "maps/arvill2.map",
            "maps/arvillag.map",
            "maps/bhrnddst.map",
            "maps/bhrndmtn.map",
            "maps/broken1.gam",
            "maps/broken1.map",
            "maps/broken2.gam",
            "maps/broken2.map",
            "maps/cardesrt.gam",
            "maps/cardesrt.map",
            "maps/cave0.map",
            "maps/cave06.gam",
            "maps/cave1.map",
            "maps/cave2.map",
            "maps/cave3.map",
            "maps/cave4.map",
            "maps/cave5.map",
            "maps/cave6.map",
            "maps/cave7.map",
            "maps/city1.map",
            "maps/city2.map",
            "maps/city3.map",
            "maps/city4.map",
            "maps/city5.map",
            "maps/city6.map",
            "maps/city7.map",
            "maps/city8.map",
            "maps/coast1.map",
            "maps/coast10.map",
            "maps/coast11.map",
            "maps/coast12.map",
            "maps/coast2.map",
            "maps/coast3.map",
            "maps/coast4.map",
            "maps/coast5.map",
            "maps/coast6.map",
            "maps/coast7.map",
            "maps/coast8.map",
            "maps/coast9.map",
            "maps/cowbomb.gam",
            "maps/cowbomb.map",
            "maps/denbus1.cfg",
            "maps/denbus1.gam",
            "maps/denbus1.map",
            "maps/denbus2.gam",
            "maps/denbus2.map",
            "maps/denres1.gam",
            "maps/denres1.map",
            "maps/depolv1.gam",
            "maps/depolv1.map",
            "maps/depolva.gam",
            "maps/depolva.map",
            "maps/depolvb.gam",
            "maps/depolvb.map",
            "maps/desert1.map",
            "maps/desert2.map",
            "maps/desert3.map",
            "maps/desert4.map",
            "maps/desert5.map",
            "maps/desert6.map",
            "maps/desert7.map",
            "maps/desert8.map",
            "maps/desert9.map",
            "maps/desrt10.map",
            "maps/desrt11.map",
            "maps/desrt12.map",
            "maps/desrt13.map",
            "maps/dnslvrun.map",
            "maps/encdet.map",
            "maps/encdock.map",
            "maps/encfite.gam",
            "maps/encfite.map",
            "maps/encgd.map",
            "maps/encpres.gam",
            "maps/encpres.map",
            "maps/encrctr.gam",
            "maps/encrctr.map",
            "maps/enctrp.gam",
            "maps/enctrp.map",
            "maps/gammovie.gam",
            "maps/gammovie.map",
            "maps/geckjunk.gam",
            "maps/geckjunk.map",
            "maps/geckpwpl.gam",
            "maps/geckpwpl.map",
            "maps/gecksetl.cfg",
            "maps/gecksetl.gam",
            "maps/gecksetl.map",
            "maps/gecktunl.gam",
            "maps/gecktunl.map",
            "maps/gstcav1.gam",
            "maps/gstcav1.map",
            "maps/gstcav2.gam",
            "maps/gstcav2.map",
            "maps/gstfarm.gam",
            "maps/gstfarm.map",
            "maps/klacanyn.gam",
            "maps/klacanyn.map",
            "maps/kladwtwn.cfg",
            "maps/kladwtwn.gam",
            "maps/kladwtwn.map",
            "maps/klagraz.gam",
            "maps/klagraz.map",
            "maps/klamall.gam",
            "maps/klamall.map",
            "maps/klaratcv.map",
            "maps/klatoxcv.gam",
            "maps/klatoxcv.map",
            "maps/klatrap.map",
            "maps/mbase12.cfg",
            "maps/mbase12.map",
            "maps/mbase34.cfg",
            "maps/mbase34.gam",
            "maps/mbase34.map",
            "maps/mbclose.gam",
            "maps/mbclose.map",
            "maps/modbrah.gam",
            "maps/modbrah.map",
            "maps/modgard.gam",
            "maps/modgard.map",
            "maps/modinn.gam",
            "maps/modinn.map",
            "maps/modmain.gam",
            "maps/modmain.map",
            "maps/modshit.gam",
            "maps/modshit.map",
            "maps/modwell.gam",
            "maps/modwell.map",
            "maps/mountn1.map",
            "maps/mountn2.map",
            "maps/mountn3.map",
            "maps/mountn4.map",
            "maps/mountn5.map",
            "maps/mountn6.map",
            "maps/navarro.cfg",
            "maps/navarro.gam",
            "maps/navarro.map",
            "maps/ncr1.gam",
            "maps/ncr1.map",
            "maps/ncr2.cfg",
            "maps/ncr2.gam",
            "maps/ncr2.map",
            "maps/ncr3.gam",
            "maps/ncr3.map",
            "maps/ncr4.gam",
            "maps/ncr4.map",
            "maps/ncrent.gam",
            "maps/ncrent.map",
            "maps/newr1.cfg",
            "maps/newr1.gam",
            "maps/newr1.map",
            "maps/newr1a.map",
            "maps/newr2.gam",
            "maps/newr2.map",
            "maps/newr2a.map",
            "maps/newr3.gam",
            "maps/newr3.map",
            "maps/newr4.gam",
            "maps/newr4.map",
            "maps/newrba.gam",
            "maps/newrba.map",
            "maps/newrcs.gam",
            "maps/newrcs.map",
            "maps/newrgo.gam",
            "maps/newrgo.map",
            "maps/newrst.gam",
            "maps/newrst.map",
            "maps/newrvb.map",
            "maps/raiders1.gam",
            "maps/raiders1.map",
            "maps/raiders2.cfg",
            "maps/raiders2.gam",
            "maps/raiders2.map",
            "maps/reddown.gam",
            "maps/reddown.map",
            "maps/reddtun.cfg",
            "maps/reddtun.map",
            "maps/redment.gam",
            "maps/redment.map",
            "maps/redmtun.map",
            "maps/redwame.cfg",
            "maps/redwame.gam",
            "maps/redwame.map",
            "maps/redwan1.cfg",
            "maps/redwan1.map",
            "maps/rndbess.gam",
            "maps/rndbess.map",
            "maps/rndbhead.gam",
            "maps/rndbhead.map",
            "maps/rndbridg.gam",
            "maps/rndbridg.map",
            "maps/rndcafe.gam",
            "maps/rndcafe.map",
            "maps/rndexcow.gam",
            "maps/rndexcow.map",
            "maps/rndforvr.gam",
            "maps/rndforvr.map",
            "maps/rndholy1.gam",
            "maps/rndholy1.map",
            "maps/rndholy2.gam",
            "maps/rndholy2.map",
            "maps/rndparih.gam",
            "maps/rndparih.map",
            "maps/rndshutl.gam",
            "maps/rndshutl.map",
            "maps/rndtinwd.gam",
            "maps/rndtinwd.map",
            "maps/rndtoxic.gam",
            "maps/rndtoxic.map",
            "maps/rnduvilg.gam",
            "maps/rnduvilg.map",
            "maps/rndwhale.gam",
            "maps/rndwhale.map",
            "maps/sfchina.cfg",
            "maps/sfchina.gam",
            "maps/sfchina.map",
            "maps/sfchina2.cfg",
            "maps/sfchina2.map",
            "maps/sfdock.map",
            "maps/sfelronb.gam",
            "maps/sfelronb.map",
            "maps/sfshutl1.map",
            "maps/sfshutl2.map",
            "maps/sftanker.gam",
            "maps/sftanker.map",
            "maps/v13_orig.map",
            "maps/v13ent.gam",
            "maps/v13ent.map",
            "maps/v15_orig.map",
            "maps/v15ent.gam",
            "maps/v15ent.map",
            "maps/v15sent.gam",
            "maps/v15sent.map",
            "maps/vault13.cfg",
            "maps/vault13.gam",
            "maps/vault13.map",
            "maps/vault15.gam",
            "maps/vault15.map",
            "maps/vctycocl.gam",
            "maps/vctycocl.map",
            "maps/vctyctyd.gam",
            "maps/vctyctyd.map",
            "maps/vctydwtn.gam",
            "maps/vctydwtn.map",
            "maps/vctyvlt.gam",
            "maps/vctyvlt.map",
        ];*/

        let maps = vec!["maps/ncr2.map"];

        let mut map = None;

        for f in &maps {
            if !f.ends_with(".map") {
                continue;
            }
            ic!(f);
            let mut m = load_map(&mut fs.reader(f).unwrap(), &proto_db, mg.hex()).unwrap();

            for elev in &m.sqr_tiles {
                if let Some(ref elev) = elev {
                    for &(floor, roof) in elev {
                        frm_db.get_or_load(Fid::new(EntityKind::SqrTile, 0, 0, 0, floor).unwrap(), render);
                        frm_db.get_or_load(Fid::new(EntityKind::SqrTile, 0, 0, 0, roof).unwrap(), render);
                    }
                }
            }

            for obj in &m.objs {
                frm_db.get_or_load(obj.fid, render).unwrap();
            }

            map = Some(m);
        }

//        let obj_fid = Fid::from_packed(0x013f003f).unwrap();
//        println!("{:?}", frm_db.name(obj_fid));
//        return;
//        frm_db.get_or_load(obj_fid, render).unwrap();

        let mut map = map.unwrap();

        let mut player_pos = map.entrance;
//        let player_pos = htg.from_linear(0x4450);
        ic!(player_pos);
//        let p = stg.from_screen(htg.to_screen(player_pos.point));
        mg.center2(player_pos.point);
//        println!("{:#?} {:#?}", htg, stg);
//        println!("{:?}", htg.to_screen(htg.from_linear(0x2da4)));
//        return;
//        stg.set_pos(p);
//        stg.set_screen_pos((245, 160));
//        stg.set_pos((40, 45));

        let dude_fid = Fid::from_packed(0x0100003e).unwrap();
//        let dude_fid = Fid::EGG;
        frm_db.get_or_load(dude_fid, render).unwrap();
        let mut dude_obj = Object {
            hex_pos: Some(player_pos),
            scr_shift: Point::new(0, 0),
            scr_pos: Point::new(0, 0),
            fid: dude_fid,
            frame_idx: 0,
            direction: Direction::NE,
            flags: BitFlags::empty(),
            pid: None,
        };


        let visible_rect = Rect::with_size(0, 0, 640, 380);



//        let start_y = stg.from_screen((0, 0)).y;
//        let start_x = stg.from_screen((639, 0)).x;
//        let end_x = stg.from_screen((0, 479)).x;
//        let end_y = stg.from_screen((639, 479)).y;
//        println!("{} {} {} {}", start_x, end_x, start_y, end_y);
//
//        for sqr_y in start_y..=end_y {
//            for sqr_x in (end_x..=start_x).rev() {
////                println!("{} {} {:?}", sqr_x, sqr_y, stg.to_linear((sqr_x, sqr_y)));
//                if let Some(tile_num) = stg.to_linear((sqr_x, sqr_y)) {
//                    let tile_id = map_tiles[tile_num as usize].0;
////                    println!("{} {} {} tile_id:{} {}", tile_num, sqr_x, sqr_y, tile_id, tiles_lst[tile_id as usize]);
//                    if let Some(tex) = map_tiles_tex.get(&tile_id) {
//                        let scr_pt = stg.to_screen((sqr_x, sqr_y));
////                        println!("{} {} {} {:?}", tile_num, sqr_x, sqr_y, scr_pt);
//                        render.draw(tex, scr_pt.x, scr_pt.y, 0x10000);
//                    } else {
//                        println!("no tex {}", tile_id);
//                    }
//                }
//            }
//        }

//        return;

        frm_db.get_or_load(Fid::EGG, render).unwrap();

        let scroll_inc = 10;
        let mut elevation = map.entrance.elevation;

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                        mg.scroll((scroll_inc, 0));
                        player_pos.point = mg.center_hex();
                    }
                    Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                        mg.scroll((-scroll_inc, 0));
                        player_pos.point = mg.center_hex();
                    }
                    Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                        mg.scroll((0, -scroll_inc));
                        player_pos.point = mg.center_hex();
                    }
                    Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                        mg.scroll((0, scroll_inc));
                        player_pos.point = mg.center_hex();
                    }
                    Event::KeyDown { keycode: Some(Keycode::X), .. } => {
                        let mut d = dude_obj.direction.ordinal() as isize - 1;
                        if d < 0 {
                            d += Direction::len() as isize;
                        }
                        dude_obj.direction = Direction::from_ordinal(d as usize);
                    }
                    Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                        dude_obj.direction = Direction::from_ordinal((dude_obj.direction.ordinal() + 1) % Direction::len());
                    }
                    Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                        let new_elevation = map.sqr_tiles.iter().enumerate()
                            .skip(elevation)
                            .filter_map(|(i, v)| v.as_ref().map(|_| i))
                            .next();
                        if let Some(new_elevation) = new_elevation {
                            elevation = new_elevation;
                        }
                    }
                    Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }

            render_floor(render, mg.sqr(), &visible_rect,
                |num| Some(frm_db.get(Fid::new(EntityKind::SqrTile, 0, 0, 0, map.sqr_tiles[elevation].as_ref().unwrap()[num as usize].0).unwrap()).frame_lists[Direction::NE].frames[0].texture.clone())
            );
            for obj in &mut map.objs {
                let elevation = elevation;
                if obj.hex_pos.map(|p| p.elevation == elevation).unwrap_or(true) {
                    obj.render(render, &visible_rect, 0x10000, &frm_db, &proto_db, mg.hex(),
                        player_pos.point, Fid::EGG);
                }
            }

            dude_obj.hex_pos = Some(player_pos);
            dude_obj.render(render, &visible_rect, 0x10000, &frm_db, &proto_db, mg.hex(),
                player_pos.point, Fid::EGG);

//            render_roof(render, &stg, &visible_rect,
//                |num| Some(frm_db.get(Fid::new(EntityKind::SqrTile, 0, 0, 0, map.sqr_tiles[map.entrance.elevation].as_ref().unwrap()[num as usize].1).unwrap()).frame_lists[Direction::NE].frames[0].texture.clone())
//            );

            let now = Instant::now();
            render.update(now);
            render.present();
            render.cleanup();

            ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
        }
    }
    return;

//    fn is_mapped_color(c_idx: usize) -> bool {
//        c_idx > 0 &&
//            c_idx < 229
//    }
//
//
//    {
//        use std::fs::File;
//        let mut expected = Vec::with_capacity(256*256);
//        File::open("../../devp/vault13/misc/reveng/render/misc/expected_darken_lighten_lut.raw").unwrap()
//            .read_to_end(&mut expected)
//            .unwrap();
//
//        for c_idx in 0..256 {
//            for amount in 0..256 {
//                let e = expected[c_idx * 256 + amount];
////                println!("{} {:x}", c1idx * 256 + c2idx, e);
//                if !is_mapped_color(c_idx) || !is_mapped_color(c_idx) {
//                    continue;
//                }
//
//                let rgb15 = pal.rgb15(c_idx as u8);
//                let a = if amount <= 128 {
//                    rgb15.darken(amount as u8)
//                } else {
//                    rgb15.lighten(amount as u8 - 128)
//                };
//
//                assert_eq!(pal.color_idx(a), e, "{} {}", c_idx, amount);
//            }
//        }
//
////        panic!();
//    }

//    {
////        println!("{:x}", pal.rgb15(1).as_u16());
//        let c1 = Rgb15::with_colors(29, 29, 29);
//        let c2 = Rgb15::with_colors(27, 24, 24);
////        println!("{:?} {:x} {:?} {:x}", c1, c1.as_u16(), c2, c2.as_u16());
////        let mixed = c1.add(c2, |c| pal.quantize(c));
////        println!("{:?}", mixed);
////        println!("!!!{:x}", pal.color_idx(mixed));
////        println!("!!!{:?}", pal.rgb15(182));
////        panic!();
//    }
//
//    {
//        use std::fs::File;
//        let mut expected = Vec::with_capacity(256*256);
//        File::open("../../devp/vault13/misc/reveng/render/misc/expected_rgb15_add_lut.raw").unwrap()
//            .read_to_end(&mut expected)
//            .unwrap();
//
//        for c1idx in 0..256 {
//            for c2idx in 0..256 {
//                let e = expected[c1idx * 256 + c2idx];
////                println!("{} {:x}", c1idx * 256 + c2idx, e);
//                if !is_mapped_color(c1idx) || !is_mapped_color(c2idx) {
//                    continue;
//                }
//                let c1 = pal.rgb15(c1idx as u8);
//                let c2 = pal.rgb15(c2idx as u8);
//                let a = pal.color_idx(c1.blend(c2, |c| pal.quantize(c)));
//
//                assert_eq!(a, e,
//                    "{} {:?} {} {:?}", c1idx, c1, c2idx, c2);
//            }
//        }
//    }


//    println!("{:?}", Rgb15::with_colors(31, 31, 31).darken(127).colors());

//    let tile_tex = read_frm(&mut File::open("../../devp/vault13/misc/reveng/render/misc/BRICK30.FRM").unwrap()).unwrap().remove(0);
//    write_tex_png("/tmp/v13_tex.png", &tex, 0, &pal);

//    let shelf_tex = read_frm(&mut File::open("../../devp/vault13/misc/reveng/render/misc/BEDMILT1.FRM").unwrap()).unwrap().remove(0);
//    let egg_frm = read_frm(&mut File::open("../../devp/vault13/misc/reveng/render/misc/EGG.FRM").unwrap()).unwrap().remove(0);
//    write_tex_png("/tmp/egg.png", &egg_tex, &pal);

//    let mut r = Render::new(640, 480, Box::new(pal));
//    r.draw_light_mapped(&tile_tex, 100, 100, &[0x10000, 0, 0, 0x10000, 0x10000, 0, 0x10000, 0, 0x10000, 0]);
//    r.draw_masked(&shelf_tex, 100, 100, &egg_tex, 80, 80, 0x8000);

//    write_light_map_png("/tmp/actual_lm.png", &r.light_map);
//    let expected_light_map = read_light_map("../../devp/vault13/misc/reveng/render/misc/expected_light_map.bin").unwrap();
//    write_light_map_png("/tmp/expected_lm.png", &expected_light_map);

//    write_tex_png("/tmp/v13_tex.png", &r.back_buf, &r.palette);
//
//    for y in 0..LightMap::HEIGHT {
//        for x in 0..LightMap::WIDTH {
//            let i = (y * LightMap::HEIGHT + x) as usize;
//            let expected = expected_light_map.data[i];
////            let expected = cmp::min(cmp::max(expected, 0), 0x10000) as u32;
//            assert_eq!(r.light_map.data[i], expected, "{} {}", x, y);
//        }
//    }
}

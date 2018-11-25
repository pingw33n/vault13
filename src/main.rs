#![allow(proc_macro_derive_resolution_fallback)]
#![allow(unused)]
#![deny(non_snake_case)]

extern crate bstring;
extern crate byteorder;
extern crate enumflags;
#[macro_use] extern crate enumflags_derive;
#[macro_use] extern crate enum_map;
#[macro_use] extern crate enum_primitive_derive;
#[macro_use] extern crate icecream;
#[macro_use] extern crate if_chain;
extern crate flate2;
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



pub fn load_map(rd: &mut impl Read, proto_db: &ProtoDb) -> io::Result<(Box<[(u16, u16)]>, i32)> {
    // header

    let version = rd.read_u32::<BigEndian>()?;

    let mut name = [0; 16];
    rd.read_exact(&mut name[..]).unwrap();

    let entrance_pos = rd.read_i32::<BigEndian>()?;
     println!("entrance_pos={}", entrance_pos);
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

    let mut elevation_tiles: Vec<Option<_>> = Vec::with_capacity(MAX_ELEVATIONS);

    for i in 0..MAX_ELEVATIONS {
        if flags & (1 << (i as u32 + 1)) != 0 {
            println!("No {} elevation", i);
            elevation_tiles.push(None);
            continue;
        }
        let mut tiles = Vec::with_capacity(10000);
        for _ in 0..tiles.capacity() {
            let roof_id = rd.read_u16::<BigEndian>()?;
            let floor_id = rd.read_u16::<BigEndian>()?;
            tiles.push((floor_id, roof_id));
        }
        elevation_tiles.push(Some(tiles));
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
    if true {
        for _ in 0..MAX_ELEVATIONS {
            let obj_count = rd.read_u32::<BigEndian>()?;
            println!("obj_count={}", obj_count);

            for _ in 0..obj_count {
                let obj = read_obj(rd, proto_db, version != 19)?;
            }
        }
    }

    Ok((elevation_tiles[entrance_elevation].take().unwrap().into(), entrance_pos))
}

fn read_obj(rd: &mut impl Read, proto_db: &ProtoDb, f2: bool) -> io::Result<()> {
    fn read_without_inventory(rd: &mut impl Read, proto_db: &ProtoDb, f2: bool) -> io::Result<i32> {
        let id = rd.read_u32::<BigEndian>()?;
        let pos = rd.read_u32::<BigEndian>()?;
        let x = rd.read_i32::<BigEndian>()?;
        let y = rd.read_i32::<BigEndian>()?;
        let sx = rd.read_i32::<BigEndian>()?;
        let sy = rd.read_i32::<BigEndian>()?;
        let frm_idx = rd.read_i32::<BigEndian>()?;
        let direction = Direction::from_u32(rd.read_u32::<BigEndian>()?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid object direction"));
        let fid = rd.read_u32::<BigEndian>()?;
        let flags = rd.read_u32::<BigEndian>()?;
        let elevation = rd.read_u32::<BigEndian>()?;
        let pid = Pid::from_packed(rd.read_u32::<BigEndian>()?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid object Pid"))?;
        let cid = rd.read_u32::<BigEndian>()?;
        let light_radius = rd.read_i32::<BigEndian>()?;
        let light_intensity = rd.read_i32::<BigEndian>()?;
        let _outline_color = rd.read_u32::<BigEndian>()?;
        let sid = rd.read_u32::<BigEndian>()?;
        let script_idx = rd.read_u32::<BigEndian>()?;

        // proto update data

        let inventory_len = rd.read_i32::<BigEndian>()?;
        let inventory_max = rd.read_i32::<BigEndian>()?;
        let _unused = rd.read_u32::<BigEndian>()?;

        let updated_flags = rd.read_u32::<BigEndian>()?;

        ic!(id);
        ic!(pid);

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
            let update_flags = if updated_flags == 0xcccccccc {
                0
            } else {
                updated_flags
            };
            println!("pid {:?}", pid);
            match pid.kind() {
                EntityKind::Item => {
                    let kind = proto_db.kind(pid).item().unwrap();
                    match kind {
                        ItemKind::Weapon => {
                            let charges = rd.read_i32::<BigEndian>()?;
                            let ammo_pid = rd.read_u32::<BigEndian>()?;
                        }
                        ItemKind::Ammo => {
                            let charges = rd.read_i32::<BigEndian>()?;
                        }
                        ItemKind::Misc => {
                            let charges = rd.read_i32::<BigEndian>()?;
                        }
                        ItemKind::Key => {
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
                        let map_id = rd.read_u32::<BigEndian>()?;
                        let dude_pos = rd.read_u32::<BigEndian>()?;
                        let elevation = rd.read_u32::<BigEndian>()?;
                        let direction = Direction::from_u32(rd.read_u32::<BigEndian>()?)
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid exit direction"));
                    }
                }
                _ => {}
            }
        }

        if pid.is_exit_area() {
            /* TODO  if charges <= 0
          {
            v7 = obj->art_fid & 0xFFF;
            if ( v7 < 33 )
              obj->art_fid = art_id_(OBJ_TYPE_MISC, v7 + 16, (obj->art_fid & 0xFF0000) >> 16, 0);
          }*/
        } else if pid.kind() == EntityKind::Item && flags & 1 == 0 {
            // object_fix_weapon_ammo_
            match proto_db.kind(pid) {
                ExactEntityKind::Item(ItemKind::Weapon) => {
                    //                if charges == 0xcccccccc || charges == -1 {
//                                // object_fix_weapon_ammo()
//                                // TODO
//                                // obj_->_.item.charges = proto->_.item._.weapon.maxAmmo;
//                            }
//                            let ammo_pid = if ammo_pid == 0xcccccccc || ammo_pid == 0xffffffff {
//                                // object_fix_weapon_ammo()
//                                // TODO
//                                // obj_->_.item.ammoPid = proto->_.item._.weapon.ammoPid;
//                            }
//                            let ammo_pid = Self::from_packed(ammo_pid)
//                                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "malformed ResourceId"));
                    unimplemented!();
                }
                ExactEntityKind::Item(ItemKind::Misc) => {
                     // TODO
                        // max_charges = proto->_.item._.misc.maxCharges;
//                            if max_charges == 0xCCCCCCCC
//        {
//          name = proto_name_(obj->_.pid);
//          debug_printf_(aErrorMiscItemP, name);
//          obj_->_.item.charges = 0;
//        }
//                            let charges = if charges == 0xcccccccc {
//                                 obj_->_.item.charges = max_charges;
//                            } else if charges != max_charges {
//                                obj_->_.item.charges = max_charges;
//                            };
                    unimplemented!();
                }
                _ => {}
            }
        }

        Ok(inventory_len)
    }

    let inventory_len = read_without_inventory(rd, proto_db, f2)?;

    // inventory

    assert!(inventory_len == 0);
    // TODO

    Ok(())
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

fn main() {
    let master_dat = "/Users/dlysai/Dropbox/f2/MASTER.DAT";
    let critter_dat = "/Users/dlysai/Dropbox/f2/CRITTER.DAT";
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

        let kind = EntityKind::Critter;
        for i in 1..proto_db.len(kind)+1 {
            let pid = Pid::new(kind, i as u32);
            println!("{:?}", proto_db.name(pid));
            println!("{:?}", proto_db.description(pid));
            if let Ok(proto) = proto_db.proto(pid) {
//                let path = frm_db.name(proto.fid).unwrap();
//                println!("{:?} {:?} {:?}", proto.fid, path, fs.metadata(&path).unwrap());
//                let frm = frm_db.get_or_load(proto.fid, render);
//                println!("{} {:#?}", i, frm);
                println!("{:?}", proto.fid);
                for fid in all_fids(proto.fid) {
                    if frm_db.exists(fid) {
                        println!("  {:?}", fid);
                        println!("  {:?}", frm_db.name(fid));
                    }
                }

            } else {
                println!("{:?}", proto_db.proto(pid));
            }
    //        println!("{:#?}", proto_db.proto(pid));
            println!();
        }
        return;

        let tiles_lst = read_lst(&mut fs.reader("art/tiles/tiles.lst").unwrap()).unwrap();
        let (map_tiles, player_pos) = load_map(&mut fs.reader("maps/artemple.map").unwrap(), &proto_db).unwrap();

        let mut map_tiles_tex: HashMap<u16, TextureHandle> = HashMap::new();
        for &(floor, roof) in &map_tiles[..] {
            for &tile_id in &[floor, roof] {
                match map_tiles_tex.entry(tile_id) {
                    Entry::Vacant(v) => {
                        let path = format!("art/tiles/{}", &tiles_lst[tile_id as usize].fields[0]);
    //                    println!("loading {}", path);
                        if let Ok(ref mut reader) = fs.reader(&path) {
                            let tex = read_frm_(render, reader).unwrap();
                            v.insert(tex[0].clone());
                        } else {
                            println!("Couldn't read: {} {}", tile_id, path);
                        }
                    }
                    Entry::Occupied(_) => {}
                }
            }
        }

//        let hmjpmsaa_tex = read_frm_(render, &mut fs.reader("art/critters/HMJMPSAA.FRM").unwrap()).unwrap().remove(0);
//        let tile_tex = read_frm_(render, &mut fs.reader("art/tiles/slime05.FRM").unwrap()).unwrap().remove(0);
        let tile_tex = read_frm_(render, &mut fs.reader("art/tiles/shore08.FRM").unwrap()).unwrap().remove(0);
        let barrel_tex = read_frm_(render, &mut fs.reader("art/scenery/barrel.FRM").unwrap()).unwrap().remove(0);
//        render.draw_light_mapped(&tile_tex, 100, 100, &[0x10000, 0, 0, 0x10000, 0x10000, 0, 0x10000, 0, 0x10000, 0]);
//        render.draw(&tile_tex, 100, 100, 0x10000);
//        render.draw_masked(&shelf_tex, 100, 100, &egg_tex, 80, 80, 0x8000);
//        render.draw(&shelf_tex, 200, 200, 0x5000);

//        let mut start_x = 600;
//        let mut start_y = -12;
//        while start_y < 480 {
//            let mut x = start_x;
//            let mut y = start_y;
//            while x > -80 {
//                render.draw(&tile_tex, x, y, 0x10000);
////                    render.draw_multi_light(&tile_tex, x, y, &[0x10000, 0, 0, 0x10000, 0x10000, 0, 0x10000, 0, 0x10000, 0]);
//                x -= 48;
//                y += 12;
//            }
//            start_x += 32;
//            start_y += 24;
//        }

//        render.draw_translucent_dark(&hmjpmsaa_tex, 200, 200, TRANS_WALL, 0x10000);
//        render.draw_translucent(&hmjpmsaa_tex, 300, 300, TRANS_WALL, 0x5000);
//        render.draw(&hmjpmsaa_tex, 350, 300, 0x10000);
//        render.draw(&barrel_tex, 400, 300, 0x10000);

        let mut htg = hex::TileGrid::default();
        let mut stg = sqr::TileGrid::default();
        let player_pos = htg.from_linear(player_pos);
        let p = stg.from_screen(htg.to_screen(player_pos));
        map::center(&mut htg, &mut stg, player_pos, 640, 380);
//        stg.set_pos(p);
//        stg.set_screen_pos((245, 160));
//        stg.set_pos((40, 45));

        render_floor(render, &stg, Rect::with_size(0, 0, 640, 380),
            |num| map_tiles_tex.get(&map_tiles[num as usize].0));
        render_roof(render, &stg, Rect::with_size(0, 0, 640, 380),
            |num| map_tiles_tex.get(&map_tiles[num as usize].1));

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

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }

            let now = Instant::now();
            render.update(now);
            render.present();

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
//        File::open("/Users/dlysai/devp/vault13/misc/reveng/render/misc/expected_darken_lighten_lut.raw").unwrap()
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
//        File::open("/Users/dlysai/devp/vault13/misc/reveng/render/misc/expected_rgb15_add_lut.raw").unwrap()
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

//    let tile_tex = read_frm(&mut File::open("/Users/dlysai/devp/vault13/misc/reveng/render/misc/BRICK30.FRM").unwrap()).unwrap().remove(0);
//    write_tex_png("/tmp/v13_tex.png", &tex, 0, &pal);

//    let shelf_tex = read_frm(&mut File::open("/Users/dlysai/devp/vault13/misc/reveng/render/misc/BEDMILT1.FRM").unwrap()).unwrap().remove(0);
//    let egg_tex = read_frm(&mut File::open("/Users/dlysai/devp/vault13/misc/reveng/render/misc/EGG.FRM").unwrap()).unwrap().remove(0);
//    write_tex_png("/tmp/egg.png", &egg_tex, &pal);

//    let mut r = Render::new(640, 480, Box::new(pal));
//    r.draw_light_mapped(&tile_tex, 100, 100, &[0x10000, 0, 0, 0x10000, 0x10000, 0, 0x10000, 0, 0x10000, 0]);
//    r.draw_masked(&shelf_tex, 100, 100, &egg_tex, 80, 80, 0x8000);

//    write_light_map_png("/tmp/actual_lm.png", &r.light_map);
//    let expected_light_map = read_light_map("/Users/dlysai/devp/vault13/misc/reveng/render/misc/expected_light_map.bin").unwrap();
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

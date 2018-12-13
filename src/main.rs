#![allow(dead_code)]
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(non_snake_case)]

extern crate bit_vec;
extern crate bstring;
extern crate byteorder;
extern crate enumflags;
extern crate env_logger;
#[macro_use] extern crate enumflags_derive;
#[macro_use] extern crate enum_map;
#[macro_use] extern crate enum_primitive_derive;
extern crate flate2;
#[macro_use] extern crate icecream;
#[macro_use] extern crate log;
extern crate num_traits;
extern crate png;
extern crate sdl2;
extern crate slotmap;

mod asset;
mod fs;
mod game;
mod graphics;
mod util;

use std::rc::Rc;
use asset::*;
use asset::palette::read_palette;
use asset::proto::*;
use graphics::color::PaletteOverlay;
use graphics::geometry::map::{ELEVATION_COUNT, MapGrid};
use graphics::render::software::*;
use graphics::render::*;
use asset::map::*;
use asset::frm::*;
use game::object::*;
use enumflags::BitFlags;
use graphics::*;
use graphics::geometry::Direction;
use game::object::LightEmitter;
use game::object::Inventory;
use graphics::Rect;
use game::world::World;
use util::two_dim_array::Array2d;
use game::sequence::{Sequence, Sequencer};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use graphics::map::render_floor;
use std::cmp;
use graphics::ElevatedPoint;
use game::object::Egg;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use util::EnumExt;
use graphics::frm::OutlineStyle;
use graphics::map::render_roof;
use game::sequence::chain::Chain;
use game::sequence::cancellable::Cancel;
use game::sequence::impls::move_seq::Move;
use game::sequence::impls::stand::Stand;

fn main() {
    env_logger::init();

    let master_dat = "../../Dropbox/f2/MASTER.DAT";
    let critter_dat = "../../Dropbox/f2/CRITTER.DAT";
    let map_name = "newr1.map";

    let mut fs = fs::FileSystem::new();
    fs.register_provider(fs::dat::v2::new_provider(master_dat).unwrap());
    fs.register_provider(fs::dat::v2::new_provider(critter_dat).unwrap());
    let fs = Rc::new(fs);

    let ref proto_db = Rc::new(ProtoDb::new(fs.clone(), "english").unwrap());
    let frm_db = Rc::new(FrmDb::new(fs.clone(), "english").unwrap());
    let pal = read_palette(&mut fs.reader("color.pal").unwrap()).unwrap();

    let sdl = sdl2::init().unwrap();
    let mut event_pump = sdl.event_pump().unwrap();
    let video = sdl.video().unwrap();

    let window = video.window("Vault 13", 640, 480)
        .position_centered()
        .allow_highdpi()
        .build()
        .unwrap();
    let canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    let ref mut renderer = SoftwareRenderer::new(canvas, Box::new(pal.clone()), PaletteOverlay::standard());
    let texture_factory = renderer.new_texture_factory();

    let map_grid = MapGrid::new(640, 380);

    let mut objects = Objects::new(map_grid.hex().clone(), ELEVATION_COUNT, proto_db.clone(), frm_db.clone());

    let map = MapReader {
        reader: &mut fs.reader(&format!("maps/{}", map_name)).unwrap(),
        objects: &mut objects,
        proto_db: &proto_db,
        frm_db: &frm_db,
        tile_grid: map_grid.hex(),
        texture_factory: &texture_factory,
    }.read().unwrap();

    for elev in &map.sqr_tiles {
        if let Some(ref elev) = elev {
            for &(floor, roof) in elev {
                frm_db.get_or_load(Fid::new_generic(EntityKind::SqrTile, floor).unwrap(), &texture_factory).unwrap();
                frm_db.get_or_load(Fid::new_generic(EntityKind::SqrTile, roof).unwrap(), &texture_factory).unwrap();
            }
        }
    }

    let mut ambient_light = 0x6666;

    fn all_fids(fid: Fid) -> Vec<Fid> {
        let mut r = vec![fid];
        match fid.kind() {
            EntityKind::Critter => {
                for wk in WeaponKind::iter() {
                    for anim in CritterAnim::iter() {
                        r.push(Fid::new_critter(None, anim, wk, fid.id()).unwrap());
                        for direction in Direction::iter() {
                            r.push(Fid::new_critter(Some(direction), anim, wk, fid.id()).unwrap());
                        }
                    }
                }
            }
            _ => {}
        }
        r
    }

    let mut world = World::new(proto_db.clone(), frm_db.clone(), map_grid, Array2d::with_default(200, 200), objects);
    world.rebuild_light_grid();

    let dude_fid = Fid::from_packed(0x101600A).unwrap();
    for fid in all_fids(dude_fid) {
        let _ = frm_db.get_or_load(fid, &texture_factory);
    }
    let dude_objh = world.insert_object(Object::new(
        BitFlags::empty(),
        Some(map.entrance),
        Point::new(0, 0),
        Point::new(0, 0),
        dude_fid,
        Direction::NW,
        LightEmitter {
            intensity: 0x10000,
            radius: 4,
        },
        None,
        Inventory::new(),
        None,

    ));
    world.make_object_standing(&dude_objh);
    frm_db.get_or_load(Fid::EGG, &texture_factory).unwrap();


    frm_db.get_or_load(Fid::MOUSE_HEX2, &texture_factory).unwrap();
    let mouse_objh = world.insert_object(Object::new(
        BitFlags::from_bits(0xA000041C).unwrap(),
        Some(map.entrance),
        Point::new(0, 0),
        Point::new(0, 0),
        Fid::MOUSE_HEX2,
        Direction::NW,
        LightEmitter {
            intensity: 0,
            radius: 0,
        },
        None,
        Inventory::new(),
        Some(game::object::Outline {
            style: OutlineStyle::Red,
            translucent: true,
            disabled: false,
        }),
    ));

    world.map_grid_mut().center2(map.entrance.point);

    let visible_rect = Rect::with_size(0, 0, 640, 380);
    let scroll_inc = 10;
    let mut roof_visible = false;

    let mut sequencer = Sequencer::new();

    frm_db.get_or_load(Fid::MAIN_HUD, &texture_factory).unwrap();

    let (seq, dude_seq) = Chain::endless();
    sequencer.start(seq);
    let mut dude_seq_cancel: Option<Cancel> = None;

    'running: loop {
        let dude_pos = world.objects().get(&dude_objh).borrow().pos.unwrap();
        let elevation = dude_pos.elevation;
        for event in event_pump.poll_iter() {
            match event {
                Event::MouseMotion { x, y, .. } => {
                    let hex_pos = world.map_grid().hex().from_screen((x, y));
                    let sqr_pos = world.map_grid().sqr().from_screen((x, y));
                    renderer.canvas_mut().window_mut().set_title(&format!(
                        "hex pos: {}, {} ({}), sqr pos: {}, {} ({})",
                        hex_pos.x, hex_pos.y, world.map_grid().hex().to_linear_inv(hex_pos).unwrap_or(-1),
                        sqr_pos.x, sqr_pos.y, world.map_grid().sqr().to_linear_inv(sqr_pos).unwrap_or(-1))).unwrap();

                    world.set_object_pos(&mouse_objh, ElevatedPoint::new(elevation, hex_pos));
                }
                Event::MouseButtonUp { x, y, mouse_btn, .. } => {
                    if let Some(signal) = dude_seq_cancel.take() {
                        signal.cancel();
                    }

                    let to = world.map_grid().hex().from_screen((x, y));
                    if let Some(path) = world.path_for_object(&dude_objh, to, true) {
                        let anim = if mouse_btn == MouseButton::Left {
                            CritterAnim::Running
                        } else {
                            CritterAnim::Walk
                        };
                        if !path.is_empty() {
                            let (seq, signal) = Move::new(dude_objh.clone(), anim, path).cancellable();
                            dude_seq_cancel = Some(signal);
                            dude_seq.push(seq.then(Stand::new(dude_objh.clone())));
                        }
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    world.map_grid_mut().scroll((scroll_inc, 0));
                }
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    world.map_grid_mut().scroll((-scroll_inc, 0));
                }
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    world.map_grid_mut().scroll((0, -scroll_inc));
                }
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    world.map_grid_mut().scroll((0, scroll_inc));
                }
                Event::KeyDown { keycode: Some(Keycode::Comma), .. } => {
                    let mut obj = world.objects().get(&dude_objh).borrow_mut();
                    obj.direction = obj.direction.rotate_ccw();
                }
                Event::KeyDown { keycode: Some(Keycode::Period), .. } => {
                    let mut obj = world.objects().get(&dude_objh).borrow_mut();
                    obj.direction = obj.direction.rotate_cw();
                }
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    let mut obj = world.objects().get(&dude_objh).borrow_mut();
                    let mut new_elevation = obj.pos.unwrap().elevation + 1;
                    while new_elevation < map.sqr_tiles.len() && map.sqr_tiles[new_elevation].is_none() {
                        new_elevation += 1;
                    }
                    if new_elevation < map.sqr_tiles.len() && map.sqr_tiles[new_elevation].is_some() {
                        obj.pos.as_mut().unwrap().elevation = new_elevation;
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                    let mut obj = world.objects().get(&dude_objh).borrow_mut();
                    let mut new_elevation = obj.pos.unwrap().elevation as isize - 1;
                    while new_elevation >= 0 && map.sqr_tiles[new_elevation as usize].is_none() {
                        new_elevation -= 1;
                    }
                    if new_elevation >= 0 && map.sqr_tiles[new_elevation as usize].is_some() {
                        obj.pos.as_mut().unwrap().elevation = new_elevation as usize;
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::LeftBracket), .. } => {
                    ambient_light = cmp::max(ambient_light as i32 - 1000, 0) as u32;
                    renderer.canvas_mut().window_mut().set_title(&format!("ambient_light: {:x}", ambient_light)).unwrap();
                }
                Event::KeyDown { keycode: Some(Keycode::RightBracket), .. } => {
                    ambient_light = cmp::min(ambient_light + 1000, 0x10000);
                    renderer.canvas_mut().window_mut().set_title(&format!("ambient_light: {:x}", ambient_light)).unwrap();
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    roof_visible = !roof_visible;
                }
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        render_floor(renderer, world.map_grid().sqr(), &visible_rect,
            |num| {
                let fid = Fid::new_generic(EntityKind::SqrTile, map.sqr_tiles[elevation].as_ref().unwrap()[num as usize].0).unwrap();
                Some(frm_db.get(fid).frame_lists[Direction::NE].frames[0].texture.clone())
            },
            |point| {
                let l = world.light_grid().get_clipped(ElevatedPoint { elevation, point });
                cmp::max(l, ambient_light)
            }
        );

        let egg = Egg {
            pos: world.objects().get(&dude_objh).borrow().pos.unwrap().point,
            fid: Fid::EGG,
        };
        let egg = Some(&egg);
        world.objects().render(renderer, elevation, &visible_rect, world.map_grid().hex(), egg,
            |pos| if let Some(pos) = pos {
                cmp::max(world.light_grid().get_clipped(pos), ambient_light)
            } else {
                ambient_light
            });

        if roof_visible {
            render_roof(renderer, &world.map_grid().sqr(), &visible_rect,
                |num| Some(frm_db.get(Fid::new_generic(EntityKind::SqrTile,
                    map.sqr_tiles[elevation].as_ref().unwrap()[num as usize].1).unwrap()).first().texture.clone()));
        }

        world.objects().render_outlines(renderer, elevation, &visible_rect, world.map_grid().hex());

        // TODO render text.

        renderer.draw(&frm_db.get(Fid::MAIN_HUD).first().texture, 0, visible_rect.bottom, 0x10000);

        let now = Instant::now();

        sequencer.update(now, &mut world);

        renderer.update(now);

        renderer.present();
        renderer.cleanup();

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
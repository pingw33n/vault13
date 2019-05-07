#![allow(dead_code)]
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(non_snake_case)]

#[macro_use] mod macros;

mod asset;
mod fs;
mod game;
mod graphics;
mod sequence;
mod ui;
mod util;
mod vm;

use std::rc::Rc;
use crate::asset::*;
use crate::asset::palette::read_palette;
use crate::asset::proto::*;
use crate::graphics::color::PaletteOverlay;
use crate::graphics::geometry::map::{ELEVATION_COUNT, MapGrid};
use crate::graphics::render::software::*;
use crate::asset::map::*;
use crate::asset::frm::*;
use crate::game::object::*;
use enumflags::BitFlags;
use crate::graphics::*;
use crate::graphics::geometry::hex::Direction;
use crate::game::object::LightEmitter;
use crate::graphics::Rect;
use crate::game::world::World;
use crate::sequence::{Sequence, Sequencer};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use std::cmp;
use crate::graphics::EPoint;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use crate::util::EnumExt;
use crate::graphics::sprite::OutlineStyle;
use crate::game::sequence::move_seq::Move;
use crate::game::sequence::stand::Stand;
use crate::asset::font::load_fonts;
use crate::graphics::color::*;
use crate::graphics::font::*;
use asset::script::db::ScriptDb;
use crate::game::script::Scripts;
use crate::vm::{Vm, PredefinedProc};
use crate::game::script::ScriptKind;
use std::path::PathBuf;
use crate::game::START_GAME_TIME;
use crate::game::fidget::Fidget;
use crate::ui::{Ui, Cursor};
use log::*;
use crate::ui::message_panel::MessagePannel;

fn args() -> clap::App<'static, 'static> {
    use clap::*;

    App::new("Vault 13 Demo")
        .arg(Arg::with_name("RESOURCE_DIR")
            .help("Resource directory where master.dat, critter.dat and patch000.dat can be found")
            .required(true))
        .arg(Arg::with_name("MAP")
            .help("Map name to load. For example: artemple")
            .required(true))
        .after_help(
            "EXAMPLE:\n\
          \x20   vault13 /path/to/fallout2 artemple")
}

fn main() {
    env_logger::init();

    util::random::check_chi_square();

    let master_dat: PathBuf;
    let critter_dat: PathBuf;
    let patch_dat: PathBuf;
    let map_name: String;
    {
        let args = args().get_matches();

        let res_dir = args.value_of("RESOURCE_DIR").unwrap();
        master_dat = [res_dir, "master.dat"].iter().collect();
        critter_dat = [res_dir, "critter.dat"].iter().collect();
        patch_dat = [res_dir, "patch000.dat"].iter().collect();

        let s = args.value_of("MAP").unwrap().to_lowercase();
        map_name = if s.ends_with(".map") {
            s[..s.len() - 4].into()
        } else {
            s
        };
    }

    let mut fs = fs::FileSystem::new();
    fs.register_provider(fs::dat::v2::new_provider(patch_dat).unwrap());
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

    let mouse = sdl.mouse();
    mouse.set_relative_mouse_mode(true);

    let canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    let gfx_backend = Backend::new(canvas, Box::new(pal.clone()), PaletteOverlay::standard());
    let texture_factory = gfx_backend.new_texture_factory();

    let fonts = Rc::new(load_fonts(&fs, &texture_factory));

    let mut canvas = gfx_backend.into_canvas(fonts.clone());
    let canvas = canvas.as_mut();

    let map_grid = MapGrid::new(640, 380);

    let mut objects = Objects::new(map_grid.hex().clone(), ELEVATION_COUNT, proto_db.clone(), frm_db.clone());

    let mut scripts = Scripts::new(ScriptDb::new(fs.clone()).unwrap(), Vm::default());

    let map = MapReader {
        reader: &mut fs.reader(&format!("maps/{}.map", map_name)).unwrap(),
        objects: &mut objects,
        proto_db: &proto_db,
        frm_db: &frm_db,
        tile_grid: map_grid.hex(),
        texture_factory: &texture_factory,
        scripts: &mut scripts,

    }.read().unwrap();

    for elev in &map.sqr_tiles {
        if let Some(ref elev) = elev {
            for &(floor, roof) in elev.as_slice() {
                frm_db.get_or_load(Fid::new_generic(EntityKind::SqrTile, floor).unwrap(), &texture_factory).unwrap();
                frm_db.get_or_load(Fid::new_generic(EntityKind::SqrTile, roof).unwrap(), &texture_factory).unwrap();
            }
        }
    }

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

    let mut world = World::new(proto_db.clone(), frm_db.clone(), map_grid, map.sqr_tiles, objects);
    world.game_time = START_GAME_TIME;
    world.rebuild_light_grid();

    let dude_fid = Fid::from_packed(0x101600A).unwrap();
    let mut dude_obj = Object::new(dude_fid, None, Some(map.entrance));
    dude_obj.direction = Direction::NE;
    dude_obj.light_emitter = LightEmitter {
        intensity: 0x10000,
        radius: 4,
    };
    let dude_objh = world.insert_object(dude_obj);
    world.set_dude_obj(dude_objh);

    for obj in world.objects().iter() {
        for fid in all_fids(world.objects().get(obj).borrow().fid) {
            let _ = frm_db.get_or_load(fid, &texture_factory);
        }
    }

    world.make_object_standing(dude_objh);
    frm_db.get_or_load(Fid::EGG, &texture_factory).unwrap();

    world.map_grid_mut().center2(map.entrance.point);

    let mut sequencer = Sequencer::new();

    scripts.vars.global_vars = if map.savegame {
        unimplemented!("read save.dat")
    } else {
        asset::read_game_global_vars(&mut fs.reader("maps/vault13.gam").unwrap()).unwrap().into()
    };
    scripts.vars.map_vars = if map.savegame {
        map.map_vars.clone()
    } else {
        let path = format!("maps/{}.gam", map_name);
        if fs.exists(&path) {
            asset::read_game_global_vars(&mut fs.reader(&path).unwrap()).unwrap().into()
        } else {
            Vec::new().into()
        }
    };

    // Init scripts.
    {
        let ctx = &mut game::script::Context {
            world: &mut world,
            sequencer: &mut sequencer,
        };

        // PredefinedProc::Start for map script is never called.
        // MapEnter in map script is called before anything else.
        if let Some(sid) = scripts.map_sid() {
            scripts.execute_predefined_proc(sid, PredefinedProc::MapEnter, ctx);
        }

        scripts.execute_procs(PredefinedProc::Start, ctx, |sid| sid.kind() != ScriptKind::System);
        scripts.execute_map_procs(PredefinedProc::MapEnter, ctx);
    }

    let mut mouse_obj = Object::new(Fid::MOUSE_HEX_OUTLINE, None, Some(map.entrance));
    mouse_obj.flags = BitFlags::from_bits(0xA000041C).unwrap();
    mouse_obj.outline = Some(game::object::Outline {
        style: OutlineStyle::Red,
        translucent: true,
        disabled: false,
    });
    let mouse_objh = world.insert_object(mouse_obj);

    let visible_rect = Rect::with_size(0, 0, 640, 380);
    let scroll_inc = 10;
    let mut roof_visible = false;

    // Load all interface frame sets.
    for id in 0.. {
        let fid = Fid::new_generic(EntityKind::Interface, id).unwrap();
        if frm_db.name(fid).is_none() {
            break;
        }
        if let Err(e) = frm_db.get_or_load(fid, &texture_factory) {
            warn!("couldn't load interface frame set {:?}: {}", fid, e);
        }
    }

    let ui = &mut Ui::new(frm_db.clone());
    ui.cursor = ui::Cursor::Arrow;

    {
        use ui::button::Button;
        use graphics::sprite::Sprite;

        let main_hud = ui.new_window(Rect::with_size(0, 379, 640, 100), Some(Sprite::new(Fid::IFACE)));

        // Message panel.
        let message_panel = ui.new_widget(main_hud, Rect::with_size(23, 26, 165, 65), None, None,
            MessagePannel::new(fonts.clone(),
                FontKey::antialiased(1),
                Rgb15::new(0, 31, 0),
                100));

        {
            let mut mp = ui.widget(message_panel).borrow_mut();
            let mp = mp.downcast_mut::<MessagePannel>().unwrap();
            mp.push_message("You see a young man with bulging muscles and a very confident air about him.");
            mp.push_message("He looks Unhurt");
            mp.push_message("You see: Rocks.");
            mp.push_message("You see: Test 1.");
            mp.push_message("You see: Test 2.");
            mp.push_message("You see: Test 3.");
        }

        // Inventory button.
        // Original location is a bit off, at y=41.
        ui.new_widget(main_hud, Rect::with_size(211, 40, 32, 21), None, None,
            Button::new(Fid::INVENTORY_BUTTON_UP, Fid::INVENTORY_BUTTON_DOWN));

        // Options button.
        ui.new_widget(main_hud, Rect::with_size(210, 62, 34, 34), None, None,
            Button::new(Fid::OPTIONS_BUTTON_UP, Fid::OPTIONS_BUTTON_DOWN));

        // Single/burst switch button.
        ui.new_widget(main_hud, Rect::with_size(218, 6, 22, 21), None, None,
            Button::new(Fid::BIG_RED_BUTTON_UP, Fid::BIG_RED_BUTTON_DOWN));

        // Skilldex button.
        ui.new_widget(main_hud, Rect::with_size(523, 6, 22, 21), None, None,
            Button::new(Fid::BIG_RED_BUTTON_UP, Fid::BIG_RED_BUTTON_DOWN));

        // MAP button.
        ui.new_widget(main_hud, Rect::with_size(526, 40, 41, 19), None, None,
            Button::new(Fid::MAP_BUTTON_UP, Fid::MAP_BUTTON_DOWN));

        // CHA button.
        ui.new_widget(main_hud, Rect::with_size(526, 59, 41, 19), None, None,
            Button::new(Fid::CHARACTER_BUTTON_UP, Fid::CHARACTER_BUTTON_DOWN));

        // PIP button.
        ui.new_widget(main_hud, Rect::with_size(526, 78, 41, 19), None, None,
            Button::new(Fid::PIP_BUTTON_UP, Fid::PIP_BUTTON_DOWN));

        // Attack button.
        // FIXME this should be a custom button with overlay text images.
        ui.new_widget(main_hud, Rect::with_size(267, 26, 188, 67), None, None,
            Button::new(Fid::SINGLE_ATTACK_BUTTON_UP, Fid::SINGLE_ATTACK_BUTTON_DOWN));
    }

    let mut fidget = Fidget::new();

    let mut mouse_hex_pos = Point::new(0, 0);
    let mut mouse_sqr_pos = Point::new(0, 0);
    let mut draw_path_blocked = false;
    let mut draw_debug = true;
    'running: loop {
        let now = Instant::now();

        for event in event_pump.poll_iter() {
            let handled = ui.handle_input(now, &event);
            if !handled {
            match event {
                Event::MouseMotion { x, y, .. } => {
                    ui.cursor = Cursor::Hidden;
                    world.objects_mut().get(mouse_objh).borrow_mut().flags.remove(Flag::TurnedOff);
                    mouse_hex_pos = world.map_grid().hex().from_screen((x, y));
                    mouse_sqr_pos = world.map_grid().sqr().from_screen((x, y));
                    let new_pos = EPoint::new(world.elevation(), mouse_hex_pos);
                    world.set_object_pos(mouse_objh, new_pos);
                    draw_path_blocked = world.path_for_object(dude_objh, mouse_hex_pos, true).is_none();
                }
                Event::MouseButtonUp { x, y, mouse_btn, .. } => {
                    if let Some(signal) = world.objects().get(dude_objh).borrow_mut().sequence.take() {
                        signal.cancel();
                    }

                    let to = world.map_grid().hex().from_screen((x, y));
                    if let Some(path) = world.path_for_object(dude_objh, to, true) {
                        let anim = if mouse_btn == MouseButton::Left {
                            CritterAnim::Running
                        } else {
                            CritterAnim::Walk
                        };
                        if !path.is_empty() {
                            let (seq, signal) = Move::new(dude_objh, anim, path).cancellable();
                            world.objects().get(dude_objh).borrow_mut().sequence = Some(signal);
                            sequencer.start(seq.then(Stand::new(dude_objh)));
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
                    let mut obj = world.objects().get(dude_objh).borrow_mut();
                    obj.direction = obj.direction.rotate_ccw();
                }
                Event::KeyDown { keycode: Some(Keycode::Period), .. } => {
                    let mut obj = world.objects().get(dude_objh).borrow_mut();
                    obj.direction = obj.direction.rotate_cw();
                }
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    let new_pos = {
                        let obj = world.objects().get(dude_objh).borrow_mut();
                        let mut new_pos = obj.pos.unwrap();
                        new_pos.elevation += 1;
                        while new_pos.elevation < ELEVATION_COUNT && !world.has_elevation(new_pos.elevation) {
                            new_pos.elevation += 1;
                        }
                        new_pos
                    };
                    if new_pos.elevation < ELEVATION_COUNT && world.has_elevation(new_pos.elevation) {
                        world.objects_mut().set_pos(dude_objh, new_pos);
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                    let new_pos = {
                        let obj = world.objects().get(dude_objh).borrow_mut();
                        let mut new_pos = obj.pos.unwrap();
                        if new_pos.elevation > 0 {
                            new_pos.elevation -= 1;
                            while new_pos.elevation > 0 && !world.has_elevation(new_pos.elevation) {
                                new_pos.elevation -= 1;
                            }
                        }
                        new_pos
                    };
                    if world.has_elevation(new_pos.elevation) {
                        world.objects_mut().set_pos(dude_objh, new_pos);
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::LeftBracket), .. } => {
                    world.ambient_light = cmp::max(world.ambient_light as i32 - 1000, 0) as u32;
                }
                Event::KeyDown { keycode: Some(Keycode::RightBracket), .. } => {
                    world.ambient_light = cmp::min(world.ambient_light + 1000, 0x10000);
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    roof_visible = !roof_visible;
                }
                Event::KeyDown { keycode: Some(Keycode::Backquote), .. } => {
                    draw_debug = !draw_debug;
                }
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
            } else {
                ui.cursor = Cursor::Arrow;
                world.objects_mut().get(mouse_objh).borrow_mut().flags.insert(Flag::TurnedOff);
            }
        }

        ui.update(now);

        world.render(canvas, &visible_rect, roof_visible);

        if draw_path_blocked {
            let center = world.map_grid().hex().to_screen(mouse_hex_pos) + Point::new(16, 8);
            canvas.draw_text(b"X".as_ref().into(), center.x, center.y, FontKey::antialiased(1),
                RED, &DrawOptions {
                    horz_align: HorzAlign::Center,
                    vert_align: VertAlign::Middle,
                    dst_color: Some(BLACK),
                    outline: Some(graphics::render::Outline::Fixed { color: BLACK, trans_color: None }),
                    .. Default::default()
                });
        }

        ui.render(canvas);

        if draw_debug {
            let dude_pos = world.objects().get(dude_objh).borrow().pos.unwrap().point;
            let ref msg = format!(
                "mouse hex: {}, {} ({})\n\
                 mouse sqr: {}, {} ({})\n\
                 dude hex: {}, {} ({})\n\
                 ambient: 0x{:x}",
                mouse_hex_pos.x, mouse_hex_pos.y,
                world.map_grid().hex().to_linear_inv(mouse_hex_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                mouse_sqr_pos.x, mouse_sqr_pos.y,
                world.map_grid().sqr().to_linear_inv(mouse_sqr_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                dude_pos.x, dude_pos.y,
                world.map_grid().hex().to_linear_inv(dude_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                world.ambient_light,
            );
            canvas.draw_text(msg.as_bytes().into(), 2, 1, FontKey::antialiased(1), Rgb15::new(0, 31, 0),
                &DrawOptions {
                    dst_color: Some(BLACK),
                    outline: Some(graphics::render::Outline::Fixed { color: BLACK, trans_color: None }),
                    .. Default::default()
                });
        }

        sequencer.update(&mut sequence::Context {
            time: now,
            world: &mut world
        });

        fidget.update(now, &mut world, &visible_rect, &mut sequencer);

        canvas.update(now);

        canvas.present();
        canvas.cleanup();

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
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
use crate::asset::map::ELEVATION_COUNT;
use crate::asset::palette::read_palette;
use crate::asset::proto::*;
use crate::graphics::color::PaletteOverlay;
use crate::graphics::render::software::*;
use crate::asset::map::*;
use crate::asset::frame::*;
use crate::game::object::*;
use crate::graphics::*;
use crate::graphics::geometry::hex::Direction;
use crate::game::object::LightEmitter;
use crate::graphics::Rect;
use crate::game::world::World;
use crate::sequence::{Sequence, Sequencer};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::cmp;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use crate::util::EnumExt;
use crate::game::sequence::move_seq::Move;
use crate::game::sequence::stand::Stand;
use crate::asset::font::load_fonts;
use crate::graphics::color::*;
use crate::graphics::font::*;
use asset::script::db::ScriptDb;
use crate::game::script::Scripts;
use crate::vm::{Vm, PredefinedProc};
use crate::game::script::ScriptKind;
use std::path::{Path, PathBuf};
use crate::game::START_GAME_TIME;
use crate::game::fidget::Fidget;
use crate::ui::Ui;
use log::*;
use crate::ui::message_panel::MessagePannel;
use clap::ArgMatches;
use bstring::BString;
use measure_time::*;
use crate::graphics::geometry::{TileGridView, hex, sqr};
use std::cell::RefCell;
use crate::game::ui::playfield::{Playfield, HexCursorStyle, ObjectActionIcon};
use crate::ui::out::OutEventData;

fn args() -> clap::App<'static, 'static> {
    use clap::*;

    App::new("Vault 13 Demo")
        .arg(Arg::with_name("RESOURCE_DIR")
            .help("One or more resource directories where master.dat, critter.dat and patchXXX.dat \
                   can be found")
            .required(true))
        .arg(Arg::with_name("MAP")
            .help("Map name to load. For example: artemple")
            .required(true))
        .after_help(
            "EXAMPLE:\n\
          \x20   vault13 /path/to/fallout2 artemple")
}

fn setup_file_system(fs: &mut fs::FileSystem, args: &ArgMatches) {
    let res_dir = Path::new(args.value_of("RESOURCE_DIR").unwrap());
    info!("Using resources dir: {}", res_dir.display());

    let mut dat_files = Vec::new();

    // Add patchXXX.dat files.
    for i in 0..999 {
        let file = format!("patch{:03}.dat", i);
        let path: PathBuf = [res_dir, Path::new(&file)].iter().collect();
        if path.is_file() {
            info!("Found {}", file);
            dat_files.push(path)
        } else {
            break;
        }
    }
    dat_files.reverse();

    for file in &["master.dat", "critter.dat"] {
        let path: PathBuf = [res_dir, Path::new(file)].iter().collect();
        if path.is_file() {
            info!("Found {}", file);
            dat_files.push(path);
        }
    }

    let data_dir: PathBuf = [res_dir, Path::new("data")].iter().collect();
    if data_dir.is_dir() {
        info!("Found `data` dir");
        fs.register_provider(fs::std::new_provider(data_dir).unwrap());
    }

    for dat_file in dat_files.iter().rev() {
        fs.register_provider(fs::dat::v2::new_provider(dat_file).unwrap());
    }
}

struct Timer {
    time: Instant,
    last: Instant,
}

impl Timer {
    pub fn new(time: Instant) -> Self {
        Self {
            time,
            last: time,
        }
    }

    pub fn time(&self) -> Instant {
        self.time
    }

    pub fn delta(&self) -> Duration {
        self.time - self.last
    }

    pub fn tick(&mut self, time: Instant) {
        assert!(time >= self.time);
        self.last = self.time;
        self.time = time;
    }
}

struct PausableTime {
    time: Instant,
    paused: bool,
}

impl PausableTime {
    pub fn new(time: Instant) -> Self {
        Self {
            time,
            paused: false,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn is_running(&self) -> bool {
        !self.is_paused()
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }

    pub fn update(&mut self, delta: Duration) {
        if !self.paused {
            self.time += delta;
        }
    }

    pub fn time(&self) -> Instant {
        self.time
    }
}

fn main() {
    env_logger::init();

    util::random::check_chi_square();

    let mut fs = fs::FileSystem::new();

    let map_name: String;
    {
        let args = &args().get_matches();
        setup_file_system(&mut fs, args);

        let s = args.value_of("MAP").unwrap().to_lowercase();
        map_name = if s.ends_with(".map") {
            s[..s.len() - 4].into()
        } else {
            s
        };
    }

    let fs = Rc::new(fs);

    let ref proto_db = Rc::new(ProtoDb::new(fs.clone(), "english").unwrap());
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

    let frm_db = Rc::new(FrameDb::new(fs.clone(), "english", texture_factory.clone()).unwrap());

    let fonts = Rc::new(load_fonts(&fs, &texture_factory));

    let mut canvas = gfx_backend.into_canvas(fonts.clone());
    let canvas = canvas.as_mut();

    let hex_grid = hex::TileGrid::default();
    let sqr_grid = sqr::TileGrid::default();

    let mut objects = Objects::new(hex_grid.clone(), ELEVATION_COUNT, proto_db.clone(), frm_db.clone());

    let mut scripts = Scripts::new(ScriptDb::new(fs.clone()).unwrap(), Vm::default());

    let map = MapReader {
        reader: &mut fs.reader(&format!("maps/{}.map", map_name)).unwrap(),
        objects: &mut objects,
        proto_db: &proto_db,
        frm_db: &frm_db,
        scripts: &mut scripts,

    }.read().unwrap();

    for elev in &map.sqr_tiles {
        if let Some(ref elev) = elev {
            for &(floor, roof) in elev.as_slice() {
                frm_db.get(FrameId::new_generic(EntityKind::SqrTile, floor).unwrap()).unwrap();
                frm_db.get(FrameId::new_generic(EntityKind::SqrTile, roof).unwrap()).unwrap();
            }
        }
    }

    fn for_each_direction(fid: FrameId, mut f: impl FnMut(FrameId)) {
        for direction in Direction::iter() {
            if let Some(fid) = fid.with_direction(Some(direction)) {
                f(fid);
            }
        }
    }

    let viewport = Rect::with_size(0, 0, 640, 380);
    let mut world = World::new(proto_db.clone(), frm_db.clone(), hex_grid.clone(), viewport,
        map.sqr_tiles, objects);
    world.game_time = START_GAME_TIME;
    world.rebuild_light_grid();

    let dude_fid = FrameId::from_packed(0x101600A).unwrap();
    let mut dude_obj = Object::new(dude_fid, None, Some(map.entrance));
    dude_obj.direction = Direction::NE;
    dude_obj.light_emitter = LightEmitter {
        intensity: 0x10000,
        radius: 4,
    };
    let dude_objh = world.insert_object(dude_obj);
    world.set_dude_obj(dude_objh);

    {
        debug_time!("preloading object FIDs");
        for obj in world.objects().iter() {
            for_each_direction(world.objects().get(obj).borrow().fid, |fid| {
                if let Err(e) = frm_db.get(fid) {
                    warn!("error loading {:?}: {:?}", fid, e);
                }
            });
        }
    }

    world.make_object_standing(dude_objh);
    frm_db.get(FrameId::EGG).unwrap();

    world.camera_mut().look_at(map.entrance.point);

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

    let world = Rc::new(RefCell::new(world));

    let scroll_inc = 10;

    // Load all interface frame sets.
    for id in 0.. {
        let fid = FrameId::new_generic(EntityKind::Interface, id).unwrap();
        if frm_db.name(fid).is_none() {
            break;
        }
        if let Err(e) = frm_db.get(fid) {
            warn!("couldn't load interface frame set {:?}: {}", fid, e);
        }
    }

    let ui = &mut Ui::new(frm_db.clone(), 640, 480);
    ui.set_cursor(ui::Cursor::Arrow);

    let playfield = {
        let rect = Rect::with_size(0, 0, 640, 379);
        let win = ui.new_window(rect.clone(), None);
        ui.new_widget(win, rect, None, None, Playfield::new(world.clone()))
    };

    let message_panel;
    {
        use ui::button::Button;
        use graphics::sprite::Sprite;

        let main_hud = ui.new_window(Rect::with_size(0, 379, 640, 100), Some(Sprite::new(FrameId::IFACE)));

        // Message panel.
        message_panel = ui.new_widget(main_hud, Rect::with_size(23, 26, 165, 65), None, None,
            MessagePannel::new(fonts.clone(),
                FontKey::antialiased(1),
                Rgb15::new(0, 31, 0),
                100));

        // Inventory button.
        // Original location is a bit off, at y=41.
        ui.new_widget(main_hud, Rect::with_size(211, 40, 32, 21), None, None,
            Button::new(FrameId::INVENTORY_BUTTON_UP, FrameId::INVENTORY_BUTTON_DOWN));

        // Options button.
        ui.new_widget(main_hud, Rect::with_size(210, 62, 34, 34), None, None,
            Button::new(FrameId::OPTIONS_BUTTON_UP, FrameId::OPTIONS_BUTTON_DOWN));

        // Single/burst switch button.
        ui.new_widget(main_hud, Rect::with_size(218, 6, 22, 21), None, None,
            Button::new(FrameId::BIG_RED_BUTTON_UP, FrameId::BIG_RED_BUTTON_DOWN));

        // Skilldex button.
        ui.new_widget(main_hud, Rect::with_size(523, 6, 22, 21), None, None,
            Button::new(FrameId::BIG_RED_BUTTON_UP, FrameId::BIG_RED_BUTTON_DOWN));

        // MAP button.
        ui.new_widget(main_hud, Rect::with_size(526, 40, 41, 19), None, None,
            Button::new(FrameId::MAP_BUTTON_UP, FrameId::MAP_BUTTON_DOWN));

        // CHA button.
        ui.new_widget(main_hud, Rect::with_size(526, 59, 41, 19), None, None,
            Button::new(FrameId::CHARACTER_BUTTON_UP, FrameId::CHARACTER_BUTTON_DOWN));

        // PIP button.
        ui.new_widget(main_hud, Rect::with_size(526, 78, 41, 19), None, None,
            Button::new(FrameId::PIP_BUTTON_UP, FrameId::PIP_BUTTON_DOWN));

        // Attack button.
        // FIXME this should be a custom button with overlay text images.
        ui.new_widget(main_hud, Rect::with_size(267, 26, 188, 67), None, None,
            Button::new(FrameId::SINGLE_ATTACK_BUTTON_UP, FrameId::SINGLE_ATTACK_BUTTON_DOWN));
    }

    let mut fidget = Fidget::new();

    let mut draw_debug = true;

    let mut shift_down = false;

    let ui_out_events = &mut Vec::new();

    let mut last_picked_obj = None;

    let start = Instant::now();
    let mut timer = Timer::new(start);
    let mut game_update_time = PausableTime::new(start);

    'running: loop {
        for event in event_pump.poll_iter() {
            let handled = ui.handle_input(ui::HandleInput {
                now: timer.time(),
                event: &event,
                out: ui_out_events,
            });
            if !handled {
                let mut world = world.borrow_mut();
                match event {
                    Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                        world.camera_mut().origin.x -= scroll_inc;
                    }
                    Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                        world.camera_mut().origin.x += scroll_inc;
                    }
                    Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                        world.camera_mut().origin.y += scroll_inc;
                    }
                    Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                        world.camera_mut().origin.y -= scroll_inc;
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
                        let mut pf = ui.widget_mut::<Playfield>(playfield);
                        pf.roof_visible = pf.roof_visible;
                    }
                    Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                        game_update_time.toggle();
                    }
                    Event::KeyDown { keycode: Some(Keycode::Backquote), .. } => {
                        draw_debug = !draw_debug;
                    }
                    Event::KeyDown { keycode: Some(Keycode::LShift), .. } |
                    Event::KeyDown { keycode: Some(Keycode::RShift), .. } => shift_down = true,
                    Event::KeyUp { keycode: Some(Keycode::LShift), .. } |
                    Event::KeyUp { keycode: Some(Keycode::RShift), .. } => shift_down = false,
                    Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }
        }

        ui.update(timer.time(), ui_out_events);

        for event in ui_out_events.drain(..) {
            match event.data {
                OutEventData::ObjectPick { action, obj: objh } => {
                    let world = world.borrow();
                    let picked_dude = Some(objh) == world.dude_obj();
                    if action {
                        if picked_dude {
                            let mut obj = world.objects().get(dude_objh).borrow_mut();
                            if let Some(signal) = obj.sequence.take() {
                                signal.cancel();
                            }
                            obj.direction = obj.direction.rotate_cw();
                        }
                    } else {
                        ui.widget_mut::<Playfield>(playfield).object_action_icon = Some(if picked_dude {
                            ObjectActionIcon::Rotate
                        } else {
                            ObjectActionIcon::Look
                        });

                        if last_picked_obj != Some(objh) {
                            last_picked_obj = Some(objh);

                            let obj = world.objects().get(objh).borrow();
                            let name: Option<BString> = if let Some(pid) = obj.pid {
                                if let Some(name) = proto_db.name(pid).unwrap() {
                                    Some(name.into())
                                } else {
                                    None
                                }
                            } else if picked_dude {
                                Some(b"[dude]"[..].into())
                            } else {
                                None
                            };

                            if let Some(name) = name {
                                let mut mp = ui.widget_mut::<MessagePannel>(message_panel);
                                let mut m = BString::new();
                                m.push_str("You see: ");
                                m.push_str(name);
                                mp.push_message(m);
                            }
                        }
                    }
                }
                OutEventData::HexPick { action, pos } => {
                    if action {
                        let world = world.borrow();
                        let dude_objh = world.dude_obj().unwrap();
                        if let Some(signal) = world.objects().get(dude_objh).borrow_mut().sequence.take() {
                            signal.cancel();
                        }

                        if let Some(path) = world.path_for_object(dude_objh, pos.point, true) {
                            let anim = if shift_down {
                                CritterAnim::Walk
                            } else {
                                CritterAnim::Running
                            };
                            if !path.is_empty() {
                                let (seq, signal) = Move::new(dude_objh, anim, path).cancellable();
                                world.objects().get(dude_objh).borrow_mut().sequence = Some(signal);
                                sequencer.start(seq.then(Stand::new(dude_objh)));
                            }
                        }
                    } else {
                        let mut pf = ui.widget_mut::<Playfield>(playfield);
                        pf.hex_cursor_style = if world.borrow().path_for_object(dude_objh, pos.point, true).is_some() {
                            HexCursorStyle::Normal
                        } else {
                            HexCursorStyle::Blocked
                        };
                    }
                }
                _ => {}
            }
        }

        ui.sync();

        if game_update_time.is_running() {
            let mut world = world.borrow_mut();
            sequencer.update(&mut sequence::Context {
                time: game_update_time.time(),
                world: &mut world
            });

            fidget.update(game_update_time.time(), &mut world, &mut sequencer);
        }

        canvas.update(timer.time());

        // Render

        canvas.clear(BLACK);

        ui.render(canvas);

        if draw_debug {
            let world = world.borrow();
            let pf = ui.widget_ref::<Playfield>(playfield);
            let (mouse_hex_pos, mouse_sqr_pos) = if let Some(EPoint { point, .. }) = pf.hex_cursor_pos() {
                (point, world.camera().sqr().from_screen(
                    world.camera().hex().to_screen(point) + Point::new(16, 8)))
            } else {
                (Point::new(-1, -1), Point::new(-1, -1))
            };
            let dude_pos = world.objects().get(dude_objh).borrow().pos.unwrap().point;
            let ref msg = format!(
                "mouse hex: {}, {} ({})\n\
                 mouse sqr: {}, {} ({})\n\
                 dude hex: {}, {} ({})\n\
                 ambient: 0x{:x}\n\
                 paused: {}",
                mouse_hex_pos.x, mouse_hex_pos.y,
                hex_grid.to_linear_inv(mouse_hex_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                mouse_sqr_pos.x, mouse_sqr_pos.y,
                sqr_grid.to_linear_inv(mouse_sqr_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                dude_pos.x, dude_pos.y,
                hex_grid.to_linear_inv(dude_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                world.ambient_light,
                game_update_time.is_paused(),
            );
            canvas.draw_text(msg.as_bytes().into(), 2, 1, FontKey::antialiased(1), Rgb15::new(0, 31, 0),
                &DrawOptions {
                    dst_color: Some(BLACK),
                    outline: Some(graphics::render::Outline::Fixed { color: BLACK, trans_color: None }),
                    .. Default::default()
                });
        }

        canvas.present();
        canvas.cleanup();

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));

        timer.tick(Instant::now());
        game_update_time.update(timer.delta());
    }
}
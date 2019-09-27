#![allow(dead_code)]
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(non_snake_case)]
#![deny(unused_must_use)]

#[macro_use] mod macros;

mod asset;
mod fs;
mod game;
mod graphics;
mod sequence;
mod state;
mod ui;
mod util;
mod vm;

use log::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Instant, Duration};

use crate::asset::EntityKind;
use crate::asset::font::load_fonts;
use crate::asset::frame::{FrameDb, FrameId};
use crate::asset::message::Messages;
use crate::asset::palette::read_palette;
use crate::asset::proto::ProtoDb;
use crate::game::state::GameState;
use crate::game::ui::world::WorldView;
use crate::graphics::{EPoint, Point};
use crate::graphics::color::{BLACK, GREEN};
use crate::graphics::color::palette::overlay::PaletteOverlay;
use crate::graphics::font::{self, FontKey};
use crate::graphics::geometry::TileGridView;
use crate::graphics::geometry::sqr;
use crate::graphics::render::software::Backend;
use crate::state::AppState;
use crate::ui::Ui;

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

fn setup_file_system(fs: &mut fs::FileSystem, args: &clap::ArgMatches) {
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

    let language = "english";

    let fs = Rc::new(fs);

    let ref proto_db = Rc::new(ProtoDb::new(fs.clone(), language).unwrap());

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

    let gfx_backend: Backend = Backend::new(canvas, Box::new(pal.clone()), PaletteOverlay::standard());
    let texture_factory = gfx_backend.new_texture_factory();

    let frm_db = Rc::new(FrameDb::new(fs.clone(), language, texture_factory.clone()).unwrap());

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

    let fonts = Rc::new(load_fonts(&fs, &texture_factory));

    let mut canvas = gfx_backend.into_canvas(fonts.clone());
    let canvas = canvas.as_mut();

    let start = Instant::now();
    let mut timer = Timer::new(start);

    let ui = &mut Ui::new(frm_db.clone(), fonts.clone(), 640, 480);
    ui.set_cursor(ui::Cursor::Arrow);
    ui.set_cursor_pos(Point::new(640 / 2, 480 / 2));

    let misc_msgs = Rc::new(Messages::read_file(&fs, language, "game/misc.msg").unwrap());
    let mut state = GameState::new(
        fs.clone(),
        language,
        proto_db.clone(),
        frm_db.clone(),
        fonts.clone(),
        misc_msgs,
        start,
        ui,
    );

    state.new_game(&map_name, "Narg".into(), ui);

    let mut draw_debug = true;

    let ui_commands = &mut Vec::new();

    'running: loop {
        // Handle input.

        for event in event_pump.poll_iter() {
            let mut handled = ui.handle_input(ui::HandleInput {
                now: timer.time(),
                event: &event,
                out: ui_commands,
            });
            if !handled {
                handled = state.handle_event(&event, ui);
            }
            if !handled {
                match event {
                    Event::KeyDown { keycode: Some(Keycode::Backquote), .. } => {
                        draw_debug = !draw_debug;
                    }
                    Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }
        }

        // Update.

        ui.update(timer.time(), ui_commands);

        for event in ui_commands.drain(..) {
            state.handle_ui_command(event, ui);
        }

        state.update(timer.delta(), ui);

        ui.sync();

        canvas.update(timer.time());

        // Render

        canvas.clear(BLACK);

        ui.render(canvas);

        if draw_debug {
            let world = state.world().borrow();
            let world_view = ui.widget_ref::<WorldView>(state.world_view());
            let (mouse_hex_pos, mouse_sqr_pos) = if let Some(EPoint { point, .. }) = world_view.hex_cursor_pos() {
                (point, world.camera().sqr().from_screen(
                    world.camera().hex().to_screen(point) + Point::new(16, 8)))
            } else {
                (Point::new(-1, -1), Point::new(-1, -1))
            };
            let dude_pos = world.objects().get(world.dude_obj().unwrap()).borrow().pos.unwrap().point;
            let ref msg = format!(
                "mouse: {}, {}\n\
                 mouse hex: {}, {} ({})\n\
                 mouse sqr: {}, {} ({})\n\
                 dude hex: {}, {} ({})\n\
                 ambient: 0x{:x}\n\
                 paused: {}",
                ui.cursor_pos().x, ui.cursor_pos().y,
                mouse_hex_pos.x, mouse_hex_pos.y,
                world.hex_grid().to_linear_inv(mouse_hex_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                mouse_sqr_pos.x, mouse_sqr_pos.y,
                sqr::TileGrid::default().to_linear_inv(mouse_sqr_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                dude_pos.x, dude_pos.y,
                world.hex_grid().to_linear_inv(dude_pos).map(|v| v.to_string()).unwrap_or_else(|| "N/A".into()),
                world.ambient_light,
                state.time().is_paused(),
            );
            canvas.draw_text(msg.as_bytes().into(), 2, 1, FontKey::antialiased(1), GREEN,
                &font::DrawOptions {
                    dst_color: Some(BLACK),
                    outline: Some(graphics::render::Outline::Fixed { color: BLACK, trans_color: None }),
                    .. Default::default()
                });
        }

        canvas.present();
        canvas.cleanup();

        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));

        timer.tick(Instant::now());
    }
}
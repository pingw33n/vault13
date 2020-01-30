use bstring::{bstr, BString};
use enum_map::{enum_map, EnumMap};
use if_chain::if_chain;
use log::*;
use measure_time::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::time::{Instant, Duration};

use crate::asset::{self, EntityKind, CritterAnim, ItemKind};
use crate::asset::frame::{FrameDb, FrameId};
use crate::asset::map::{MapReader, ELEVATION_COUNT};
use crate::asset::message::{BULLET, Messages};
use crate::asset::proto::{CritterFlag, ProtoDb};
use crate::asset::script::db::ScriptDb;
use crate::fs::FileSystem;
use crate::game::dialog::Dialog;
use crate::game::fidget::Fidget;
use crate::game::object::{self, LightEmitter, Object};
use crate::game::sequence::move_seq::Move;
use crate::game::sequence::stand::Stand;
use crate::game::script::{self, Scripts, ScriptKind};
use crate::game::ui::action_menu::{self, Action};
use crate::game::ui::hud;
use crate::game::ui::scroll_area::ScrollArea;
use crate::game::ui::world::{HexCursorStyle, WorldView};
use crate::game::world::{ScrollDirection, World};
use crate::graphics::Rect;
use crate::graphics::font::Fonts;
use crate::graphics::geometry::hex::{self, Direction};
use crate::sequence::{self, *};
use crate::sequence::event::PushEvent;
use crate::state::AppState;
use crate::ui::{self, Ui};
use crate::ui::command::{UiCommand, UiCommandData, ObjectPickKind};
use crate::ui::message_panel::MessagePanel;
use crate::util::{EnumExt, sprintf};
use crate::util::random::random;
use crate::vm::{Vm, PredefinedProc, Suspend};

const SCROLL_STEP: i32 = 10;

pub struct GameState {
    time: PausableTime,
    fs: Rc<FileSystem>,
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrameDb>,
    world: Rc<RefCell<World>>,
    scripts: Scripts,
    sequencer: Sequencer,
    fidget: Fidget,
    message_panel: ui::Handle,
    world_view: ui::Handle,
    dialog: Option<Dialog>,
    shift_key_down: bool,
    last_picked_obj: Option<object::Handle>,
    object_action_menu: Option<ObjectActionMenu>,
    user_paused: bool,
    map_id: Option<i32>,
    in_combat: bool,
    seq_events: Vec<sequence::Event>,
    misc_msgs: Rc<Messages>,
    scroll_areas: EnumMap<ScrollDirection, ui::Handle>,
}

impl GameState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fs: Rc<FileSystem>,
        language: &str,
        proto_db: Rc<ProtoDb>,
        frm_db: Rc<FrameDb>,
        fonts: Rc<Fonts>,
        misc_msgs: Rc<Messages>,
        now: Instant,
        ui: &mut Ui,
    ) -> Self {
        let time = PausableTime::new(now);

        let viewport = Rect::with_size(0, 0, 640, 380);
        let hex_grid = hex::TileGrid::default();

        let critter_names = Messages::read_file(&fs, language, "game/scrname.msg").unwrap();

        let scripts = Scripts::new(
            proto_db.clone(),
            ScriptDb::new(fs.clone(), language).unwrap(),
            Vm::default());
        let world = World::new(
            proto_db.clone(),
            frm_db.clone(),
            critter_names,
            hex_grid,
            viewport,
            now,
            fonts);
        let world = Rc::new(RefCell::new(world));
        let sequencer = Sequencer::new(now);
        let fidget = Fidget::new(now);

        let world_view_rect = Rect::with_size(0, 0, 640, 379);
        let world_view = {
            let win = ui.new_window(world_view_rect.clone(), None);
            ui.new_widget(win, world_view_rect, None, None, WorldView::new(world.clone()))
        };
        let message_panel = hud::create(ui);

        let scroll_areas = Self::create_scroll_areas(Rect::with_size(0, 0, 640, 480), ui);

        Self {
            time,
            fs,
            frm_db,
            proto_db,
            world,
            scripts,
            sequencer,
            fidget,
            message_panel,
            world_view,
            dialog: None,
            shift_key_down: false,
            last_picked_obj: None,
            object_action_menu: None,
            user_paused: false,
            map_id: None,
            in_combat: false,
            seq_events: Vec::new(),
            misc_msgs,
            scroll_areas,
        }
    }

    pub fn world(&self) -> &RefCell<World> {
        &self.world
    }

    pub fn world_view(&self) -> ui::Handle {
        self.world_view
    }

    pub fn time(&self) -> &PausableTime {
        &self.time
    }

    pub fn new_game(&mut self, map_name: &str, dude_name: &bstr, ui: &mut Ui) {
        self.world.borrow_mut().clear();
        // Reinsert the hex cursor. Needs `world` to be not borrowed.
        ui.widget_mut::<WorldView>(self.world_view).ensure_hex_cursor();

        let world = &mut self.world.borrow_mut();

        let map = MapReader {
            reader: &mut self.fs.reader(&format!("maps/{}.map", map_name)).unwrap(),
            objects: world.objects_mut(),
            proto_db: &self.proto_db,
            frm_db: &self.frm_db,
            scripts: &mut self.scripts,
        }.read().unwrap();

        self.map_id = Some(map.id);

        for elev in &map.sqr_tiles {
            if let Some(ref elev) = elev {
                for &(floor, roof) in elev.as_slice() {
                    self.frm_db.get(FrameId::new_generic(EntityKind::SqrTile, floor).unwrap()).unwrap();
                    self.frm_db.get(FrameId::new_generic(EntityKind::SqrTile, roof).unwrap()).unwrap();
                }
            } else {}
        }

        fn for_each_direction(fid: FrameId, mut f: impl FnMut(FrameId)) {
            for direction in Direction::iter() {
                if let Some(fid) = fid.with_direction(Some(direction)) {
                    f(fid);
                }
            }
        }
        {
            debug_time!("preloading object FIDs");
            for obj in world.objects().iter() {
                for_each_direction(world.objects().get(obj).borrow().fid, |fid| {
                    if let Err(e) = self.frm_db.get(fid) {
                        warn!("error preloading {:?}: {:?}", fid, e);
                    }
                });
            }
        }
        self.frm_db.get(FrameId::EGG).unwrap();

        world.set_sqr_tiles(map.sqr_tiles);
        world.rebuild_light_grid();

        let dude_fid = FrameId::from_packed(0x100003E).unwrap();
        //    let dude_fid = FrameId::from_packed(0x101600A).unwrap();
        let mut dude_obj = Object::new(dude_fid, Some(self.proto_db.dude()), Some(map.entrance));
        dude_obj.direction = Direction::NE;
        dude_obj.light_emitter = LightEmitter {
            intensity: 0x10000,
            radius: 4,
        };
        let dude_objh = world.insert_object(dude_obj);
        debug!("dude obj: {:?}", dude_objh);
        world.set_dude_obj(dude_objh);
        world.dude_name = dude_name.into();

        world.make_object_standing(dude_objh);

        world.camera_mut().look_at(map.entrance.point);

        self.scripts.vars.global_vars = if map.savegame {
            unimplemented!("read save.dat")
        } else {
            asset::read_game_global_vars(&mut self.fs.reader("data/vault13.gam").unwrap()).unwrap().into()
        };
        self.scripts.vars.map_vars = if map.savegame {
            map.map_vars.clone()
        } else {
            let path = format!("maps/{}.gam", map_name);
            if self.fs.exists(&path) {
                asset::read_map_global_vars(&mut self.fs.reader(&path).unwrap()).unwrap().into()
            } else {
                Vec::new().into()
            }
        };

        // Init scripts.
        {
            let ctx = &mut script::Context {
                world,
                sequencer: &mut self.sequencer,
                dialog: &mut self.dialog,
                message_panel: self.message_panel,
                ui,
                map_id: map.id,
            };

            // PredefinedProc::Start for map script is never called.
            // MapEnter in map script is called before anything else.
            if let Some(sid) = self.scripts.map_sid() {
                self.scripts.execute_predefined_proc(sid, PredefinedProc::MapEnter, ctx)
                    .map(|r| r.suspend.map(|_| panic!("can't suspend in MapEnter")));
            }

            self.scripts.execute_procs(PredefinedProc::Start, ctx, |sid| sid.kind() != ScriptKind::System);
            self.scripts.execute_map_procs(PredefinedProc::MapEnter, ctx);
        }
    }

    fn handle_action(
        &mut self,
        ui: &mut Ui,
        obj: object::Handle,
        action: Action,
    ) {
        match action {
            Action::Cancel => {},
            Action::Drop | Action::Unload => unreachable!(),
            Action::Inventory => {
                // TODO
            }
            Action::Look => {
                self.dude_examine_object(obj, ui);
            }
            Action::Push => {
                // TODO
            }
            Action::Rotate => {
                let world = self.world.borrow_mut();
                let mut obj = world.objects().get(obj).borrow_mut();
                if let Some(signal) = obj.sequence.take() {
                    signal.cancel();
                }
                obj.direction = obj.direction.rotate_cw();
            }
            Action::Talk => {
                let talker = self.world.borrow().dude_obj().unwrap();
                self.action_talk(talker, obj, ui);
            }
            Action::UseHand => {
                // TODO
            }
            Action::UseSkill => {
                // TODO
            }
        }
    }

    fn handle_seq_events(&mut self, ui: &mut Ui) {
        use sequence::Event::*;
        let mut events = std::mem::replace(&mut self.seq_events, Vec::new());
        for event in events.drain(..) {
            match event {
                ObjectMoved { obj, old_pos, new_pos } => {
                    dbg!((obj, old_pos, new_pos));
                }
                Talk { talker, talked } => {
                    self.talk(talker, talked, ui);
                }
                _ => {}
            }
        }
        std::mem::replace(&mut self.seq_events, events);
    }

    fn actions(&self, objh: object::Handle) -> Vec<Action> {
        let mut r = Vec::new();
        let world = self.world.borrow();
        let obj = world.objects().get(objh).borrow();
        match obj.kind() {
            EntityKind::Critter => {
                if Some(objh) == world.dude_obj() {
                    r.push(Action::Rotate);
                } else {
                    if world.objects().can_talk_to(objh) {
                        if !self.in_combat {
                            r.push(Action::Talk);
                        }
                    } else if !obj.proto.as_ref().unwrap().borrow()
                        .sub.critter().unwrap()
                        .flags.contains(CritterFlag::NoSteal)
                    {
                        r.push(Action::UseHand);
                    }
                    if world.objects().can_push(world.dude_obj().unwrap(), objh,
                        &self.scripts, self.in_combat)
                    {
                        r.push(Action::Push);
                    }
                }
                r.extend_from_slice(&[Action::Look, Action::Inventory, Action::UseSkill])
            }
            EntityKind::Item => {
                r.extend_from_slice(&[Action::UseHand, Action::Look]);
                if world.objects().item_kind(objh) == Some(ItemKind::Container) {
                    r.extend_from_slice(&[Action::UseSkill, Action::Inventory]);
                }
            }
            EntityKind::Scenery => {
                if world.objects().can_use(objh) {
                    r.push(Action::UseHand)
                }
                r.extend_from_slice(&[Action::Look, Action::Inventory, Action::UseSkill])
            }
            EntityKind::Wall => {
                r.push(Action::Look);
                if world.objects().can_use(objh) {
                    r.push(Action::UseHand)
                }
            }
            _ => {}
        }
        if !r.is_empty() {
            r.push(Action::Cancel)
        }
        r
    }

    fn look_at_object(&mut self, looker: object::Handle, looked: object::Handle, ui: &mut Ui)
        -> Option<BString>
    {
        let sid = {
            let world = self.world.borrow();
            let lookero = world.objects().get(looker).borrow();
            let lookedo = world.objects().get(looked).borrow();
            if lookero.sub.critter().map(|c| c.is_dead()).unwrap_or(true)
                // TODO This is only useful for mapper?
                || lookedo.kind() == EntityKind::SqrTile
                || lookedo.proto.is_none()
            {
                return None;
            }
            lookedo.script.map(|(v, _)| v)
        };

        if_chain! {
            if let Some(sid) = sid;
            if let Some(r) = self.scripts.execute_predefined_proc(sid, PredefinedProc::LookAt,
                &mut script::Context {
                    world: &mut self.world.borrow_mut(),
                    sequencer: &mut self.sequencer,
                    dialog: &mut self.dialog,
                    ui,
                    message_panel: self.message_panel,
                    map_id: self.map_id.unwrap(),
                });
            then {
                assert!(r.suspend.is_none(), "can't suspend");
                if r.script_overrides {
                    return None;
                }
            }
        }

        let world = self.world.borrow();
        let lookedo = world.objects().get(looked).borrow();
        let msg_id = if lookedo.sub.critter().map(|c| c.is_dead()).unwrap_or(false) {
            491 + random(0, 1)
        } else {
            490
        };
        if_chain! {
            if let Some(msg) = self.proto_db.messages().get(msg_id);
            if let Some(name) = world.object_name(looked);
            then {
                Some(sprintf(&msg.text, &[&*name]))
            } else {
                None
            }
        }
    }

    fn dude_look_at_object(&mut self, obj: object::Handle, ui: &mut Ui) {
        let dude_obj = self.world().borrow().dude_obj().unwrap();
        if let Some(msg) = self.look_at_object(dude_obj, obj, ui) {
            self.push_message(&msg, ui);
        }
    }

    // obj_examine_func()
    fn examine_object(&mut self, examiner: object::Handle, examined: object::Handle, ui: &mut Ui)
        -> Vec<BString>
    {
        let sid = {
            let world = self.world.borrow();
            let examinero = world.objects().get(examiner).borrow();
            let examinedo = world.objects().get(examined).borrow();
            if examinero.sub.critter().map(|c| c.is_dead()).unwrap_or(false)
                // TODO This is only useful for mapper?
                || examinedo.kind() == EntityKind::SqrTile
            {
                return Vec::new();
            }
            examinedo.script.map(|(v, _)| v)
        };

        let script_overrides = if_chain! {
            if let Some(sid) = sid;
            if let Some(r) = self.scripts.execute_predefined_proc(sid, PredefinedProc::Description,
                &mut script::Context {
                    world: &mut self.world.borrow_mut(),
                    sequencer: &mut self.sequencer,
                    dialog: &mut self.dialog,
                    ui,
                    message_panel: self.message_panel,
                    map_id: self.map_id.unwrap(),
                });
            then {
                assert!(r.suspend.is_none(), "can't suspend");
                r.script_overrides
            } else {
                false
            }
        };

        let mut r = Vec::new();

        if !script_overrides {
            let world = self.world.borrow();
            let examinedo = world.objects().get(examined).borrow();
            if !examinedo.sub.critter().map(|c| c.is_dead()).unwrap_or(false) {
                let descr = examinedo.proto.as_ref()
                    .and_then(|p| {
                        p.borrow().description()
                            .filter(|s| {
                                // Compare to "<None>".
                                s != &self.proto_db.messages().get(10).unwrap().text
                            })
                            .map(|s| s.to_owned())
                    })
                    .unwrap_or_else(|| self.proto_db.messages().get(493).unwrap().text.clone());
                r.push(descr);
            }
        }

        // TODO critter state/hp, weapon/ammo description, car info etc

        r
    }

    fn dude_examine_object(&mut self, obj: object::Handle, ui: &mut Ui) {
        let dude_obj = self.world().borrow().dude_obj().unwrap();
        for msg in self.examine_object(dude_obj, obj, ui) {
            self.push_message(&msg, ui);
        }
    }

    fn push_message(&self, msg: &bstr, ui: &mut Ui) {
        let mut mp = ui.widget_mut::<MessagePanel>(self.message_panel);
        let mut m = BString::new();
        m.push(BULLET);
        m.push_str(msg);
        mp.push_message(m);
    }

    // action_talk_to()
    fn action_talk(&mut self, talker: object::Handle, talked: object::Handle, ui: &mut Ui) {
        // TODO handle combat state

        {
            let world = self.world.borrow();
            let objs = world.objects();

            if objs.distance(talker, talked).unwrap() >= 9 || // TODO this value is different (12) in can_talk2()
                objs.is_shot_blocked(talker, talked)
            {
                // TODO original cancels only Walk/Run animation, is this important?
                objs.get(talker).borrow_mut().cancel_sequence();

                let dest = objs.get(talked).borrow().pos.unwrap().point;
                // TODO (move_to_object()) shorten the move path by 1 tile if the `talked` is MultiHex
                let (seq, cancel) = Move::new(talker, dest, CritterAnim::Running).cancellable();
                objs.get(talker).borrow_mut().sequence = Some(cancel);
                self.sequencer.start(seq
                    .then(Stand::new(talker))
                    .then(PushEvent::new(sequence::Event::Talk { talker, talked })));
                return;
            }
        }

        self.talk(talker, talked, ui);
    }

    // talk_to(), gdialogEnter()
    fn talk(&mut self, talker: object::Handle, talked: object::Handle, ui: &mut Ui) {
        if self.world.borrow().objects().can_talk_now(talker, talked) {
            let world = &mut self.world.borrow_mut();
            // TODO optimize this.
            for obj in world.objects().iter() {
                world.objects().get(obj).borrow_mut().cancel_sequence();
            }
            self.sequencer.cleanup(&mut sequence::Cleanup {
                world,
            });
            let script = world.objects().get(talked).borrow().script;
            if let Some((sid, _)) = script {
                match self.scripts.execute_predefined_proc(sid, PredefinedProc::Talk,
                    &mut script::Context {
                        world,
                        sequencer: &mut self.sequencer,
                        dialog: &mut self.dialog,
                        ui,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                    }).and_then(|r| r.suspend)
                    {
                        None | Some(Suspend::GsayEnd) => {}
                    }
            }
        } else {
            assert_eq!(talker, self.world.borrow().dude_obj().unwrap());
            let msg = &self.misc_msgs.get(2000).unwrap().text;
            self.push_message(&msg, ui);
        }
    }

    fn create_scroll_areas(rect: Rect, ui: &mut Ui) -> EnumMap<ScrollDirection, ui::Handle> {
        let mut new = |rect, cur, curx| {
            let win = ui.new_window(rect, None);
            ui.new_widget(win, Rect::with_size(0, 0, rect.width(), rect.height()), None, None,
                ScrollArea::new(cur, curx, Duration::from_millis(0), Duration::from_millis(30)))
        };
        use ui::Cursor::*;
        use ScrollDirection::*;
        enum_map! {
            N => new(
                Rect::new(rect.left + 1, rect.top, rect.right - 1, rect.top + 1),
                ScrollNorth, ScrollNorthX),
            NE => new(
                Rect::new(rect.right - 1, rect.top, rect.right, rect.top + 1),
                ScrollNorthEast, ScrollNorthEastX),
            E => new(
                Rect::new(rect.right - 1, rect.top + 1, rect.right, rect.bottom - 1),
                ScrollEast, ScrollEastX),
            SE => new(
                Rect::new(rect.right - 1, rect.bottom - 1, rect.right, rect.bottom),
                ScrollSouthEast, ScrollSouthEastX),
            S => new(
                Rect::new(rect.left + 1, rect.bottom - 1, rect.right - 1, rect.bottom),
                ScrollSouth, ScrollSouthX),
            SW => new(
                Rect::new(rect.left, rect.bottom - 1, rect.left + 1, rect.bottom),
                ScrollSouthWest, ScrollSouthWestX),
            W => new(
                Rect::new(rect.left, rect.top + 1, rect.left + 1, rect.bottom - 1),
                ScrollWest, ScrollWestX),
            NW => new(
                Rect::new(rect.left, rect.top, rect.left + 1, rect.top + 1),
                ScrollNorthWest, ScrollNorthWestX),
        }
    }
}

impl AppState for GameState {
    fn handle_event(&mut self, event: &Event, ui: &mut Ui) -> bool {
        let mut world = self.world.borrow_mut();
        match event {
            Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                world.scroll(ScrollDirection::E, 1);
            }
            Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                world.scroll(ScrollDirection::W, 1);
            }
            Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                world.scroll(ScrollDirection::N, 1);
            }
            Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                world.scroll(ScrollDirection::S, 1);
            }
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                let dude_obj = world.dude_obj().unwrap();
                let new_pos = {
                    let obj = world.objects().get(dude_obj).borrow_mut();
                    let mut new_pos = obj.pos.unwrap();
                    new_pos.elevation += 1;
                    while new_pos.elevation < ELEVATION_COUNT && !world.has_elevation(new_pos.elevation) {
                        new_pos.elevation += 1;
                    }
                    new_pos
                };
                if new_pos.elevation < ELEVATION_COUNT && world.has_elevation(new_pos.elevation) {
                    world.objects_mut().set_pos(dude_obj, new_pos);
                }
            }
            Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                let dude_obj = world.dude_obj().unwrap();
                let new_pos = {
                    let obj = world.objects().get(dude_obj).borrow_mut();
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
                    world.objects_mut().set_pos(dude_obj, new_pos);
                }
            }
            Event::KeyDown { keycode: Some(Keycode::LeftBracket), .. } => {
                world.ambient_light = cmp::max(world.ambient_light as i32 - 1000, 0) as u32;
            }
            Event::KeyDown { keycode: Some(Keycode::RightBracket), .. } => {
                world.ambient_light = cmp::min(world.ambient_light + 1000, 0x10000);
            }
            Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                let mut wv = ui.widget_mut::<WorldView>(self.world_view);
                wv.roof_visible = wv.roof_visible;
            }
            Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                self.user_paused = !self.user_paused;
            }

            Event::KeyDown { keycode: Some(Keycode::LShift), .. } |
            Event::KeyDown { keycode: Some(Keycode::RShift), .. } => self.shift_key_down = true,
            Event::KeyUp { keycode: Some(Keycode::LShift), .. } |
            Event::KeyUp { keycode: Some(Keycode::RShift), .. } => self.shift_key_down = false,
            _ => return false,
        }
        true
    }

    fn handle_ui_command(&mut self, command: UiCommand, ui: &mut Ui) {
        match command.data {
            UiCommandData::ObjectPick { kind, obj: objh } => {
                let actions = self.actions(objh);
                let default_action = actions.first().cloned();
                match kind {
                    ObjectPickKind::Hover => {
                        // TODO highlight item on Action::UseHand: gmouse_bk_process()

                        ui.widget_mut::<WorldView>(self.world_view).default_action_icon = if self.object_action_menu.is_none() {
                            default_action
                        }  else {
                            None
                        };

                        if self.last_picked_obj != Some(objh) {
                            self.last_picked_obj = Some(objh);
                            self.dude_look_at_object(objh, ui);
                        }
                    }
                    ObjectPickKind::ActionMenu => {
                        ui.widget_mut::<WorldView>(self.world_view).default_action_icon = None;

                        let world_view_win = ui.window_of(self.world_view).unwrap();
                        self.object_action_menu = Some(ObjectActionMenu {
                            menu: action_menu::show(actions, world_view_win, ui),
                            obj: objh,
                        });

                        self.time.set_paused(true);
                    }
                    ObjectPickKind::DefaultAction => if let Some(a) = default_action {
                        self.handle_action(ui, objh, a);
                    }
                }
            }
            UiCommandData::HexPick { action, pos } => {
                if action {
                    let world = self.world.borrow();
                    let dude_objh = world.dude_obj().unwrap();
                    if let Some(signal) = world.objects().get(dude_objh).borrow_mut().sequence.take() {
                        signal.cancel();
                    }

                    let anim = if self.shift_key_down {
                        CritterAnim::Walk
                    } else {
                        CritterAnim::Running
                    };
                    let (seq, signal) = Move::new(dude_objh, pos.point, anim).cancellable();
                    world.objects().get(dude_objh).borrow_mut().sequence = Some(signal);
                    self.sequencer.start(seq.then(Stand::new(dude_objh)));
                } else {
                    let mut wv = ui.widget_mut::<WorldView>(self.world_view);
                    let dude_obj = self.world.borrow().dude_obj().unwrap();
                    wv.hex_cursor_style = if self.world.borrow()
                        .path_for_object(dude_obj, pos.point, true, false).is_some()
                    {
                        HexCursorStyle::Normal
                    } else {
                        HexCursorStyle::Blocked
                    };
                }
            }
            UiCommandData::Action { action } => {
                let object_action = self.object_action_menu.take().unwrap();
                self.handle_action(ui, object_action.obj, action);
                action_menu::hide(object_action.menu, ui);
                self.time.set_paused(false);
            }
            UiCommandData::Pick { id } => {
                let (sid, proc_id) = {
                    let dialog = self.dialog.as_mut().unwrap();

                    assert!(dialog.is(command.source));
                    let proc_id = dialog.option(id).proc_id;
                    dialog.clear_options(ui);

                    (dialog.sid(), proc_id)
                };
                let finished = if let Some(proc_id) = proc_id {
                    self.scripts.execute_proc(sid, proc_id,
                        &mut script::Context {
                            ui,
                            world: &mut self.world.borrow_mut(),
                            sequencer: &mut self.sequencer,
                            dialog: &mut self.dialog,
                            message_panel: self.message_panel,
                            map_id: self.map_id.unwrap(),
                        }).assert_no_suspend();
                    // No dialog options means the dialog is finished.
                    self.dialog.as_ref().unwrap().is_empty()
                } else {
                    true
                };
                if finished {
                    self.scripts.resume(&mut script::Context {
                        ui,
                        world: &mut self.world.borrow_mut(),
                        sequencer: &mut self.sequencer,
                        dialog: &mut self.dialog,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                    }).assert_no_suspend();
                    assert!(!self.scripts.can_resume());
                    // TODO call MapUpdate (multiple times?), see gdialogEnter()
                }

            }
            UiCommandData::Scroll => {
                let (dir, widg) = self.scroll_areas
                    .iter()
                    .find(|&(_, w)| w == &command.source)
                    .unwrap();
                let scrolled = self.world.borrow_mut().scroll(dir, 1) > 0;
                ui.widget_mut::<ScrollArea>(*widg).set_enabled(scrolled);
            }
            _ => {}
        }
    }

    fn update(&mut self, delta: Duration, ui: &mut Ui) {
        self.time.update(delta);

        self.time.set_paused(self.user_paused || self.scripts.can_resume());

        if self.time.is_running() {
            {
                let mut world = self.world.borrow_mut();
                world.update(self.time.time());

                self.seq_events.clear();
                self.sequencer.update(&mut sequence::Update {
                    time: self.time.time(),
                    world: &mut world,
                    out: &mut self.seq_events,
                });
            }

            self.handle_seq_events(ui);

            self.fidget.update(self.time.time(), &mut self.world.borrow_mut(), &mut self.sequencer);
        } else {
            self.sequencer.cleanup(&mut sequence::Cleanup {
                world: &mut self.world.borrow_mut(),
            });
        }
    }
}

pub struct PausableTime {
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

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
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

struct ObjectActionMenu {
    menu: ui::Handle,
    obj: object::Handle,
}
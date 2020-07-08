use bstring::{bstr, BString};
use enum_map::{enum_map, EnumMap};
use if_chain::if_chain;
use log::*;
use measure_time::*;
use sdl2::event::{Event as SdlEvent};
use sdl2::keyboard::Keycode;
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::time::{Instant, Duration};

use crate::asset::{self, *};
use crate::asset::frame::{FrameDb, FrameId};
use crate::asset::map::{ELEVATION_COUNT, MapId, MapReader};
use crate::asset::map::db::MapDb;
use crate::asset::message::{BULLET, Messages};
use crate::asset::proto::*;
use crate::asset::script::db::ScriptDb;
use crate::event::*;
use crate::fs::FileSystem;
use crate::game::dialog::Dialog;
use crate::game::fidget::Fidget;
use crate::game::inventory::Inventory;
use crate::game::object::{self, *};
use crate::game::rpg::Rpg;
use crate::game::sequence::ObjSequencer;
use crate::game::sequence::frame_anim::{AnimDirection, FrameAnim, FrameAnimOptions};
use crate::game::sequence::move_seq::Move;
use crate::game::sequence::stand::Stand;
use crate::game::script::{self, Scripts, ScriptKind};
use crate::game::skilldex::{self, Skilldex};
use crate::game::ui::action_menu::{self, Action};
use crate::game::ui::hud;
use crate::game::ui::scroll_area::ScrollArea;
use crate::game::ui::world::{HexCursorStyle, WorldView};
use crate::game::world::{ScrollDirection, World, WorldRef};
use crate::graphics::{EPoint, Rect};
use crate::graphics::font::Fonts;
use crate::graphics::geometry::hex::{self, Direction};
use crate::sequence::{self, Sequencer};
use crate::sequence::send_event::SendEvent;
use crate::sequence::chain::Chain;
use crate::state::{self, *};
use crate::ui::{self, Ui};
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
    map_db: MapDb,
    world: WorldRef,
    scripts: Scripts,
    obj_sequencer: ObjSequencer,
    fidget: Fidget,
    message_panel: ui::Handle,
    world_view: ui::Handle,
    dialog: Option<Dialog>,
    shift_key_down: bool,
    last_picked_obj: Option<object::Handle>,
    object_action_menu: Option<ObjectActionMenu>,
    user_paused: bool,
    map_id: Option<MapId>,
    in_combat: bool,
    misc_msgs: Rc<Messages>,
    scroll_areas: EnumMap<ScrollDirection, ui::Handle>,
    rpg: Rpg,
    skilldex: Skilldex,
    inventory: Inventory,
    ui_sequencer: Sequencer,
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

        let map_db = MapDb::new(&fs).unwrap();
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
        let obj_sequencer = ObjSequencer::new(now);
        let fidget = Fidget::new(now);

        let world_view_rect = Rect::with_size(0, 0, 640, 379);
        let world_view = {
            let win = ui.new_window(world_view_rect, None);
            ui.new_widget(win, world_view_rect, None, None, WorldView::new(world.clone()))
        };
        let message_panel = hud::create(ui);

        let scroll_areas = Self::create_scroll_areas(Rect::with_size(0, 0, 640, 480), ui);

        let rpg = Rpg::new(&fs, language).unwrap();

        let skilldex = Skilldex::new(&fs, language);

        let inventory = Inventory::new(world.clone(), &fs, language);

        let ui_sequencer = Sequencer::new(now);

        Self {
            time,
            fs,
            frm_db,
            proto_db,
            map_db,
            world,
            scripts,
            obj_sequencer,
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
            misc_msgs,
            scroll_areas,
            rpg,
            skilldex,
            inventory,
            ui_sequencer,
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

    pub fn new_game(&mut self) {
        self.scripts.vars.global_vars =
            asset::read_game_global_vars(&mut self.fs.reader("data/vault13.gam").unwrap()).unwrap().into();

        let dude_fid = FrameId::from_packed(0x100003E).unwrap();
        //    let dude_fid = FrameId::from_packed(0x101600A).unwrap();
        let _ = self.world.borrow_mut().objects_mut().create(
            Some(dude_fid),
            Some(self.proto_db.dude()),
            Some(Default::default()),
            Some(&self.rpg));

        // TODO replace with proper init
        self.world.borrow_mut().objects_mut().dude_mut().sub.as_critter_mut().unwrap()
            .hit_points = 44;
        let d = self.proto_db.dude();
        let mut d = d.borrow_mut();
        d.set_name("Narg".into());
        let c = d.sub.as_critter_mut().unwrap();
        c.base_stats[Stat::Strength] = 9;
        c.base_stats[Stat::Perception] = 6;
        c.base_stats[Stat::Endurance] = 10;
        c.base_stats[Stat::Charisma] = 4;
        c.base_stats[Stat::Intelligence] = 5;
        c.base_stats[Stat::Agility] = 8;
        c.base_stats[Stat::Luck] = 5;
        c.base_stats[Stat::HitPoints] = 44;
        c.base_stats[Stat::ArmorClass] = 8;
        c.base_stats[Stat::ActionPoints] = 9;
        c.base_stats[Stat::MeleeDmg] = 8;
        c.base_stats[Stat::CarryWeight] = 250;
    }

    pub fn switch_map(&mut self, map_name: &str, ui: &mut Ui) {
        debug!("switching map to `{}`", map_name);

        if let Some(map_id) = self.map_id {
            let ctx = &mut script::Context {
                world: &mut self.world.borrow_mut(),
                obj_sequencer: &mut self.obj_sequencer,
                dialog: &mut self.dialog,
                message_panel: self.message_panel,
                ui,
                map_id,
                source_obj: None,
                target_obj: None,
                skill: None,
                rpg: &mut self.rpg,
            };
            self.scripts.execute_map_procs(PredefinedProc::MapExit, ctx);
        }

        let mut dude_obj = {
            let mut world = self.world.borrow_mut();
            let dude_obj = world.objects().dude();
            let dude_obj = world.objects_mut().remove_deep(dude_obj);
            world.clear();
            dude_obj
        };

        self.scripts.reset();
        self.obj_sequencer.clear();

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
                for_each_direction(world.objects().get(obj).fid, |fid| {
                    if let Err(e) = self.frm_db.get(fid) {
                        warn!("error preloading {:?}: {:?}", fid, e);
                    }
                });
            }
        }
        self.frm_db.get(FrameId::EGG).unwrap();

        world.set_sqr_tiles(map.sqr_tiles);

        {
            let mut dude_obj = dude_obj.objects.get_mut(dude_obj.root).unwrap();
            dude_obj.direction = map.entrance_direction;
            dude_obj.set_light_emitter(LightEmitter {
                intensity: 0x10000,
                radius: 4,
            });
            dude_obj.set_pos(Some(map.entrance));
        }
        let dude_obj = world.objects_mut().insert_graph(dude_obj);

        world.objects_mut().make_standing(dude_obj);

        {
            assert!(!map.savegame);
            let path = format!("maps/{}.gam", map_name);
            self.scripts.vars.map_vars = if self.fs.exists(&path) {
                asset::read_map_global_vars(&mut self.fs.reader(&path).unwrap()).unwrap().into()
            } else {
                Vec::new().into()
            };
        }

        // Init scripts.
        {
            let ctx = &mut script::Context {
                world,
                obj_sequencer: &mut self.obj_sequencer,
                dialog: &mut self.dialog,
                message_panel: self.message_panel,
                ui,
                map_id: map.id,
                source_obj: None,
                target_obj: None,
                skill: None,
                rpg: &mut self.rpg,
            };

            // PredefinedProc::Start for map script is never called.
            // MapEnter in map script is called before anything else.
            if let Some(sid) = self.scripts.map_sid() {
                self.scripts.execute_predefined_proc(sid, PredefinedProc::MapEnter, ctx)
                    .map(|r| r.suspend.map(|_| panic!("can't suspend in MapEnter")));
            }

            self.scripts.execute_procs(PredefinedProc::Start, ctx, |sid| sid.kind() != ScriptKind::System);
            self.scripts.execute_map_procs(PredefinedProc::MapEnter, ctx);
            self.scripts.execute_map_procs(PredefinedProc::MapUpdate, ctx);
        }

        world.camera_look_at_dude();
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
                self.obj_sequencer.cancel(obj);
                let world = self.world.borrow_mut();
                let mut obj = world.objects().get_mut(obj);
                obj.direction = obj.direction.rotate_cw();
            }
            Action::Talk => {
                let talker = self.world.borrow().objects().dude();
                self.action_talk(talker, obj, ui);
            }
            Action::UseHand => {
                let user = self.world.borrow().objects().dude();
                self.action_use_obj(user, obj);
            }
            Action::UseSkill => {
                self.show_skilldex(ui, Some(obj));
            }
        }
    }

    fn actions(&self, objh: object::Handle) -> Vec<(Action, Event)> {
        let mut r = Vec::new();
        let world = self.world.borrow();
        let obj = world.objects().get(objh);
        match obj.kind() {
            EntityKind::Critter => {
                if objh == world.objects().dude() {
                    r.push(Action::Rotate);
                } else {
                    if world.objects().get(objh).can_talk_to() {
                        if !self.in_combat {
                            r.push(Action::Talk);
                        }
                    } else if !obj.proto().unwrap()
                        .sub.as_critter().unwrap()
                        .flags.contains(CritterFlag::NoSteal)
                    {
                        r.push(Action::UseHand);
                    }
                    if world.objects().can_push(world.objects().dude(), objh,
                        &self.scripts, self.in_combat)
                    {
                        r.push(Action::Push);
                    }
                }
                r.extend_from_slice(&[Action::Look, Action::Inventory, Action::UseSkill])
            }
            EntityKind::Item => {
                r.extend_from_slice(&[Action::UseHand, Action::Look]);
                if world.objects().get(objh).item_kind() == Some(ItemKind::Container) {
                    r.extend_from_slice(&[Action::UseSkill, Action::Inventory]);
                }
            }
            EntityKind::Scenery => {
                if world.objects().get(objh).can_use() {
                    r.push(Action::UseHand)
                }
                r.extend_from_slice(&[Action::Look, Action::Inventory, Action::UseSkill])
            }
            EntityKind::Wall => {
                r.push(Action::Look);
                if world.objects().get(objh).can_use() {
                    r.push(Action::UseHand)
                }
            }
            _ => {}
        }
        if !r.is_empty() {
            r.push(Action::Cancel)
        }
        r.iter()
            .map(|&action| (action, Event::Action { action }))
            .collect()
    }

    fn look_at_object(&mut self, looker: object::Handle, looked: object::Handle, ui: &mut Ui)
        -> Option<BString>
    {
        let sid = {
            let world = self.world.borrow();
            let lookero = world.objects().get(looker);
            let lookedo = world.objects().get(looked);
            if lookero.sub.as_critter().map(|c| c.is_dead()).unwrap_or(true)
                // TODO This is only useful for mapper?
                || lookedo.kind() == EntityKind::SqrTile
                || lookedo.proto_ref().is_none()
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
                    obj_sequencer: &mut self.obj_sequencer,
                    dialog: &mut self.dialog,
                    ui,
                    message_panel: self.message_panel,
                    map_id: self.map_id.unwrap(),
                    source_obj: Some(looker),
                    target_obj: Some(looked),
                    skill: None,
                    rpg: &mut self.rpg,
                });
            then {
                assert!(r.suspend.is_none(), "can't suspend");
                if r.script_overrides {
                    return None;
                }
            }
        }

        let world = self.world.borrow();
        let lookedo = world.objects().get(looked);
        let msg_id = if lookedo.sub.as_critter().map(|c| c.is_dead()).unwrap_or(false) {
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
        let dude_obj = self.world().borrow().objects().dude();
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
            let examinero = world.objects().get(examiner);
            let examinedo = world.objects().get(examined);
            if examinero.sub.as_critter().map(|c| c.is_dead()).unwrap_or(false)
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
                    obj_sequencer: &mut self.obj_sequencer,
                    dialog: &mut self.dialog,
                    ui,
                    message_panel: self.message_panel,
                    map_id: self.map_id.unwrap(),
                    source_obj: Some(examiner),
                    target_obj: Some(examined),
                    skill: None,
                    rpg: &mut self.rpg,
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
            let examinedo = world.objects().get(examined);
            if !examinedo.sub.as_critter().map(|c| c.is_dead()).unwrap_or(false) {
                let descr = examinedo.proto()
                    .and_then(|p| {
                        p.description()
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
        let dude_obj = self.world().borrow().objects().dude();
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
                let chain = Chain::new();
                chain.control()
                    .cancellable(Move::new(talker, PathTo::Object(talked), CritterAnim::Running))
                    .finalizing(Stand::new(talker))
                    .finalizing(SendEvent::new(Event::Talk { talker, talked }));
                self.obj_sequencer.replace(talker, chain);
                return;
            }
        }

        self.talk(talker, talked, ui);
    }

    // talk_to(), gdialogEnter()
    fn talk(&mut self, talker: object::Handle, talked: object::Handle, ui: &mut Ui) {
        if self.world.borrow().objects().can_talk_now(talker, talked) {
            let world = &mut self.world.borrow_mut();
            self.obj_sequencer.clear();
            self.obj_sequencer.sync(&mut sequence::Sync {
                world,
                ui,
            });
            let script = world.objects().get(talked).script;
            if let Some((sid, _)) = script {
                match self.scripts.execute_predefined_proc(sid, PredefinedProc::Talk,
                    &mut script::Context {
                        world,
                        obj_sequencer: &mut self.obj_sequencer,
                        dialog: &mut self.dialog,
                        ui,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                        source_obj: Some(talker),
                        target_obj: Some(talked),
                        skill: None,
                        rpg: &mut self.rpg,
                    }).and_then(|r| r.suspend)
                    {
                        None | Some(Suspend::GsayEnd) => {}
                    }
            }
        } else {
            assert_eq!(talker, self.world.borrow().objects().dude());
            let msg = &self.misc_msgs.get(2000).unwrap().text;
            self.push_message(&msg, ui);
        }
    }

    //  action_use_an_item_on_object_
    fn action_use_obj(&mut self, user: object::Handle, used: object::Handle) {
        let world = self.world.borrow();
        let objs = world.objects();
        let usero = objs.get(user);
        let usedo = objs.get(used);

        let used_kind = usedo.proto().map(|p| p.kind()).unwrap();
        if used_kind == ExactEntityKind::Scenery(SceneryKind::LadderDown) {
            // TODO action_climb_ladder
            return;
        }

        let seq = Chain::new();

        let move_anim = if usero.distance(&usedo).unwrap() < 5 {
            CritterAnim::Walk
        } else {
            CritterAnim::Running
        };
        seq.control().cancellable(Move::new(user, PathTo::Object(used), move_anim));

        let weapon = usero.fid.critter().unwrap().weapon();
        if weapon != WeaponKind::Unarmed {
            seq.control().cancellable(FrameAnim::new(user,
                FrameAnimOptions { anim: Some(CritterAnim::PutAway), ..Default::default() }));
        }

        if used_kind != ExactEntityKind::Scenery(SceneryKind::Stairs) {
            // FIXME must call check_next_to() before running this animation
            let use_anim = if usedo.is_critter_prone() ||
                usedo.kind() == EntityKind::Scenery &&
                    usedo.proto().unwrap().flags_ext.contains(FlagExt::Prone)
            {
                CritterAnim::MagicHandsGround
            } else {
                CritterAnim::MagicHandsMiddle
            };
            seq.control().cancellable(FrameAnim::new(user,
                FrameAnimOptions { anim: Some(use_anim), ..Default::default() }));
        }

        seq.control().cancellable(SendEvent::new(Event::Use { user, used }));
        if weapon != WeaponKind::Unarmed {
            seq.control().cancellable(FrameAnim::new(user,
                FrameAnimOptions { anim: Some(CritterAnim::TakeOut), ..Default::default() }));
        }
        seq.control().finalizing(Stand::new(user));

        self.obj_sequencer.replace(user, seq);
    }

    // obj_use
    fn use_obj(&mut self, user: object::Handle, used: object::Handle, ui: &mut Ui) {
        if !self.check_next_to(user, used, ui) {
            return;
        }
        // TODO why different results?
        // if ( user == g_obj_dude )
        //   {
        //     if ( used_type != OBJ_TYPE_SCENERY )
        //       return -1;
        //   }
        //   else if ( used_type != OBJ_TYPE_SCENERY )
        //   {
        //     return 0;
        //   }

        let (used_kind, script) = {
            let world = self.world.borrow();
            let usedo = world.objects().get(used);
            let used_kind = unwrap_or_return!(
                usedo.proto().map(|p| p.kind()),
                Some(ExactEntityKind::Scenery(v)) => v);
            (used_kind, usedo.script)
        };

        if used_kind == SceneryKind::Door {
            self.use_door(user, used, ui);
        } else {
            let world = &mut self.world.borrow_mut();

            let script_overrides = if let Some((sid, _)) = script {
                self.scripts.execute_predefined_proc(sid, PredefinedProc::Use,
                    &mut script::Context {
                        world,
                        obj_sequencer: &mut self.obj_sequencer,
                        dialog: &mut self.dialog,
                        ui,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                        source_obj: Some(user),
                        target_obj: Some(used),
                        skill: None,
                        rpg: &mut self.rpg,
                    }).unwrap().assert_no_suspend().script_overrides
            } else {
                false
            };
            let script_overrides = if !script_overrides {
                match used_kind {
                    SceneryKind::Door => unreachable!(),
                    // TODO
                    | SceneryKind::Stairs
                    | SceneryKind::Elevator
                    | SceneryKind::LadderDown
                    | SceneryKind::LadderUp
                    => {
                        warn!("{:?} use is not implemented", used_kind);
                        false
                    }
                    SceneryKind::Misc => false,
                }
            } else {
                false
            };
            if !script_overrides && user == world.objects().dude() {
                if let Some(obj_name) = world.object_name(used) {
                    let msg = &self.proto_db.messages().get(MSG_YOU_SEE_X).unwrap().text;
                    let msg = sprintf(msg, &[&obj_name]);
                    self.push_message(&msg, ui);
                }
            }
        }
    }

    fn use_door(&mut self, user: object::Handle, door: object::Handle, ui: &mut Ui) {
        let world = &mut self.world.borrow_mut();

        let script = {
            let dooro = world.objects().get(door);
            if dooro.is_locked().unwrap() {
                // TODO sfx
            }
            dooro.script
        };

        if let Some((sid, _)) = script {
            let script_overrides = self.scripts.execute_predefined_proc(sid, PredefinedProc::Use,
                &mut script::Context {
                    world,
                    obj_sequencer: &mut self.obj_sequencer,
                    dialog: &mut self.dialog,
                    ui,
                    message_panel: self.message_panel,
                    map_id: self.map_id.unwrap(),
                    source_obj: Some(user),
                    target_obj: Some(door),
                    skill: None,
                    rpg: &mut self.rpg,
                }).unwrap().assert_no_suspend().script_overrides;
            if script_overrides {
                return;
            }
        }

        let dooro = world.objects().get(door);
        let need_open = if dooro.frame_idx > 0 { // Indicates the door is open
            if world.objects().has_blocker_at(dooro.pos(), None) {
                let msg = &self.proto_db.messages().get(MSG_DOORWAY_SEEMS_TO_BE_BLOCKED).unwrap().text;
                self.push_message(&msg, ui);
                return
            }
            false
        } else {
            if dooro.sub.as_scenery().unwrap().as_door().unwrap().flags.contains(DoorFlag::Open) {
                return;
            }
            true
        };

        let seq = Chain::new();
        seq.control()
            .cancellable(FrameAnim::new(door, FrameAnimOptions {
                direction: if need_open { AnimDirection::Forward } else { AnimDirection::Backward },
                skip: 1,
                ..Default::default()
            }))
            .finalizing(SendEvent::new(Event::SetDoorState { door, open: need_open }));

        self.obj_sequencer.replace(door, seq);
    }

    // set_door_open, set_door_closed, check_door_state
    fn set_door_state(&mut self, door: object::Handle, open: bool) {
        let mut world = self.world.borrow_mut();
        {
            {
                let mut dooro = world.objects_mut().get_mut(door);
                {
                    let door = dooro.sub.as_scenery_mut().unwrap().as_door_mut().unwrap();
                    if open {
                        door.flags.insert(DoorFlag::Open);
                    } else {
                        door.flags.remove(DoorFlag::Open);
                    }
                }
                if open {
                    dooro.flags.insert(Flag::ShootThru | Flag::LightThru | Flag::NoBlock);
                } else {
                    dooro.flags.remove(Flag::ShootThru | Flag::LightThru | Flag::NoBlock);
                }
            }

            world.objects_mut().set_frame(door, if open {
                SetFrame::Last
            } else {
                SetFrame::Index(0)
            });
        }
        world.objects_mut().rebuild_light_grid();
    }

    // is_next_to
    fn check_next_to(&mut self, obj1: object::Handle, obj2: object::Handle, ui: &mut Ui) -> bool {
        if self.world.borrow().objects().distance(obj1, obj2).unwrap() > 1 {
            let msg = &self.misc_msgs.get(2000).unwrap().text;
            self.push_message(msg, ui);
            false
        } else {
            true
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

    fn set_dude_pos(&mut self, pos: EPoint, direction: Direction, ui: &mut Ui) {
        let world = &mut self.world.borrow_mut();
        let dude_objh = world.objects().dude();
        let elevation_change = {
            let mut dude_obj = world.objects_mut().get_mut(dude_objh);
            dude_obj.direction = direction;
            dude_obj.pos().elevation != pos.elevation
        };
        world.objects_mut().set_pos(dude_objh, Some(pos));
        if elevation_change {
            let ctx = &mut script::Context {
                ui,
                world,
                obj_sequencer: &mut self.obj_sequencer,
                dialog: &mut self.dialog,
                message_panel: self.message_panel,
                map_id: self.map_id.unwrap(),
                source_obj: None,
                target_obj: None,
                skill: None,
                rpg: &mut self.rpg,
            };
            self.scripts.execute_map_procs(PredefinedProc::MapUpdate, ctx);
        }
        world.camera_look_at_dude();
    }

    fn show_skilldex(&mut self, ui: &mut Ui, target: Option<object::Handle>) {
        let world = self.world.borrow();
        let dude_obj = world.objects().get(world.objects().dude());
        let levels = EnumMap::from(
            |skill: skilldex::Skill| self.rpg.skill(skill.into(), &dude_obj, world.objects()));
        self.skilldex.show(ui, levels, target);
    }

    // action_use_skill_on
    fn action_use_skill_on(&mut self, skill: Skill, target: object::Handle) {
        let world = self.world.borrow();
        let objs = world.objects();
        let user = world.objects().dude();
        let usero = objs.get(user);
        let targeto = objs.get(target);

        match skill {
            Skill::Lockpick => {
                // TODO check combat state
                match targeto.kind() {
                    EntityKind::Item | EntityKind::Scenery => {}
                    _ => {
                        debug!("{:?} can't use Lockpick on non-item/non-scenery {:?}", user, target);
                        return;
                    }
                }
            }
            // TODO
            Skill::Steal => {}
            Skill::Traps => {}
            Skill::FirstAid | Skill::Doctor => {}
            Skill::Science | Skill::Repair => {}
            _ => return,
        }

        // TODO handle party members

        let seq = Chain::new();

        let move_anim = if usero.distance(&targeto).unwrap() < 5 {
            CritterAnim::Walk
        } else {
            CritterAnim::Running
        };
        seq.control().cancellable(Move::new(user, PathTo::Object(target), move_anim));

        let use_anim = if targeto.is_critter_prone() {
            CritterAnim::MagicHandsGround
        } else {
            CritterAnim::MagicHandsMiddle
        };
        // FIXME must call check_next_to() before running this animation
        seq.control()
            .cancellable(FrameAnim::new(user,
                FrameAnimOptions { anim: Some(use_anim), ..Default::default() }))
            .cancellable(SendEvent::new(Event::UseSkill { skill, user, target }))
            .finalizing(Stand::new(user));

        self.obj_sequencer.replace(user, seq);
    }

    // obj_use_skill_on
    fn use_skill_on(&mut self,
        skill: Skill,
        user: object::Handle,
        target: object::Handle,
        ui: &mut Ui,
    ) {
        let script_overrides = {
            let world = &mut self.world.borrow_mut();
            let script = {
                let targeto = world.objects().get(target);
                if user == world.objects().dude() && targeto.is_lock_jammed() == Some(true)
                {
                    let msg = &self.misc_msgs.get(2001).unwrap().text;
                    self.push_message(msg, ui);
                    return;
                }
                targeto.script
            };

            if let Some((sid, _)) = script {
                self.scripts.execute_predefined_proc(sid, PredefinedProc::UseSkillOn,
                    &mut script::Context {
                        world,
                        obj_sequencer: &mut self.obj_sequencer,
                        dialog: &mut self.dialog,
                        ui,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                        source_obj: Some(user),
                        target_obj: Some(target),
                        skill: Some(skill),
                        rpg: &mut self.rpg,
                    }).unwrap().assert_no_suspend().script_overrides
            } else {
                false
            }
        };
        if !script_overrides {
            self.default_use_skill_on(skill, user, target, ui);
        }
    }

    fn default_use_skill_on(&mut self,
        skill: Skill,
        _user: object::Handle,
        target: object::Handle,
        ui: &mut Ui,
    ) {
        let world = &mut self.world.borrow_mut();
        {
            let targeto = world.objects().get(target);
            match skill {
                Skill::FirstAid => {
                    // TODO if !skill_use_slot_available {
                    let msg_id = 590 + random(0, 2);
                    let msg = &self.rpg.skill_msgs().get(msg_id).unwrap().text;
                    self.push_message(msg, ui);
                    return;

                    // TODO call MapUpdate after fade out - fade in
                }
                Skill::Doctor => {
                    // TODO if !skill_use_slot_available {
                    let msg_id = 590 + random(0, 2);
                    let msg = &self.rpg.skill_msgs().get(msg_id).unwrap().text;
                    self.push_message(msg, ui);
                    return;

                    // TODO call MapUpdate after fade out - fade in
                }
                Skill::Sneak | Skill::Lockpick => {}
                Skill::Repair => {
                    if targeto.proto().and_then(|p| p.sub.as_critter().map(|c| c.body_kind)) != Some(BodyKind::Robotic) {
                        self.push_message(&self.rpg.skill_msgs().get(553).unwrap().text, ui);
                        return;
                    }
                    // TODO
                    return;
                }
                Skill::Steal => {
                    // TODO
                }
                Skill::Traps => {
                    self.push_message(&self.rpg.skill_msgs().get(551).unwrap().text, ui);
                    return;
                }
                Skill::Science => {
                    self.push_message(&self.rpg.skill_msgs().get(552).unwrap().text, ui);
                    return;
                }
                _ => {
                    error!("[default_use_skill_on] invalid skill used: {:?}", skill);
                    return;
                }
            }
            // TODO show_skill_use_messages
            debug!("TODO");
        }
    }
}

impl AppState for GameState {
    fn handle_event(&mut self, ctx: HandleEvent) {
        self.inventory.handle(ctx.event, ctx.sink, &self.rpg, ctx.ui, &mut self.ui_sequencer);
        match ctx.event {
            // map_check_state
            // TODO handle special map ids: 19, 37
            Event::MapExit { map, pos, direction } => {
                match map {
                    TargetMap::CurrentMap => {
                        self.set_dude_pos(pos, direction, ctx.ui);
                    }
                    TargetMap::Map { map_id } => {
                        if self.map_id.unwrap() != map_id {
                            let map_def = self.map_db.get(map_id).unwrap();
                            let name = map_def.name.clone();
                            self.switch_map(&name, ctx.ui);
                        }
                        self.set_dude_pos(pos, direction, ctx.ui);
                    }
                    TargetMap::WorldMap(k) => {
                        warn!("map exit to {:?} is not implemented", k);
                    }
                }
            }
            Event::ObjectPick { kind, obj: objh } => {
                let actions = self.actions(objh);
                let default_action = actions.first().map(|&(a, _)| a);
                match kind {
                    ObjectPickKind::Hover => {
                        // TODO highlight item on Action::UseHand: gmouse_bk_process()

                        ctx.ui.widget_mut::<WorldView>(self.world_view).default_action_icon = if self.object_action_menu.is_none() {
                            default_action
                        }  else {
                            None
                        };

                        if self.last_picked_obj != Some(objh) {
                            self.last_picked_obj = Some(objh);
                            self.dude_look_at_object(objh, ctx.ui);
                        }
                    }
                    ObjectPickKind::ActionMenu => {
                        ctx.ui.widget_mut::<WorldView>(self.world_view).default_action_icon = None;

                        let world_view_win = ctx.ui.window_of(self.world_view).unwrap();
                        self.object_action_menu = Some(ObjectActionMenu {
                            menu: action_menu::show(actions, world_view_win, ctx.ui),
                            obj: objh,
                        });

                        self.time.set_paused(true);
                    }
                    ObjectPickKind::DefaultAction => if let Some(a) = default_action {
                        ctx.ui.widget_mut::<WorldView>(self.world_view).default_action_icon = if self.object_action_menu.is_none() {
                            default_action
                        }  else {
                            None
                        };
                        self.handle_action(ctx.ui, objh, a);
                    },
                    ObjectPickKind::Skill(skill) => {
                        self.action_use_skill_on(skill, objh);
                    }
                }
            }
            Event::HexPick { action, pos } => {
                if action {
                    let dude_objh = self.world.borrow().objects().dude();

                    let seq = Chain::new();

                    let anim = if self.shift_key_down {
                        CritterAnim::Walk
                    } else {
                        CritterAnim::Running
                    };
                    seq.control()
                        .cancellable(Move::new(dude_objh, PathTo::Point {
                            point: pos.point,
                            neighbor_if_blocked: true,
                        }, anim))
                        .finalizing(Stand::new(dude_objh));
                    self.obj_sequencer.replace(dude_objh, seq);
                } else {
                    let mut wv = ctx.ui.widget_mut::<WorldView>(self.world_view);
                    let dude_obj = self.world.borrow().objects().dude();
                    wv.hex_cursor_style = if self.world.borrow()
                        .objects().path(dude_obj, PathTo::Point {
                            point: pos.point,
                            neighbor_if_blocked: false,
                        }, false).is_some()
                    {
                        HexCursorStyle::Normal
                    } else {
                        HexCursorStyle::Blocked
                    };
                }
            }
            Event::Action { action } => {
                let object_action = self.object_action_menu.take().unwrap();
                self.handle_action(ctx.ui, object_action.obj, action);
                action_menu::hide(object_action.menu, ctx.ui);
                self.time.set_paused(false);
            }
            Event::Pick { id } => {
                let (sid, proc_id) = {
                    let dialog = self.dialog.as_mut().unwrap();
                    let proc_id = dialog.option(id).proc_id;
                    dialog.clear_options(ctx.ui);

                    (dialog.sid(), proc_id)
                };
                let finished = if let Some(proc_id) = proc_id {
                    let world = &mut self.world.borrow_mut();
                    let source_obj = Some(world.objects().dude());
                    let target_obj = Some(self.dialog.as_ref().unwrap().obj);
                    self.scripts.execute_proc(sid, proc_id,
                        &mut script::Context {
                            ui: ctx.ui,
                            world,
                            obj_sequencer: &mut self.obj_sequencer,
                            dialog: &mut self.dialog,
                            message_panel: self.message_panel,
                            map_id: self.map_id.unwrap(),
                            source_obj,
                            target_obj,
                            skill: None,
                            rpg: &mut self.rpg,
                        }).assert_no_suspend();
                    // No dialog options means the dialog is finished.
                    self.dialog.as_ref().unwrap().is_empty()
                } else {
                    true
                };
                if finished {
                    let ctx = &mut script::Context {
                        ui: ctx.ui,
                        world: &mut self.world.borrow_mut(),
                        obj_sequencer: &mut self.obj_sequencer,
                        dialog: &mut self.dialog,
                        message_panel: self.message_panel,
                        map_id: self.map_id.unwrap(),
                        source_obj: None,
                        target_obj: None,
                        skill: None,
                        rpg: &mut self.rpg,
                    };
                    self.scripts.resume(ctx).assert_no_suspend();
                    assert!(!self.scripts.can_resume());

                    // In original MapUpdate is not always called (see gdialogEnter),
                    // but for now this difference doesn't seem to matter
                    self.scripts.execute_map_procs(PredefinedProc::MapUpdate, ctx);
                }
            }
            Event::Scroll { source } => {
                let (dir, widg) = self.scroll_areas
                    .iter()
                    .find(|&(_, &w)| w == source)
                    .unwrap();
                let scrolled = self.world.borrow_mut().scroll(dir, 1) > 0;
                ctx.ui.widget_mut::<ScrollArea>(*widg).set_enabled(scrolled);
            }
            Event::Skilldex(e) => match e {
                SkilldexEvent::Cancel => self.skilldex.hide(ctx.ui),
                SkilldexEvent::Show => {
                    self.show_skilldex(ctx.ui, None);
                },
                SkilldexEvent::Skill { skill, target } => {
                    self.skilldex.hide(ctx.ui);
                    if let Some(target) = target {
                        self.action_use_skill_on(skill, target);
                    } else {
                        ctx.ui.widget_mut::<WorldView>(self.world_view).enter_skill_target_pick_mode(skill);
                    }
                }
            }
            Event::Inventory(event) => match event {
                InventoryEvent::Hover { object } => {
                    self.dude_look_at_object(object, ctx.ui);
                }
                InventoryEvent::Action { object, action: Action::Look } => {
                    let dude_obj = self.world.borrow().objects().dude();
                    let descr = self.examine_object(dude_obj, object, ctx.ui);
                    let descr = BString::join(b'\n', &descr);
                    self.inventory.examine(object, &descr, ctx.ui);
                }
                InventoryEvent::Show => {
                    self.obj_sequencer.cancel(self.world.borrow().objects().dude());
                }
                _ => {}
            }
            Event::MoveWindow(_) => {}
            Event::ObjectMoved { obj, new_pos, .. } => {
                let world = self.world.borrow();
                if obj == world.objects().dude() {
                    for &h in world.objects().at(new_pos) {
                        let obj = world.objects().get(h);
                        if let Some(map_exit) = obj.sub.as_map_exit() {
                            debug!("dude on map exit object at {:?}: {:?}", new_pos, map_exit);
                            ctx.sink.defer(Event::MapExit {
                                map: map_exit.map,
                                pos: map_exit.pos,
                                direction: map_exit.direction,
                            });
                        }
                    }
                }
            }
            Event::Talk { talker, talked } => {
                self.talk(talker, talked, ctx.ui);
            }
            Event::SetDoorState { door, open } => {
                self.set_door_state(door, open);
            }
            Event::Use { user, used } => {
                self.use_obj(user, used, ctx.ui);
            }
            Event::UseSkill { skill, user, target } => {
                self.use_skill_on(skill, user, target, ctx.ui);
            }
            _ => {}
        }
    }

    fn handle_input(&mut self, event: &SdlEvent, ui: &mut Ui) -> bool {
        let mut world = self.world.borrow_mut();
        match event {
            SdlEvent::KeyDown { keycode: Some(Keycode::Right), .. } => {
                world.scroll(ScrollDirection::E, 1);
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::Left), .. } => {
                world.scroll(ScrollDirection::W, 1);
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::Up), .. } => {
                world.scroll(ScrollDirection::N, 1);
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::Down), .. } => {
                world.scroll(ScrollDirection::S, 1);
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::A), .. } => {
                let dude_obj = world.objects().dude();
                let new_pos = {
                    let obj = world.objects().get_mut(dude_obj);
                    let mut new_pos = obj.pos();
                    new_pos.elevation += 1;
                    while new_pos.elevation < ELEVATION_COUNT && !world.has_elevation(new_pos.elevation) {
                        new_pos.elevation += 1;
                    }
                    new_pos
                };
                if new_pos.elevation < ELEVATION_COUNT && world.has_elevation(new_pos.elevation) {
                    world.objects_mut().set_pos(dude_obj, Some(new_pos));
                }
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::Z), .. } => {
                let dude_obj = world.objects().dude();
                let new_pos = {
                    let obj = world.objects().get_mut(dude_obj);
                    let mut new_pos = obj.pos();
                    if new_pos.elevation > 0 {
                        new_pos.elevation -= 1;
                        while new_pos.elevation > 0 && !world.has_elevation(new_pos.elevation) {
                            new_pos.elevation -= 1;
                        }
                    }
                    new_pos
                };
                if world.has_elevation(new_pos.elevation) {
                    world.objects_mut().set_pos(dude_obj, Some(new_pos));
                }
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::LeftBracket), .. } => {
                world.ambient_light = cmp::max(world.ambient_light as i32 - 1000, 0) as u32;
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::RightBracket), .. } => {
                world.ambient_light = cmp::min(world.ambient_light + 1000, 0x10000);
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::R), .. } => {
                let mut wv = ui.widget_mut::<WorldView>(self.world_view);
                wv.roof_visible = wv.roof_visible;
            }
            SdlEvent::KeyDown { keycode: Some(Keycode::P), .. } => {
                self.user_paused = !self.user_paused;
            }

            SdlEvent::KeyDown { keycode: Some(Keycode::LShift), .. } |
            SdlEvent::KeyDown { keycode: Some(Keycode::RShift), .. } => self.shift_key_down = true,
            SdlEvent::KeyUp { keycode: Some(Keycode::LShift), .. } |
            SdlEvent::KeyUp { keycode: Some(Keycode::RShift), .. } => self.shift_key_down = false,
            _ => return false,
        }
        true
    }

    fn update(&mut self, mut ctx: state::Update) {
        self.time.set_paused(
            self.user_paused ||
            self.scripts.can_resume() ||
            self.skilldex.is_visible() ||
            self.inventory.is_visible());

        self.time.update(ctx.delta);

        let world = &mut self.world.borrow_mut();

        if self.time.is_running() {
            world.update(self.time.time());

            self.obj_sequencer.update(&mut sequence::Update {
                time: self.time.time(),
                world,
                ui: ctx.ui,
                sink: ctx.sink,
            });

            self.fidget.update(
                self.time.time(),
                world,
                &mut self.obj_sequencer);
        } else {
            self.obj_sequencer.sync(&mut sequence::Sync {
                world,
                ui: ctx.ui,
            });
        }

        self.ui_sequencer.update(&mut sequence::Update {
            time: ctx.time,
            world,
            ui: ctx.ui,
            sink: &mut ctx.sink,
        });
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
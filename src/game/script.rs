use bstring::BString;
use byteorder::{BigEndian, ReadBytesExt};
use enum_map::EnumMap;
use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;
use log::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::io::{self, prelude::*};
use std::rc::Rc;

use crate::asset::map::MapId;
use crate::asset::proto::ProtoDb;
use crate::asset::script::ProgramId;
use crate::asset::script::db::ScriptDb;
use crate::game::object;
use crate::util::EnumExt;
use crate::vm::{self, *};
use crate::vm::value::Value;

pub const GVAR_PLAYER_REPUTATION: i32 = 0;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum ScriptKind {
    System = 0x0,
    Spatial = 0x1,
    Time = 0x2,
    Item = 0x3,
    Critter = 0x4,
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
struct SidInternal(u32);

impl SidInternal {
    pub fn new(kind: ScriptKind, id: u32) -> Self {
        assert!(id <= 0xffffff);
        Self((kind as u32) << 24 | id)
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        ScriptKind::from_u32(v >> 24)?;
        Some(Self(v))
    }

    pub fn pack(self) -> u32 {
        self.0
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let v = rd.read_u32::<BigEndian>()?;
        Self::from_packed(v)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData,
                format!("malformed SID: {:x}", v)))
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        let v = rd.read_i32::<BigEndian>()?;
        Ok(if v >= 0 {
            Some(Self::from_packed(v as u32)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData,
                    format!("malformed SID: {:x}", v)))?)
        } else {
            None
        })
    }

    pub fn kind(self) -> ScriptKind {
        ScriptKind::from_u32(self.0 >> 24).unwrap()
    }

    pub fn id(self) -> u32 {
        self.0 & 0xffffff
    }
}

/// Script instance ID is unique identifier of a program instance within a single map and can be
/// created dynamically at runtime. Multiple different script instance IDs can refer to the same
/// program.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub struct ScriptIId(SidInternal);

impl ScriptIId {
    pub fn new(kind: ScriptKind, id: u32) -> Self {
        Self(SidInternal::new(kind, id))
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        SidInternal::from_packed(v).map(Self)
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        SidInternal::read(rd).map(Self)
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        Ok(SidInternal::read_opt(rd)?.map(Self))
    }

    pub fn kind(self) -> ScriptKind {
        self.0.kind()
    }

    pub fn id(self) -> u32 {
        self.0.id()
    }
}

impl fmt::Debug for ScriptIId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScriptIId({:?}, {})", self.kind(), self.id())
    }
}

/// Script program ID is a unique identifier of a program source file, bundled with the script kind.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub struct ScriptPId(SidInternal);

impl ScriptPId {
    pub fn new(kind: ScriptKind, program_id: ProgramId) -> Self {
        Self(SidInternal::new(kind, program_id.val()))
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        SidInternal::from_packed(v).map(Self)
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        SidInternal::read(rd).map(Self)
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        Ok(SidInternal::read_opt(rd)?.map(Self))
    }

    pub fn kind(self) -> ScriptKind {
        self.0.kind()
    }

    pub fn program_id(self) -> ProgramId {
        ProgramId::new(self.0.id()).unwrap()
    }
}

impl fmt::Debug for ScriptPId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScriptPId({:?}, {})", self.kind(), self.program_id().val())
    }
}

pub struct Context<'a> {
    pub ui: &'a mut crate::ui::Ui,
    pub world: &'a mut crate::game::world::World,
    pub sequencer: &'a mut crate::sequence::Sequencer,
    pub dialog: &'a mut Option<crate::game::dialog::Dialog>,
    pub message_panel: crate::ui::Handle,
    pub map_id: MapId,
    pub source_obj: Option<object::Handle>,
    pub target_obj: Option<object::Handle>,
    pub rpg: &'a mut crate::game::rpg::Rpg,
}

pub struct Vars {
    pub map_vars: Box<[i32]>,
    pub global_vars: Box<[i32]>,
    pub external_vars: HashMap<Rc<BString>, Option<Value>>,
}

impl Vars {
    pub fn new() -> Self {
        Self {
            map_vars: Vec::new().into(),
            global_vars: Vec::new().into(),
            external_vars: HashMap::new(),
        }
    }
}

pub struct Script {
    /// Whether the program's initialization code has been run.
    pub inited: bool,
    pub program_id: ProgramId,
    pub program: vm::Handle,
    pub local_vars: Box<[i32]>,
    pub object: Option<object::Handle>,
}

/// Interface for instantiating new scripts from within a script context.
/// The instantiation itself is deferred until the script procedure returns.
pub struct NewScripts {
    unused_sids: EnumMap<ScriptKind, ScriptIId>,
    new_scripts: Vec<(ScriptIId, ProgramId)>,
}

impl NewScripts {
    fn new(scripts: &Scripts) -> Self {
        let mut unused_sids = EnumMap::from(|k| ScriptIId::new(k, 0));
        for &sid in scripts.scripts.keys() {
            if sid.id() > unused_sids[sid.kind()].id() {
                unused_sids[sid.kind()] = sid;
            }
        }
        let mut r = Self { unused_sids, new_scripts: Vec::new() };
        for k in ScriptKind::iter() {
            r.bump(k);
        }
        r
    }

    #[must_use]
    pub fn new_script(&mut self, kind: ScriptKind, prg_id: ProgramId) -> ScriptIId {
        let sid = self.unused_sid(kind);
        self.bump(kind);
        self.new_scripts.push((sid, prg_id));
        sid
    }

    fn unused_sid(&self, kind: ScriptKind) -> ScriptIId {
        self.unused_sids[kind]
    }

    fn bump(&mut self, kind: ScriptKind) {
        let cur = self.unused_sids[kind].id();
        self.unused_sids[kind] = ScriptIId::new(kind, cur.checked_add(1).unwrap());
    }

    fn instantiate(self, scripts: &mut Scripts) {
        for (sid, prg_id) in self.new_scripts {
            scripts.instantiate(sid, prg_id, None).unwrap();
        }
    }
}

pub struct Scripts {
    proto_db: Rc<ProtoDb>,
    db: ScriptDb,
    vm: Vm,
    programs: HashMap<ProgramId, Rc<vm::Program>>,
    scripts: HashMap<ScriptIId, Script>,
    map_sid: Option<ScriptIId>,
    pub vars: Vars,
    suspend_stack: Vec<ScriptIId>,
}

impl Scripts {
    pub fn new(proto_db: Rc<ProtoDb>, db: ScriptDb, vm: Vm) -> Self {
        Self {
            proto_db,
            db,
            vm,
            programs: HashMap::new(),
            scripts: HashMap::new(),
            map_sid: None,
            vars: Vars::new(),
            suspend_stack: Vec::new(),
        }
    }

    pub fn map_sid(&self) -> Option<ScriptIId> {
        self.map_sid
    }

    pub fn reset(&mut self) {
        self.scripts.clear();
        self.map_sid = None;
        self.vars.map_vars = vec![].into();
        self.vars.external_vars.clear();
        self.suspend_stack.clear();
    }

    pub fn instantiate(&mut self,
        sid: ScriptIId,
        program_id: ProgramId,
        local_vars: Option<Box<[i32]>>,
    ) -> io::Result<()> {
        let program = match self.programs.entry(program_id) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                let db = &self.db;
                let (code, info) = db.load(program_id)?;
                let program = Rc::new(self.vm.load(info.name.clone(), code)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                        format!("error loading program {} ({}): {:?}",
                        info.name, program_id.val(), e)))?);
                e.insert(program.clone());
                debug!("loaded `{}` #{} local_var_count={} as {:?}",
                    info.name, program_id.val(), info.local_var_count, sid);
                program
            },
        };

        let local_var_count = self.db.info(program_id).unwrap().local_var_count;
        let local_vars = if let Some(local_vars) = local_vars {
            assert_eq!(local_vars.len(), local_var_count);
            local_vars
        } else {
            vec![0; local_var_count].into()
        };

        let program = self.vm.insert(program);
        let existing = self.scripts.insert(sid, Script {
            inited: false,
            program_id,
            program,
            local_vars,
            object: None,
        });
        if let Some(existing) = existing {
            panic!("{:?} program #{} duplicates existing program #{}",
                sid, program_id.val(), existing.program_id.val());
        }
        Ok(())
    }

    pub fn instantiate_map_script(&mut self, program_id: ProgramId) -> io::Result<ScriptIId> {
        assert!(self.map_sid.is_none());
        let sid = NewScripts::new(self).unused_sid(ScriptKind::System);
        self.instantiate(sid, program_id, None)?;
        self.map_sid = Some(sid);
        Ok(sid)
    }

    pub fn get(&self, sid: ScriptIId) -> Option<&Script> {
        self.scripts.get(&sid)
    }

    pub fn attach_to_object(&mut self, sid: ScriptIId, obj: object::Handle) {
        self.scripts.get_mut(&sid).unwrap().object = Some(obj);
    }

    pub fn execute_proc(&mut self, sid: ScriptIId, proc_id: ProcedureId,
        ctx: &mut Context) -> InvocationResult
    {
        let (r, new_scripts) = {
            let new_scripts = NewScripts::new(self);
            let script = self.scripts.get_mut(&sid).unwrap();
            let mut vm_ctx = Self::make_vm_ctx(
                &mut script.local_vars,
                &mut self.vars,
                &mut self.db,
                new_scripts,
                &self.proto_db,
                script.object,
                ctx);
            if !script.inited {
                debug!("[{:?}#{}:{}] running program initialization code",
                    sid,
                    script.program_id.val(),
                    self.vm.program_state(script.program).program().name());
                self.vm.run(script.program, &mut vm_ctx).unwrap()
                    .assert_no_suspend();
                script.inited = true;
            }
            let prg = self.vm.program_state_mut(script.program);
            debug!("[{:?}#{}:{}] executing proc {:?} ({:?})",
                sid,
                script.program_id.val(),
                prg.program().name(),
                proc_id,
                prg.program().proc(proc_id).map(|p| p.name()));
            let r = prg.execute_proc(proc_id, &mut vm_ctx).unwrap();
            if r.suspend.is_some() {
                self.suspend_stack.push(sid);
            }
            (r, vm_ctx.new_scripts)
        };
        new_scripts.instantiate(self);
        r
    }

    #[must_use]
    pub fn execute_proc_name(&mut self, sid: ScriptIId, proc: &Rc<BString>,
        ctx: &mut Context)-> Option<InvocationResult>
    {
        let script = self.scripts.get_mut(&sid).unwrap();
        let proc_id = self.vm.program_state(script.program)
            .program()
            .proc_id(proc)?;
        Some(self.execute_proc(sid, proc_id, ctx))
    }

    #[must_use]
    pub fn has_predefined_proc(&self, sid: ScriptIId, proc: PredefinedProc) -> bool {
        let script = self.scripts.get(&sid).unwrap();
        self.vm.program_state(script.program).program()
            .predefined_proc_id(proc)
            .is_some()
    }

    pub fn execute_predefined_proc(&mut self, sid: ScriptIId, proc: PredefinedProc,
        ctx: &mut Context) -> Option<InvocationResult>
    {
        let script = self.scripts.get_mut(&sid).unwrap();
        let proc_id = self.vm.program_state(script.program)
            .program()
            .predefined_proc_id(proc)?;
        Some(self.execute_proc(sid, proc_id, ctx))
    }

    pub fn execute_procs(&mut self, proc: PredefinedProc, ctx: &mut Context,
        filter: impl Fn(ScriptIId) -> bool)
    {
        // TODO avoid allocation
        let sids: Vec<_> = self.scripts.keys().cloned().collect();
        for sid in sids {
            if filter(sid) {
                if let Some(r) = self.execute_predefined_proc(sid, proc, ctx) {
                    assert!(r.suspend.is_none(), "can't suspend in {:?}", proc);
                }
            }
        }
    }

    pub fn execute_map_procs(&mut self, proc: PredefinedProc, ctx: &mut Context) {
        assert!(proc == PredefinedProc::MapEnter
            || proc == PredefinedProc::MapExit
            || proc == PredefinedProc::MapUpdate);

        // Execute map script first.
        // MapEnter is ignored since it's executed separately immediately after map loaded.
        if proc != PredefinedProc::MapEnter {
            if let Some(sid) = self.map_sid {
                self.execute_predefined_proc(sid, proc, ctx)
                    .map(|r| r.suspend.map(|_| panic!("can't suspend in {:?}", proc)));
            }
        }

        // Execute other non-map scripts.
        let map_sid = self.map_sid;
        self.execute_procs(proc, ctx, |sid| Some(sid) != map_sid);
    }

    pub fn can_resume(&self) -> bool {
        !self.suspend_stack.is_empty()
    }

    pub fn resume(&mut self, ctx: &mut Context) -> InvocationResult {
        let (r, new_scripts) = {
            let sid = self.suspend_stack.pop().unwrap();
            let new_scripts = NewScripts::new(self);
            let script = self.scripts.get_mut(&sid).unwrap();
            let mut vm_ctx = Self::make_vm_ctx(
                &mut script.local_vars,
                &mut self.vars,
                &mut self.db,
                new_scripts,
                &self.proto_db,
                script.object,
                ctx);
            let r = self.vm.program_state_mut(script.program).resume(&mut vm_ctx).unwrap();
            (r, vm_ctx.new_scripts)
        };
        new_scripts.instantiate(self);
        r
    }

    #[inline]
    fn make_vm_ctx<'a>(
        local_vars: &'a mut [i32],
        vars: &'a mut Vars,
        script_db: &'a mut ScriptDb,
        new_scripts: NewScripts,
        proto_db: &'a ProtoDb,
        self_obj: Option<object::Handle>,
        ctx: &'a mut Context,
    ) -> vm::Context<'a> {
        vm::Context {
            local_vars,
            map_vars: &mut vars.map_vars,
            global_vars: &mut vars.global_vars,
            external_vars: &mut vars.external_vars,

            self_obj,
            source_obj: ctx.source_obj,
            target_obj: ctx.target_obj,
            ui: ctx.ui,
            world: ctx.world,
            sequencer: ctx.sequencer,
            dialog: ctx.dialog,
            message_panel: ctx.message_panel,
            script_db,
            new_scripts,
            proto_db,
            map_id: ctx.map_id,
            rpg: ctx.rpg,
        }
    }
}
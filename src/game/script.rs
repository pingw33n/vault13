use bstring::BString;
use byteorder::{BigEndian, ReadBytesExt};
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
use crate::vm::{self, *};
use crate::vm::value::Value;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum ScriptKind {
    System = 0x0,
    Spatial = 0x1,
    Time = 0x2,
    Item = 0x3,
    Critter = 0x4,
}

/// Script ID carries different semantics than other identifiers (`FrameId`, `ProtoId`). It is a unique
/// identifier of a program instance within a single map, while the aforementioned identifiers
/// refer to static assets. For the reference to the script bytecode file there's another
/// identifier - program ID that maps to file name in `scripts.lst`.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Sid(u32);

impl Sid {
    pub fn new(kind: ScriptKind, id: u32) -> Self {
        assert!(id <= 0xffffff);
        Sid((kind as u32) << 24 | id)
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        ScriptKind::from_u32(v >> 24)?;
        Some(Sid(v))
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

impl fmt::Debug for Sid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sid(0x{:08x})", self.0)
    }
}

pub struct Context<'a> {
    pub ui: &'a mut crate::ui::Ui,
    pub world: &'a mut crate::game::world::World,
    pub sequencer: &'a mut crate::sequence::Sequencer,
    pub dialog: &'a mut Option<crate::game::dialog::Dialog>,
    pub message_panel: crate::ui::Handle,
    pub map_id: MapId,
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

pub struct Scripts {
    proto_db: Rc<ProtoDb>,
    db: ScriptDb,
    vm: Vm,
    programs: HashMap<ProgramId, Rc<vm::Program>>,
    scripts: HashMap<Sid, Script>,
    map_sid: Option<Sid>,
    pub vars: Vars,
    suspend_stack: Vec<Sid>,
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

    pub fn map_sid(&self) -> Option<Sid> {
        self.map_sid
    }

    pub fn reset(&mut self) {
        self.scripts.clear();
        self.map_sid = None;
        self.vars.map_vars = vec![].into();
        self.vars.external_vars.clear();
        self.suspend_stack.clear();
    }

    pub fn instantiate(&mut self, sid: Sid, program_id: ProgramId, local_vars: Option<Box<[i32]>>)
        -> io::Result<()>
    {
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
            panic!("{:?} #{} duplicates existing #{}",
                sid, program_id.val(), existing.program_id.val());
        }
        Ok(())
    }

    pub fn instantiate_map_script(&mut self, program_id: ProgramId) -> io::Result<Sid> {
        assert!(self.map_sid.is_none());
        let sid = self.next_sid(ScriptKind::System);
        self.instantiate(sid, program_id, None)?;
        self.map_sid = Some(sid);
        Ok(sid)
    }

    pub fn get(&self, sid: Sid) -> Option<&Script> {
        self.scripts.get(&sid)
    }

    pub fn attach_to_object(&mut self, sid: Sid, obj: object::Handle) {
        self.scripts.get_mut(&sid).unwrap().object = Some(obj);
    }

    pub fn execute_proc(&mut self, sid: Sid, proc_id: ProcedureId,
        ctx: &mut Context) -> InvocationResult
    {
        let script = self.scripts.get_mut(&sid).unwrap();
        let vm_ctx = &mut Self::make_vm_ctx(
            &mut script.local_vars,
            &mut self.vars,
            &mut self.db,
            &self.proto_db,
            ctx);
        vm_ctx.self_obj = script.object;
        if !script.inited {
            debug!("[{:?}#{}] running program initialization code", sid, script.program_id.val());
            self.vm.run(script.program, vm_ctx).unwrap()
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
        let r = prg.execute_proc(proc_id, vm_ctx).unwrap();
        if r.suspend.is_some() {
            self.suspend_stack.push(sid);
        }
        r
    }

    #[must_use]
    pub fn execute_proc_name(&mut self, sid: Sid, proc: &Rc<BString>,
        ctx: &mut Context)-> Option<InvocationResult>
    {
        let script = self.scripts.get_mut(&sid).unwrap();
        let proc_id = self.vm.program_state(script.program)
            .program()
            .proc_id(proc)?;
        Some(self.execute_proc(sid, proc_id, ctx))
    }

    #[must_use]
    pub fn has_predefined_proc(&self, sid: Sid, proc: PredefinedProc) -> bool {
        let script = self.scripts.get(&sid).unwrap();
        self.vm.program_state(script.program).program()
            .predefined_proc_id(proc)
            .is_some()
    }

    pub fn execute_predefined_proc(&mut self, sid: Sid, proc: PredefinedProc,
        ctx: &mut Context) -> Option<InvocationResult>
    {
        let script = self.scripts.get_mut(&sid).unwrap();
        let proc_id = self.vm.program_state(script.program)
            .program()
            .predefined_proc_id(proc)?;
        Some(self.execute_proc(sid, proc_id, ctx))
    }

    pub fn execute_procs(&mut self, proc: PredefinedProc, ctx: &mut Context,
        filter: impl Fn(Sid) -> bool)
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
        let sid = self.suspend_stack.pop().unwrap();
        let script = self.scripts.get_mut(&sid).unwrap();
        let vm_ctx = &mut Self::make_vm_ctx(
            &mut script.local_vars,
            &mut self.vars,
            &mut self.db,
            &self.proto_db,
            ctx);
        self.vm.program_state_mut(script.program).resume(vm_ctx).unwrap()
    }

    fn next_sid(&self, kind: ScriptKind) -> Sid {
        let id = self.scripts.keys()
            .cloned()
            .filter(|sid| sid.kind() == kind)
            .map(|sid| sid.id())
            .max()
            .map(|v| v + 1)
            .unwrap_or(0);
        Sid::new(kind, id)
    }

    #[inline]
    fn make_vm_ctx<'a>(
        local_vars: &'a mut [i32],
        vars: &'a mut Vars,
        script_db: &'a mut ScriptDb,
        proto_db: &'a ProtoDb,
        ctx: &'a mut Context,
    ) -> vm::Context<'a> {
        vm::Context {
            local_vars,
            map_vars: &mut vars.map_vars,
            global_vars: &mut vars.global_vars,
            external_vars: &mut vars.external_vars,

            self_obj: None,
            ui: ctx.ui,
            world: ctx.world,
            sequencer: ctx.sequencer,
            dialog: ctx.dialog,
            message_panel: ctx.message_panel,
            script_db,
            proto_db,
            map_id: ctx.map_id,
        }
    }
}
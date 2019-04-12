use log::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io;
use std::rc::Rc;

use crate::asset::script::db::ScriptDb;
use crate::asset::script::{ScriptKind, Sid};
use crate::game::object;
use crate::vm::{self, PredefinedProc, Vm};
use crate::vm::value::Value;

pub struct Context<'a> {
    pub world: &'a mut crate::game::world::World,
    pub sequencer: &'a mut crate::sequence::Sequencer,
}

pub struct Vars {
    pub map_vars: Box<[i32]>,
    pub global_vars: Box<[i32]>,
    pub external_vars: HashMap<Rc<String>, Option<Value>>,
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
    pub program_id: u32,
    pub program: vm::Handle,
    pub local_vars: Box<[i32]>,
    pub object: Option<object::Handle>,
}

pub struct Scripts {
    db: ScriptDb,
    vm: Vm,
    programs: HashMap<u32, Rc<vm::Program>>,
    scripts: HashMap<Sid, Script>,
    map_sid: Option<Sid>,
    pub vars: Vars,
}

impl Scripts {
    pub fn new(db: ScriptDb, vm: Vm) -> Self {
        Self {
            db,
            vm,
            programs: HashMap::new(),
            scripts: HashMap::new(),
            map_sid: None,
            vars: Vars::new(),
        }
    }

    pub fn map_sid(&self) -> Option<Sid> {
        self.map_sid
    }

    pub fn instantiate(&mut self, sid: Sid, program_id: u32, local_vars: Option<Box<[i32]>>)
        -> io::Result<()>
    {
        let program = match self.programs.entry(program_id) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                let db = &self.db;
                let (code, info) = db.load(program_id)?;
                let program = Rc::new(self.vm.load(code)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                        format!("error loading program {} ({}): {:?}",
                        info.name, program_id, e)))?);
                e.insert(program.clone());
                debug!("loaded `{}` #{} local_var_count={} as {:?}",
                    info.name, program_id, info.local_var_count, sid);
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
            panic!("{:?} #{} duplicates existing #{}", sid, program_id, existing.program_id);
        }
        Ok(())
    }

    pub fn instantiate_map_script(&mut self, program_id: u32) -> io::Result<Sid> {
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

    pub fn execute_predefined_proc(&mut self, sid: Sid, proc: PredefinedProc,
        ctx: &mut Context)
    {
        Self::execute_predefined_proc0(
            self.scripts.get_mut(&sid).unwrap(),
            &mut self.vm,
            sid,
            proc,
            &mut self.vars,
            ctx)
    }

    pub fn execute_procs(&mut self, proc: PredefinedProc, ctx: &mut Context,
        filter: impl Fn(Sid) -> bool)
    {
        for (&sid, script) in self.scripts.iter_mut() {
            if filter(sid) {
                Self::execute_predefined_proc0(
                    script,
                    &mut self.vm,
                    sid,
                    proc,
                    &mut self.vars,
                    ctx);
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
                self.execute_predefined_proc(sid, proc, ctx);
            }
        }

        // Execute other non-map scripts.
        let map_sid = self.map_sid;
        self.execute_procs(proc, ctx, |sid| Some(sid) != map_sid);
    }

    fn execute_predefined_proc0(
        script: &mut Script,
        vm: &mut Vm,
        sid: Sid,
        proc: PredefinedProc,
        vars: &mut Vars,
        ctx: &mut Context)
    {
        // FIXME
        // #511 == animfrfv.int
        if script.program_id != 511 {
            return;
        }
        let vm_ctx = &mut vm::Context {
            local_vars: &mut script.local_vars,
            map_vars: &mut vars.map_vars,
            global_vars: &mut vars.global_vars,
            external_vars: &mut vars.external_vars,
            self_obj: None,
            world: ctx.world,
            sequencer: ctx.sequencer,
        };
        if !script.inited {
            debug!("[{:?}#{}] running program initialization code", sid, script.program_id);
            vm.run(script.program, vm_ctx).unwrap();
            script.inited = true;
        }
        vm_ctx.self_obj = script.object;
        debug!("[{:?}#{}] executing predefined proc {:?}", sid, script.program_id, proc);
        vm.execute_predefined_proc(script.program, proc, vm_ctx).unwrap();
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
}
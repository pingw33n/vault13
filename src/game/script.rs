use log::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io;
use std::rc::Rc;

use crate::asset::script::db::ScriptDb;
use crate::asset::script::Sid;
use crate::game::object;
use crate::vm::{self, PredefinedProc, Vm};

pub struct Script {
    /// Whether the program's initialization code has been run.
    pub inited: bool,
    pub program_id: u32,
    pub program: vm::Handle,
    pub object: Option<object::Handle>,
}

pub struct Scripts {
    db: ScriptDb,
    vm: Vm,
    programs: HashMap<u32, Rc<vm::Program>>,
    scripts: HashMap<Sid, Script>,
    map_sid: Option<Sid>,
}

impl Scripts {
    pub fn new(db: ScriptDb, vm: Vm) -> Self {
        Self {
            db,
            vm,
            programs: HashMap::new(),
            scripts: HashMap::new(),
            map_sid: None,
        }
    }

    pub fn instantiate(&mut self, sid: Sid, program_id: u32) -> io::Result<()> {
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
        let program = self.vm.insert(program);
        let existing = self.scripts.insert(sid, Script {
            inited: false,
            program_id,
            program,
            object: None,
        });
        if let Some(existing) = existing {
            panic!("{:?} #{} duplicates existing #{}", sid, program_id, existing.program_id);
        }
        Ok(())
    }

    pub fn get(&self, sid: Sid) -> Option<&Script> {
        self.scripts.get(&sid)
    }

    pub fn attach_to_object(&mut self, sid: Sid, obj: &object::Handle) {
        self.scripts.get_mut(&sid).unwrap().object = Some(obj.clone());
    }

    pub fn set_map_sid(&mut self, map_sid: Option<Sid>) {
        self.map_sid = map_sid;
    }

    pub fn execute_predefined_proc(&mut self, sid: Sid, proc: PredefinedProc,
        ctx: &mut vm::Context)
    {
        Self::execute_predefined_proc0(self.scripts.get_mut(&sid).unwrap(), &mut self.vm,
            sid, proc, ctx)
    }

    pub fn execute_procs(&mut self, proc: PredefinedProc, ctx: &mut vm::Context,
        filter: impl Fn(Sid) -> bool)
    {
        for (&sid, script) in self.scripts.iter_mut() {
            if filter(sid) {
                Self::execute_predefined_proc0(script, &mut self.vm, sid, proc, ctx);
            }
        }
    }

    pub fn execute_map_procs(&mut self, proc: PredefinedProc, ctx: &mut vm::Context) {
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
        ctx: &mut vm::Context)
    {
        // FIXME
        // #511 == animfrfv.int
        if script.program_id != 511 {
            return;
        }
        ctx.self_obj = script.object.clone();
        if !script.inited {
            debug!("[{:?}#{}] running program initialization code", sid, script.program_id);
            vm.run(&script.program, ctx).unwrap();
            script.inited = true;
        }
        debug!("[{:?}#{}] executing predefined proc {:?}", sid, script.program_id, proc);
        vm.execute_predefined_proc(&script.program, proc, ctx).unwrap();
    }
}
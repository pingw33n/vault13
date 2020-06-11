//! # Variable scopes
//!
//! In general variables can have the following visibility:
//!
//! * `game` - a variable is visible from all programs of all maps.
//! * `map` - a variable is visible from all programs of the same map.
//! * `program` - a variable is visible only within program.
//! * `procedure` - a variable is visible only within procedure.
//!
//! ## Procedure variables
//!
//! Instructions: all stack instructions.
//! Visibility: `procedure`.
//! Persistent: no.
//! Identifier type: n/a.
//! Value type: any.
//!
//! These are regular variables created in procedure body. They are stored on stack and destroyed
//! when procedure returns.
//!
//! ## Program global variables
//!
//! Instructions: `fetch_global`, `store_global`.
//! Visibility: `program`.
//! Persistent: no.
//! Identifier type: `int`.
//! Value type: any.
//!
//! These are non-persistent global variables in program. Accessible from all procedures of that
//! program.
//!
//! Transient program variables stored at special offset on the data stack. This location is
//! initialized by program initialization code.
//!
//! ## External variables
//!
//! Instructions: `fetch_external`, `store_external`.
//! Visibility: `map`.
//! Persistent: no.
//! Identifier type: `string`.
//! Value type: any.
//!
//! External variables are shared across programs. External variables cleared on map switch.
//!
//! ## Map local variables (LVAR)
//!
//! Instructions: `local_var`, `set_local_var`.
//! Visibility: `program`.
//! Persistent: yes.
//! Identifier type: `int`.
//! Value type: `int`.
//!
//! Called simply "local" or "LVAR". Such variables are persistent `int` values bound to specific
//! program instance within its map.
//!
//! Local variables are stored as part of map in linear array of integers for all local variables
//! of all scripts. Each script inside map then have a pointer into this array where its variables
//! are stored. In `scripts.lst` near to each program name there's `local_vars` field that declares
//! the number of its local variables. This value is used when serializing local variables inside
//! map.
//!
//! ## Map global variables (MVAR)
//!
//! Instructions: `map_var`, `set_map_var`.
//! Visibility: `map`.
//! Persistent: yes.
//! Identifier type: `int`.
//! Value type: `int`.
//!
//! Map global variables are persistent `int` values bound to a map.
//!
//! Similarly to map local variables the map variables are stored as part of map. The number
//! of map variables are defined for each map in `.gam` files.
//!
//! ## Game global variables (GVAR)
//!
//! Instructions: `global_var`, `set_global_var`.
//! Visibility: `game`.
//! Persistent: yes.
//! Identifier type: `int`.
//! Value type: `int`.
//!
//! Called simply "global" or GVAR. Such variables are persistent `int` values that exist for the
//! whole game session duration (single savegame).
//!
//! Stored in `save.dat`. Defined in `vault13.gam`.

mod error;
mod instruction;
mod stack;
pub mod value;

use bstring::{bstr, BString};
use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use enumflags2::BitFlags;
use enumflags2_derive::EnumFlags;
use log::*;
use matches::matches;
use slotmap::{SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Cursor};
use std::rc::Rc;
use std::str;
use std::time::Duration;

use crate::game::object;
use crate::game::script::{NewScripts, ScriptKind};
use crate::util::SmKey;

use instruction::{Instruction, instruction_map, Opcode};
use stack::{Stack, StackId};

pub use error::*;
pub use value::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredefinedProc {
    Combat,
    CombatIsOver,
    CombatIsStarting,
    Create,
    Critter,
    Damage,
    Description,
    Destroy,
    Drop,
    IsDropping,
    LookAt,
    MapEnter,
    MapExit,
    MapUpdate,
    Pickup,
    Push,
    Spatial,
    Start,
    Talk,
    TimedEvent,
    Use,
    UseObjOn,
    UseSkillOn,
}

impl PredefinedProc {
    pub fn name(self) -> &'static str {
        use PredefinedProc::*;
        match self {
            Combat => "combat_is_over_p_proc",
            CombatIsOver => "combat_is_starting_p_proc",
            CombatIsStarting => "combat_p_proc",
            Create => "create_p_proc",
            Critter => "critter_p_proc",
            Damage => "damage_p_proc",
            Description => "description_p_proc",
            Destroy => "destroy_p_proc",
            Drop => "drop_p_proc",
            IsDropping => "is_dropping_p_proc",
            LookAt => "look_at_p_proc",
            MapEnter => "map_enter_p_proc",
            MapExit => "map_exit_p_proc",
            MapUpdate => "map_update_p_proc",
            Pickup => "pickup_p_proc",
            Push => "push_p_proc",
            Spatial => "spatial_p_proc",
            Start => "start",
            Talk => "talk_p_proc",
            TimedEvent => "timed_event_p_proc",
            Use => "use_p_proc",
            UseObjOn => "use_obj_on_p_proc",
            UseSkillOn => "use_skill_on_p_proc",
        }
    }
}

impl fmt::Display for PredefinedProc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Suspend {
    GsayEnd,
}

/// Result of program invocation.
#[derive(Clone, Copy, Debug, Default)]
#[must_use]
pub struct InvocationResult {
    /// Whether the `script_overrides()` instruction has been called at least once.
    /// Note this flag won't be carried over on resume.
    /// Semantically a set `script_overrides` flag means the caller should not run the default
    /// logic. For example for `PredefinedProc::Description` it should assume that the script
    /// has pushed its description to the message panel.
    pub script_overrides: bool,
    pub suspend: Option<Suspend>,
}

impl InvocationResult {
    pub fn assert_no_suspend(&self) -> &Self {
        if let Some(s) = self.suspend {
            panic!("unexpected suspend: {:?}", s);
        }
        self
    }
}

pub struct Context<'a> {
    /// Program local variables.
    pub local_vars: &'a mut [i32],

    /// Map variables.
    pub map_vars: &'a mut [i32],

    /// Global game variables.
    pub global_vars: &'a mut [i32],

    /// External variables.
    pub external_vars: &'a mut HashMap<Rc<BString>, Option<Value>>,

    pub self_obj: Option<object::Handle>,
    pub source_obj: Option<object::Handle>,
    pub target_obj: Option<object::Handle>,
    pub skill: Option<crate::asset::Skill>,
    pub ui: &'a mut crate::ui::Ui,
    pub world: &'a mut crate::game::world::World,
    pub sequencer: &'a mut crate::sequence::Sequencer,
    pub dialog: &'a mut Option<crate::game::dialog::Dialog>,
    pub message_panel: crate::ui::Handle,
    pub script_db: &'a mut crate::asset::script::db::ScriptDb,
    pub new_scripts: NewScripts,
    pub proto_db: &'a crate::asset::proto::ProtoDb,
    pub map_id: crate::asset::map::MapId,
    pub rpg: &'a mut crate::game::rpg::Rpg,
}

impl Context<'_> {
    pub fn has_running_sequence(&self, obj: object::Handle) -> bool {
        self.world.objects().get(obj).has_running_sequence()
    }
}

pub struct VmConfig {
    instructions: HashMap<u16, Instruction>,
    max_stack_len: usize,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            instructions: instruction_map(),
            max_stack_len: 2000,
        }
    }
}

pub struct StringMap {
    map: HashMap<usize, Rc<BString>>,
}

impl StringMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: usize, s: Rc<BString>) {
        self.map.insert(id, s);
    }

    pub fn get(&self, id: usize) -> Option<&Rc<BString>> {
        self.map.get(&id)
    }
}

pub type ProcedureId = u32;

#[derive(Clone, Copy, Debug, EnumFlags, Eq, PartialEq)]
#[repr(u32)]
pub enum ProcedureFlag {
    Timed           = 0x01,
    Conditional     = 0x02,
    Import          = 0x04,
    Export          = 0x08,
    Critical        = 0x10,
}

#[derive(Debug)]
pub struct Procedure {
    name: Rc<BString>,
    flags: BitFlags<ProcedureFlag>,
    delay: Duration,
    condition_pos: usize,
    body_pos: usize,
    arg_count: usize,
}

impl Procedure {
    pub fn name(&self) -> &bstr {
        self.name.as_bstr()
    }
}

struct Procs {
    by_id: Vec<Procedure>,
    by_name: HashMap<Rc<BString>, ProcedureId>,
}

pub struct Program {
    name: String,
    config: Rc<VmConfig>,
    code: Box<[u8]>,
    names: StringMap,
    strings: StringMap,
    procs: Procs,
}

impl Program {
    fn new(name: String, code: Box<[u8]>, config: Rc<VmConfig>) -> Result<Self> {
        const PROC_TABLE_START: usize = 42;
        const PROC_TABLE_HEADER_LEN: usize = 4;
        const PROC_ENTRY_LEN: usize = 24;

        if code.len() < PROC_ENTRY_LEN + PROC_TABLE_HEADER_LEN {
            return Err(Error::BadMetadata("missing procedure table".into()));
        }

        let proc_count = BigEndian::read_i32(&code[PROC_TABLE_START..]) as usize;

        let name_table_start = PROC_TABLE_START + PROC_TABLE_HEADER_LEN +
            proc_count * PROC_ENTRY_LEN;
        debug!("reading name table at 0x{:04x}", name_table_start);
        let (names, name_table_len_bytes) =
            Self::read_string_table(&code[name_table_start..])?;

        let string_table_start = name_table_start + name_table_len_bytes;
        debug!("reading string table at 0x{:04x}", string_table_start);
        let (strings, _) =
            Self::read_string_table(&code[string_table_start..])?;

        debug!("reading procedure table at 0x{:04x}", PROC_TABLE_START);
        let procs = Self::read_proc_table(&code[PROC_TABLE_START..], &names)?;

        Ok(Self {
            name,
            config,
            code,
            names,
            strings,
            procs,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn proc(&self, id: ProcedureId) -> Option<&Procedure> {
        self.procs.by_id.get(id as usize)
    }

    pub fn proc_id(&self, name: &Rc<BString>) -> Option<ProcedureId> {
        self.procs.by_name.get(name).cloned()
    }

    pub fn predefined_proc_id(&self, proc: PredefinedProc) -> Option<ProcedureId> {
        self.proc_id(&Rc::new(proc.name().into()))
    }

    fn read_string_table(buf: &[u8]) -> Result<(StringMap, usize)> {
        let mut rd = Cursor::new(buf);
        let mut read = || -> io::Result<(StringMap, usize)> {
            let mut r = StringMap::new();

            let total_len_bytes = rd.read_u32::<BigEndian>()? as usize;
            if total_len_bytes == 0xffff_ffff {
                return Ok((r, 4));
            }
            let total_len_bytes = total_len_bytes + 8;

            loop {
                let len = rd.read_u16::<BigEndian>()? as usize;
                if len == 0xffff {
                    rd.read_u16::<BigEndian>()?;
                    break;
                }
                let start = rd.position() as usize;
                let end = start + len;
                if end > buf.len() {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                let s = &buf[start..end];
                let s = if let Some(i) = s.iter().position(|&b| b == 0) {
                    &s[..i]
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                        "name or string table: string is not null-terminated"));
                };
                let s = BString::from(s);
                debug!("string {}: \"{}\"", start, s.display());
                r.insert(start, Rc::new(s));

                rd.set_position(end as u64);
            }
            if rd.position() as usize != total_len_bytes {
               warn!("name or string table ended unexpectedly");
            }
            Ok((r, total_len_bytes))
        };
        Self::map_io_err(read())
    }

    fn read_proc_table(buf: &[u8], names: &StringMap) -> Result<Procs> {
        let mut rd = Cursor::new(buf);
        let mut read = || -> io::Result<Vec<Procedure>> {
            let count = rd.read_u32::<BigEndian>()? as usize;
            let mut r = Vec::with_capacity(count);
            for i in 0..count {
                let name = rd.read_u32::<BigEndian>()? as usize;
                let name = names.get(name)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData,
                        "invalid procedure name reference"))?
                    .clone();
                let flags = BitFlags::from_bits(rd.read_u32::<BigEndian>()?)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData,
                        "invalid procedure flags"))?;
                let delay = Duration::from_millis(rd.read_u32::<BigEndian>()? as u64);
                let condition_pos = rd.read_u32::<BigEndian>()? as usize;
                let body_pos = rd.read_u32::<BigEndian>()? as usize;
                let arg_count = rd.read_u32::<BigEndian>()? as usize;

                let proc = Procedure {
                    name: name.clone(),
                    flags,
                    delay,
                    condition_pos,
                    body_pos,
                    arg_count,
                };
                debug!("procedure {} {}({}): {:#?}", i, proc.name.display(),
                    if proc.arg_count > 0 { "..." } else { "" },
                    proc);

                r.push(proc);
            }
            Ok(r)
        };
        let by_id = Self::map_io_err(read())?;
        let mut by_name = HashMap::with_capacity(by_id.len());
        for (i, proc) in by_id.iter().enumerate() {
            if by_name.contains_key(&proc.name) {
                return Self::map_io_err(Err(io::Error::new(io::ErrorKind::InvalidData,
                    format!("duplicate procedure name: {}", proc.name.display()))));
            }
            by_name.insert(proc.name.clone(), i as ProcedureId);
        }
        Ok(Procs { by_id, by_name })
    }

    fn map_io_err<T>(r: io::Result<T>) -> Result<T> {
        r.map_err(|e| match e.kind() {
            io::ErrorKind::UnexpectedEof => Error::UnexpectedEof,
            _ => Error::BadMetadata(e.to_string().into()),
        })
    }
}

pub struct ProgramState {
    program: Rc<Program>,
    code_pos: usize,
    opcode: Option<(Opcode, usize)>,
    pub data_stack: Stack<DataStackId>,
    pub return_stack: Stack<ReturnStackId>,
    /// Base offset in `data_stack` of procedure variables.
    base:  Option<usize>,
    /// Base offset in `data_stack` of program global variables.
    global_base: Option<usize>,
    instr_state: instruction::State,
    /// Stack of code positions where suspend requested.
    suspend_stack: Vec<usize>,
}

impl ProgramState {
    fn new(program: Rc<Program>) -> Self {
        let data_stack = Stack::new(program.config.max_stack_len);
        let return_stack = Stack::new(program.config.max_stack_len);

        Self {
            program,
            code_pos: 0,
            opcode: None,
            data_stack,
            return_stack,
            base: None,
            global_base: None,
            instr_state: instruction::State::new(),
            suspend_stack: Vec::new(),
        }
    }

    fn code(&self) -> &[u8] {
        &self.program.code
    }

    fn names(&self) -> &StringMap {
        &self.program.names
    }

    fn strings(&self) -> &StringMap {
        &self.program.strings
    }

    fn run(&mut self, ctx: &mut Context) -> Result<InvocationResult> {
        self.instr_state.script_overrides = false;
        let suspend = loop {
            match self.step(ctx) {
                Ok(r) => {
                    if let Some(s) = r {
                        debug!("suspending at 0x{:x}: {:?}", self.code_pos, s);
                        self.suspend_stack.push(self.code_pos);
                        break Some(s);
                    }
                }
                Err(ref e) if matches!(e, Error::Halted) => break None,
                Err(e) => return Err(e),
            }
        };
        Ok(InvocationResult {
            suspend,
            script_overrides: self.instr_state.script_overrides,
        })
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn execute_proc(&mut self, id: ProcedureId, ctx: &mut Context) -> Result<InvocationResult> {
        let proc_pos = self.program.proc(id)
            .ok_or_else(|| Error::BadProcedureId(id))?
            .body_pos;

        // setupCallWithReturnVal()
        self.return_stack.push(Value::Int(self.code_pos as i32))?;
        // TODO How important is this? The value varies in different call places.
        self.return_stack.push(Value::Int(24))?;
        self.data_stack.push(Value::Int(0))?; // flags
        self.data_stack.push(Value::Int(0))?; //unk17_
        self.data_stack.push(Value::Int(0))?; //unk19_

        self.data_stack.push(Value::Int(0))?;

        self.code_pos = proc_pos;

        self.run(ctx)
    }

    pub fn can_resume(&self) -> bool {
        !self.suspend_stack.is_empty()
    }

    pub fn resume(&mut self, ctx: &mut Context) -> Result<InvocationResult> {
        self.code_pos = self.suspend_stack.pop().unwrap();
        self.run(ctx)
    }

    fn step(&mut self, ctx: &mut Context) -> Result<Option<Suspend>> {
        trace!("code_pos: 0x{:04x}", self.code_pos);
        let opcode_pos = self.code_pos;
        let instr = self.next_instruction()?;
        self.opcode = Some((instr.opcode(), opcode_pos));
        let suspend = instr.execute(instruction::Context {
            prg: self,
            ext: ctx,
        })?;
        Ok(suspend)
    }

    fn next_instruction(&mut self) -> Result<Instruction> {
        let opcode = self.get_u16()?;
        trace!("opcode: 0x{:04x}", opcode);
        if let Some(&instr) = self.program.config.instructions.get(&opcode) {
            trace!("opcode recognized: {:?}", instr.opcode());
            self.code_pos += 2;
            Ok(instr)
        } else {
            Err(Error::BadOpcode(opcode))
        }
    }

    fn get_u16(&mut self) -> Result<u16> {
        if self.code_pos + 2 <= self.code().len() {
            Ok(BigEndian::read_u16(&self.code()[self.code_pos..]))
        } else {
            Err(Error::UnexpectedEof)
        }
    }

    fn get_i32(&mut self) -> Result<i32> {
        if self.code_pos + 4 <= self.code().len() {
            Ok(BigEndian::read_i32(&self.code()[self.code_pos..]))
        } else {
            Err(Error::UnexpectedEof)
        }
    }

    fn next_i32(&mut self) -> Result<i32> {
        let r =  self.get_i32();
        if r.is_ok() {
            self.code_pos += 4
        }
        r
    }

    fn get_f32(&mut self) -> Result<f32> {
        if self.code_pos + 4 <= self.code().len() {
            Ok(BigEndian::read_f32(&self.code()[self.code_pos..]))
        } else {
            Err(Error::UnexpectedEof)
        }
    }

    fn next_f32(&mut self) -> Result<f32> {
        let r =  self.get_f32();
        if r.is_ok() {
            self.code_pos += 4
        }
        r
    }

    fn jump(&mut self, pos: i32) -> Result<()> {
        if pos >= 0 && pos + Opcode::SIZE as i32 <= self.code().len() as i32 {
            self.code_pos = pos as usize;
            Ok(())
        } else {
            Err(Error::BadValue(BadValue::Content))
        }
    }

    fn base(&self) -> Result<usize> {
        self.base
            .ok_or_else(|| Error::BadState("no base set".into()))
    }

    fn base_encoded(&self) -> i32 {
        self.base.map(|v| v as i32).unwrap_or(-1)
    }

    fn set_base_encoded(&mut self, v: i32) {
        self.base = if v < 0 { None } else { Some(v as usize) };
    }

    fn base_val(&self, id: usize) -> Result<&Value> {
        let base = self.base()?;
        self.data_stack.get(base + id as usize)
    }

    fn base_val_mut(&mut self, id: usize) -> Result<&mut Value> {
        let base = self.base()?;
        self.data_stack.get_mut(base + id as usize)
    }

    fn global_base(&self) -> Result<usize> {
        self.global_base
            .ok_or_else(|| Error::BadState("no global_base set".into()))
    }

    fn global(&self, id: usize) -> Result<&Value> {
        let base = self.global_base()?;
        self.data_stack.get(base + id as usize)
    }

    fn global_mut(&mut self, id: usize) -> Result<&mut Value> {
        let base = self.global_base()?;
        self.data_stack.get_mut(base + id as usize)
    }
}

pub struct DataStackId;

impl StackId for DataStackId {
    const VALUE: &'static str = "data";
}

pub struct ReturnStackId;

impl StackId for ReturnStackId {
    const VALUE: &'static str = "return";
}

/// Program state handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Handle(SmKey);

pub struct Vm {
    config: Rc<VmConfig>,
    program_handles: SlotMap<SmKey, ()>,
    program_states: SecondaryMap<SmKey, ProgramState>,
}

impl Vm {
    pub fn new(config: Rc<VmConfig>) -> Self {
        Self {
            config,
            program_handles: SlotMap::with_key(),
            program_states: SecondaryMap::new(),
        }
    }

    pub fn load(&self, name: String, code: Box<[u8]>) -> Result<Program> {
        Program::new(name, code, self.config.clone())
    }

    pub fn insert(&mut self, program: Rc<Program>) -> Handle {
        let program_state = ProgramState::new(program);
        let k = self.program_handles.insert(());
        self.program_states.insert(k, program_state);
        Handle(k)
    }

    pub fn run(&mut self, program: Handle, ctx: &mut Context) -> Result<InvocationResult> {
        self.program_state_mut(program).run(ctx)
    }

    pub fn program_state(&self, handle: Handle) -> &ProgramState {
         self.program_states.get(handle.0)
            .expect("invalid program handle")
    }

    pub fn program_state_mut(&mut self, handle: Handle) -> &mut ProgramState {
         self.program_states.get_mut(handle.0)
            .expect("invalid program handle")
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
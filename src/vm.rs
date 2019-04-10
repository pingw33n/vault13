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
//! External program variables are shared across programs. External variables cleared on map switch.
//!
//! ## Program local variables (LVAR)
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
//! ## Map variables (MVAR)
//!
//! Instructions: `map_var`, `set_map_var`.
//! Visibility: `map`.
//! Persistent: yes.
//! Identifier type: `int`.
//! Value type: `int`.
//!
//! Map variables are persistent `int` values bound to a map.
//!
//! Similarly to program local variables the map variables are stored as part of map. The number
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
mod value;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use enumflags::BitFlags;
use enumflags_derive::EnumFlags;
use log::*;
use matches::matches;
use slotmap::{SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Cursor};
use std::result::{Result as StdResult};
use std::rc::Rc;
use std::str;
use std::time::Duration;

pub use error::*;
use instruction::{Instruction, instruction_map, Opcode};
use stack::{Stack, StackId};
use value::Value;
use crate::game::object;
use crate::util::SmKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredefinedProc {
    MapEnter,
    MapUpdate,
    MapExit,
    Start,
}

impl PredefinedProc {
    pub fn name(&self) -> &'static str {
        use PredefinedProc::*;
        match self {
            MapEnter => "map_enter_p_proc",
            MapUpdate => "map_update_p_proc",
            MapExit => "map_exit_p_proc",
            Start => "start",
        }
    }
}

impl fmt::Display for PredefinedProc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

pub struct Context<'a> {
    /// External variables.
    pub external_vars: &'a mut HashMap<Rc<String>, Option<Value>>,

    /// Global game variables.
    pub global_vars: &'a mut Vec<i32>,

    pub self_obj: Option<object::Handle>,
    pub world: &'a mut crate::game::world::World,
    pub sequencer: &'a mut crate::sequence::Sequencer,
}

impl Context<'_> {
    pub fn has_running_sequence(&self, obj: object::Handle) -> bool {
        self.world.objects().get(obj).borrow().has_running_sequence()
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
    vec: Vec<(usize, Rc<String>)>,
}

impl StringMap {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
        }
    }

    pub fn insert(&mut self, id: usize, s: Rc<String>) {
        let entry = (id, s);
        match self.find_idx(id) {
            Ok(i) => self.vec[i] = entry,
            Err(i) => self.vec.insert(i, entry),
        }
    }

    pub fn get(&self, id: usize) -> Option<&Rc<String>> {
        match self.find_idx(id) {
            Ok(i) => Some(&self.vec[i].1),
            Err(_) => None,
        }
    }

    fn find_idx(&self, id: usize) -> StdResult<usize, usize> {
        self.vec.binary_search_by_key(&id, |e| e.0)
    }
}

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
    name: Rc<String>,
    flags: BitFlags<ProcedureFlag>,
    delay: Duration,
    condition_pos: usize,
    body_pos: usize,
    arg_count: usize,
}

pub struct Program {
    config: Rc<VmConfig>,
    code: Box<[u8]>,
    names: StringMap,
    strings: StringMap,
    procs: HashMap<Rc<String>, Procedure>,
}

impl Program {
    fn new(config: Rc<VmConfig>, code: Box<[u8]>) -> Result<Self> {
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
            config,
            code,
            names,
            strings,
            procs,
        })
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
                let s = str::from_utf8(s)
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData,
                        "name or string table: string is not valid UTF-8 sequence"))?;
                debug!("string {}: \"{}\"", start, s);
                r.insert(start, Rc::new(s.to_owned()));

                rd.set_position(end as u64);
            }
            if rd.position() as usize != total_len_bytes {
               warn!("name or string table ended unexpectedly");
            }
            Ok((r, total_len_bytes))
        };
        Self::map_io_err(read())
    }

    fn read_proc_table(buf: &[u8], names: &StringMap) -> Result<HashMap<Rc<String>, Procedure>> {
        let mut rd = Cursor::new(buf);
        let mut read = || -> io::Result<HashMap<Rc<String>, Procedure>> {
            let count = rd.read_u32::<BigEndian>()? as usize;
            let mut r = HashMap::with_capacity(count);
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
                debug!("procedure {} {}({}): {:#?}", i, proc.name,
                    if proc.arg_count > 0 { "..." } else { "" },
                    proc);

                if r.contains_key(&name) {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                        format!("duplicate procedure name: {}", name)));
                }

                r.insert(name, proc);
            }
            Ok(r)
        };
        Self::map_io_err(read())
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
    // FIXME check if this an Option
    base: isize,
    global_base: Option<usize>,
    instr_state: instruction::State,
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
            base: -1,
            global_base: None,
            instr_state: instruction::State::new(),
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

    fn run(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            match self.step(ctx) {
                Ok(_) => {},
                Err(ref e) if matches!(e, Error::Halted) => break Ok(()),
                Err(e) => break Err(e),
            }
        }
    }

    pub fn execute_proc(&mut self, name: &Rc<String>, ctx: &mut Context) -> Result<()> {
        let proc_pos = self.program.procs.get(name)
            .ok_or_else(|| Error::BadProcedure(name.clone()))?
            .body_pos;

        self.return_stack.push(Value::Int(self.code_pos as i32))?;
        self.return_stack.push(Value::Int(20))?;
        self.data_stack.push(Value::Int(0))?; // flags
        self.data_stack.push(Value::Int(0))?; //unk17_
        self.data_stack.push(Value::Int(0))?; //unk19_
        self.code_pos = proc_pos;

        self.run(ctx)
    }

    fn step(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("code_pos: 0x{:04x}", self.code_pos);
        let opcode_pos = self.code_pos;
        let instr = self.next_instruction()?;
        self.opcode = Some((instr.opcode(), opcode_pos));
        instr.execute(instruction::Context {
            prg: self,
            ext: ctx,
        })
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

    fn jump(&mut self, pos: i32) -> Result<()> {
        if pos >= 0 && pos + Opcode::SIZE as i32 <= self.code().len() as i32 {
            self.code_pos = pos as usize;
            Ok(())
        } else {
            Err(Error::BadValue(BadValue::Content))
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Handle(SmKey);

pub struct Vm {
    config: Rc<VmConfig>,
    program_handles: SlotMap<SmKey, ()>,
    programs: SecondaryMap<SmKey, ProgramState>,
}

impl Vm {
    pub fn new(config: Rc<VmConfig>) -> Self {
        Self {
            config,
            program_handles: SlotMap::with_key(),
            programs: SecondaryMap::new(),
        }
    }

    pub fn load(&self, code: Box<[u8]>) -> Result<Program> {
        Program::new(self.config.clone(), code)
    }

    pub fn insert(&mut self, program: Rc<Program>) -> Handle {
        let program_state = ProgramState::new(program);
        let k = self.program_handles.insert(());
        self.programs.insert(k, program_state);
        Handle(k)
    }

    pub fn run(&mut self, program: Handle, ctx: &mut Context) -> Result<()> {
       self.program(program).run(ctx)
    }

    pub fn execute_proc(&mut self, program: Handle, name: &Rc<String>, ctx: &mut Context)
        -> Result<()>
    {
        self.program(program).execute_proc(name, ctx)
    }

    pub fn execute_predefined_proc(&mut self, program: Handle, proc: PredefinedProc, ctx: &mut Context)
        -> Result<()>
    {
        self.execute_proc(program, &Rc::new(proc.to_string()), ctx)
    }

    fn program(&mut self, program: Handle) -> &mut ProgramState {
         self.programs.get_mut(program.0)
            .expect("invalid program handle")
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
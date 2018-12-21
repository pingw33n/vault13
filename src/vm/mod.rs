mod error;
mod instruction;
mod stack;
mod value;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use enumflags::BitFlags;
use std::collections::HashMap;
use std::io::{self, Cursor};
use std::result::{Result as StdResult};
use std::rc::Rc;
use std::str;
use std::time::Duration;

use self::error::*;
use self::instruction::{Instruction, instruction_map, Opcode};
use self::stack::Stack;
use self::value::Value;

pub struct Context<'a> {
    pub external_vars: &'a mut HashMap<Rc<String>, Value>,
    pub global_vars: &'a mut Vec<i32>,
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

pub struct VmState {
    code: Box<[u8]>,
    code_pos: usize,
    opcode: Option<(Opcode, usize)>,
    data_stack: Stack,
    return_stack: Stack,
    // FIXME check if this an Option
    base: isize,
    // FIXME check if this an Option
    global_base: isize,
    names: StringMap,
    strings: StringMap,
    procs: HashMap<Rc<String>, Procedure>,
}

impl VmState {
    pub fn data_stack(&self) -> &Stack {
        &self.data_stack
    }

    pub fn return_stack(&self) -> &Stack {
        &self.return_stack
    }

    fn new(config: &VmConfig,
        code: Box<[u8]>,
        names: StringMap,
        strings: StringMap,
        procs: HashMap<Rc<String>, Procedure>) -> Self
    {
        Self {
            code,
            code_pos: 0,
            opcode: None,
            data_stack: Stack::new(config.max_stack_len),
            return_stack: Stack::new(config.max_stack_len),
            base: -1,
            global_base: -1,
            names,
            strings,
            procs,
        }
    }

    fn get_u16(&mut self) -> Result<u16> {
        if self.code_pos + 2 <= self.code.len() {
            Ok(BigEndian::read_u16(&self.code[self.code_pos..]))
        } else {
            Err(Error::UnexpectedEof)
        }
    }

    fn get_i32(&mut self) -> Result<i32> {
        if self.code_pos + 4 <= self.code.len() {
            Ok(BigEndian::read_i32(&self.code[self.code_pos..]))
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
        if pos >= 0 && pos + Opcode::SIZE as i32 <= self.code.len() as i32 {
            self.code_pos = pos as usize;
            Ok(())
        } else {
            Err(Error::BadValue(BadValue::Content))
        }
    }
}

pub struct Vm {
    config: Rc<VmConfig>,
    state: VmState,
}

impl Vm {
    pub fn new(config: Rc<VmConfig>, code: Box<[u8]>) -> Result<Self> {
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

        let state = VmState::new(&config, code, names, strings, procs);
        Ok(Self {
            config,
            state,
        })
    }

    pub fn state(&self) -> &VmState {
        &self.state
    }

    pub fn execute_proc(&mut self, name: &Rc<String>, ctx: &mut Context) -> Result<()> {
        let proc_pos = self.state.procs.get(name)
            .ok_or_else(|| Error::BadProcedure(name.clone()))?
            .body_pos;

        self.state.return_stack.push(Value::Int(self.state.code_pos as i32))?;
        self.state.return_stack.push(Value::Int(20))?;
        self.state.data_stack.push(Value::Int(0))?; // flags
        self.state.data_stack.push(Value::Int(0))?; //unk17_
        self.state.data_stack.push(Value::Int(0))?; //unk19_
        self.state.code_pos = proc_pos;

        self.run(ctx)
    }

    pub fn run(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            match self.step(ctx) {
                Ok(_) => {},
                Err(ref e) if matches!(e, Error::Halted) => break Ok(()),
                Err(e) => break Err(e),
            }
        }
    }

    pub fn step(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("code_pos: 0x{:04x}", self.state.code_pos);
        let opcode_pos = self.state.code_pos;
        let instr = self.next_instruction()?;
        self.state.opcode = Some((instr.opcode(), opcode_pos));
        instr.execute(instruction::Context {
            vm_state: &mut self.state,
            ext: ctx,
        })
    }

    fn next_instruction(&mut self) -> Result<Instruction> {
        let opcode = self.state.get_u16()?;
        trace!("opcode: 0x{:04x}", opcode);
        if let Some(&instr) = self.config.instructions.get(&opcode) {
            trace!("opcode recognized: {:?}", instr.opcode());
            self.state.code_pos += 2;
            Ok(instr)
        } else {
            Err(Error::BadOpcode(opcode))
        }
    }

    fn read_string_table(buf: &[u8]) -> Result<(StringMap, usize)> {
        let mut rd = Cursor::new(buf);
        let mut read = || -> io::Result<(StringMap, usize)> {
            let total_len_bytes = rd.read_u32::<BigEndian>()? as usize + 8;
            let mut r = StringMap::new();
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

use bstring::BString;
use std::borrow::Cow;
use std::rc::Rc;

use super::ProcedureId;
use crate::vm::instruction::Opcode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BadValue {
    Type,
    Content,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    BadInstruction,
    BadMetadata(Cow<'static, str>),
    BadOpcode(u16),
    BadProcedure(Rc<BString>),
    BadProcedureId(ProcedureId),
    BadState(Cow<'static, str>),
    BadValue(BadValue),
    BadExternalVar(Rc<BString>),
    Halted,
    Misc(Cow<'static, str>),
    UnimplementedOpcode(Opcode),
    StackOverflow,
    StackUnderflow,
    UnexpectedEof,
}

impl Error {
    pub fn is_halted(&self) -> bool {
        if let Error::Halted = self {
            true
        } else {
            false
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
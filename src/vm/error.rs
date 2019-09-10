use bstring::BString;
use std::borrow::Cow;
use std::rc::Rc;

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
    BadProcedureId(u32),
    BadState(Cow<'static, str>),
    BadValue(BadValue),
    Halted,
    Misc(Cow<'static, str>),
    UnimplementedOpcode,
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
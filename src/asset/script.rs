pub mod db;

use byteorder::{BigEndian, ReadBytesExt};
use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum ScriptKind {
    System = 0x0,
    Spatial = 0x1,
    Time = 0x2,
    Item = 0x3,
    Critter = 0x4,
}

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
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("malformed SID: {:x}", v)))
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        let v = rd.read_i32::<BigEndian>()?;
        Ok(if v >= 0 {
            Some(Self::from_packed(v as u32)
                .ok_or_else(|| Error::new(ErrorKind::InvalidData,
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

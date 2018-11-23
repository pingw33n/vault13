mod db;

pub use self::db::FrmDb;

use byteorder::{BigEndian, ReadBytesExt};
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use asset::EntityKind;

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Fid(u32);

impl Fid {
    pub fn from_packed(v: u32) -> Option<Self> {
        let r = Fid(v);
        // Check kind.
        r.kind();
        Some(r)
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let v = rd.read_u32::<BigEndian>()?;
        Ok(Self::from_packed(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("malformed FID: {:x}", v))).unwrap())
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        let v = rd.read_i32::<BigEndian>()?;
        Ok(if v >= 0 {
            Some(Self::from_packed(v as u32)
                .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                    format!("malformed FID: {:x}", v)))?)
        } else {
            None
        })
    }

    pub fn kind(self) -> EntityKind {
        EntityKind::from_u32((self.0 >> 24) & 0b1111).unwrap()
    }

    pub fn id3(self) -> u8 {
        (self.0 >> 28) as u8 & 0b111
    }

    pub fn id2(self) -> u8 {
        (self.0 >> 16) as u8 & 0xff
    }

    pub fn id1(self) -> u8 {
        (self.0 >> 12) as u8 & 0b1111
    }

    pub fn id0(self) -> u16 {
        self.0 as u16 & 0b111_1111_1111
    }
}

impl fmt::Debug for Fid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.0)
    }
}
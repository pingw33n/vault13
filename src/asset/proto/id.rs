use byteorder::{BigEndian, ReadBytesExt};
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::asset::EntityKind;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct ProtoId(u32);

impl ProtoId {
    pub const DUDE: Self = unsafe { Self::from_packed_unchecked(0x1000000) };
    pub const SHIV: Self = unsafe { Self::from_packed_unchecked(0x17F) };
    pub const POWER_ARMOR: Self = unsafe { Self::from_packed_unchecked(3) };
    pub const HARDENED_POWER_ARMOR: Self = unsafe { Self::from_packed_unchecked(0xE8) };
    pub const ADVANCED_POWER_ARMOR: Self = unsafe { Self::from_packed_unchecked(0x15C) };
    pub const ADVANCED_POWER_ARMOR_MK2: Self = unsafe { Self::from_packed_unchecked(0x15D) };
    pub const MIRRORED_SHADES: Self = unsafe { Self::from_packed_unchecked(0x1B1) };
    pub const EXIT_AREA_FIRST: Self = unsafe { Self::from_packed_unchecked(0x5000010) };
    pub const EXIT_AREA_LAST: Self = unsafe { Self::from_packed_unchecked(0x5000017) };
    pub const RADIOACTIVE_GOO_FIRST: Self = unsafe { Self::from_packed_unchecked(0x20003D9) };
    pub const RADIOACTIVE_GOO_LAST: Self = unsafe { Self::from_packed_unchecked(0x20003DC) };
    pub const ACTIVE_FLARE: Self = unsafe { Self::from_packed_unchecked(0xCD) };
    pub const ACTIVE_DYNAMITE: Self = unsafe { Self::from_packed_unchecked(0xCE) };
    pub const ACTIVE_PLASTIC_EXPLOSIVE: Self = unsafe { Self::from_packed_unchecked(0xD1) };
    pub const SCROLL_BLOCKER: Self = unsafe { Self::from_packed_unchecked(0x0500000c) };
    pub const BOTTLE_CAPS: Self = unsafe { Self::from_packed_unchecked(0x29) };
    pub const SOLAR_SCORCHER: Self = unsafe { Self::from_packed_unchecked(390) };

    pub fn new(kind: EntityKind, id: u32) -> Option<Self> {
        if id <= 0xffffff {
            Some(Self((kind as u32) << 24 | id))
        } else {
            None
        }
    }

    const unsafe fn from_packed_unchecked(v: u32) -> Self {
        Self(v)
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        let kind = EntityKind::from_u32(v >> 24)?;
        let id = v & 0xffffff;
        Self::new(kind, id)
    }

    pub fn pack(self) -> u32 {
        self.0
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let v = rd.read_u32::<BigEndian>()?;
        Self::from_packed(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("malformed PID: {:x}", v)))
    }

    pub fn read_opt(rd: &mut impl Read) -> io::Result<Option<Self>> {
        let v = rd.read_i32::<BigEndian>()?;
        Ok(if v >= 0 {
            Some(Self::from_packed(v as u32)
                .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                    format!("malformed PID: {:x}", v)))?)
        } else {
            None
        })
    }

    pub fn kind(self) -> EntityKind {
        EntityKind::from_u32(self.0 >> 24).unwrap()
    }

    /// Returns ID that is unique among entities of the same `EntityKind`.
    /// The result is in range `[0..0xffffff]`.
    pub fn id(self) -> u32 {
        self.0 & 0xffffff
    }

    pub fn is_dude(self) -> bool {
        self == Self::DUDE
    }

    pub fn is_exit_area(self) -> bool {
        self >= Self::EXIT_AREA_FIRST && self <= Self::EXIT_AREA_LAST
    }

    pub fn is_radioactive_goo(self) -> bool {
        self >= Self::RADIOACTIVE_GOO_FIRST && self <= Self::RADIOACTIVE_GOO_LAST
    }
}

impl fmt::Debug for ProtoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProtoId(0x{:08x})", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let pid = ProtoId::new(EntityKind::Item, 0).unwrap();
        assert_eq!(pid.id(), 0);
        assert_eq!(pid.pack(), 0x00_000000);
        assert_eq!(pid.is_dude(), false);
        assert_eq!(pid, ProtoId::from_packed(pid.pack()).unwrap());

        let pid = ProtoId::new(EntityKind::Skilldex, 0xffffff).unwrap();
        assert_eq!(pid.id(), 0xffffff);
        assert_eq!(pid.pack(), 0x0a_ffffff);
        assert_eq!(pid.is_dude(), false);
        assert_eq!(pid, ProtoId::from_packed(pid.pack()).unwrap());

        assert!(ProtoId::new(EntityKind::Critter, 0).unwrap().is_dude());
    }
}
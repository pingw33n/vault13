use byteorder::{BigEndian, ReadBytesExt};
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::asset::EntityKind;

/*      PID_ROCK = 0x13,
  PID_SMALL_ENERGY_CELL = 0x26,
  PID_MICRO_FUSION_CELL = 0x27,
  PID_STIMPAK = 0x28,
  PID_BOTTLE_CAPS = 0x29,
  PID_FIRST_AID_KIT = 0x2F,
  PID_ANTIDOTE = 0x31,
  PID_DYNAMITE = 0x33,
  PID_GEIGER_COUNTER = 0x34,
  PID_MENTATS = 0x35,
  PID_STEALTH_BOY = 0x36,
  PID_WATER_CHIP = 0x37,
  PID_HOLODISK = 0x3A,
  PID_MOTION_SENSOR = 0x3B,
  PID_MUTATED_FRUIT = 0x47,
  PID_BIG_BOOK_OF_SCIENCE = 0x49,
  PID_DEANS_ELECTRONICS = 0x4C,
  PID_FLARE = 0x4F,
  PID_FIRST_AID_BOOK = 0x50,
  PID_PLASTIC_EXPLOSIVES = 0x55,
  PID_SCOUT_HANDBOOK = 0x56,
  PID_BUFFOUT = 0x57,
  PID_DOCTORS_BAG = 0x5B,
  PID_PUMP_PARTS = 0x62,
  PID_GUNS_AND_BULLETS = 0x66,
  PID_NUKA_COLA = 0x6A,
  PID_RAD_X = 0x6D,
  PID_PSYCHO = 0x6E,
  PID_SUPER_STIMPAK = 0x90,
  PID_ACTIVE_FLARE = 0xCD,
  PID_ACTIVE_DYNAMITE = 0xCE,
  PID_ACTIVE_GEIGER_COUNTER = 0xCF,
  PID_ACTIVE_MOTION_SENSOR = 0xD0,
  PID_ACTIVE_PLASTIC_EXPLOSIVE = 0xD1,
  PID_ACTIVE_STEALTH_BOY = 0xD2,
  PID_TECHNICAL_MANUAL = 0xE4,
  PID_CHEMISTRY_MANUAL = 0xED,
  PID_JET = 0x103,
  PID_JET_ANTIDOTE = 0x104,
  PID_GECK = 0x16E,
  PID_CAR_TRUNK = 0x1C7,
  PID_JESSE_CONTAINER = 0x1D3,
  PID_DUDE = 0x1000000,
  PID_DRIVABLE_CAR = 0x20003F1,
  PID_NULL = 0xFFFFFFFF,

  PID_HARDENED_POWER_ARMOR = 0xE8,
  PID_ADVANCED_POWER_ARMOR = 0x15C,
  PID_ADVANCED_POWER_ARMOR_MK2 = 0x15D,
  PID_POWER_ARMOR = 0x3,
  PID_MIRRORED_SHADES = 0x1B1,
  PID_SCROLL_BLOCKER = 0x500000C,*/

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct ProtoId(u32);

impl ProtoId {
    pub const SHIV: ProtoId = ProtoId(0x17F);
    pub const EXIT_AREA_FIRST: ProtoId = ProtoId(0x5000010);
    pub const EXIT_AREA_LAST: ProtoId = ProtoId(0x5000017);
    pub const RADIOACTIVE_GOO_FIRST: ProtoId = ProtoId(0x20003D9);
    pub const RADIOACTIVE_GOO_LAST: ProtoId = ProtoId(0x20003DC);

    pub fn new(kind: EntityKind, id: Option<u32>) -> Self {
        let bits = if let Some(id) = id {
            assert!(id <= 0xffffff);
            id + 1
        } else {
            0
        };
        ProtoId((kind as u32) << 24 | bits)
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        EntityKind::from_u32(v >> 24)?;
        Some(ProtoId(v))
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
    /// A special `None` ID is possible.
    /// Note that the result it's zero based, so PID 0x01000001 has ID of 0, and
    /// PID 0x01000000 has ID of `None`.
    pub fn id(self) -> Option<u32> {
        let r = self.0 & 0xffffff;
        if r == 0 {
            None
        } else {
            Some(r - 1)
        }
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
        write!(f, "Pid(0x{:08x})", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pid_id() {
        let pid = ProtoId::new(EntityKind::Critter, Some(0));
        assert_eq!(pid.id(), Some(0));
        assert_eq!(pid.pack(), 0x01_000001);

        let pid = ProtoId::new(EntityKind::Skilldex, Some(1));
        assert_eq!(pid.id(), Some(1));
        assert_eq!(pid.pack(), 0x0a_000002);

        let pid = ProtoId::new(EntityKind::Critter, None);
        assert_eq!(pid.id(), None);
        assert_eq!(pid.pack(), 0x01_000000);
    }
}
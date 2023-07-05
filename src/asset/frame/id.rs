mod consts;

use byteorder::{BigEndian, ReadBytesExt};
use log::*;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::asset::{EntityKind, CritterAnim, WeaponKind};
use crate::graphics::geometry::hex::Direction;

pub type Idx = u16;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum FrameId {
    Critter(Critter),
    Head(Head),
    Generic(Generic),
}

impl FrameId {
    pub fn new(kind: EntityKind, direction: Option<Direction>, anim: u8, sub_anim: u8, idx: Idx)
            -> Option<Self> {
        Self::from_packed(Parts {
            kind,
            direction,
            anim,
            sub_anim,
            idx,
        }.pack()?)
    }

    pub fn new_critter(direction: Option<Direction>, anim: CritterAnim, weapon: WeaponKind, idx: Idx)
            -> Option<Self> {
        Critter::new(direction, anim, weapon, idx).map(FrameId::Critter)
    }

    pub fn new_head(anim: u8, sub_anim: u8, idx: Idx) -> Option<Self> {
        Head::new(anim, sub_anim, idx).map(FrameId::Head)
    }

    pub fn new_generic(kind: EntityKind, idx: Idx) -> Option<Self> {
        Generic::new(kind, idx).map(FrameId::Generic)
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let v = rd.read_u32::<BigEndian>()?;
        Self::from_packed(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("malformed FID: {:x}", v)))
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

    pub fn from_packed(fid: u32) -> Option<Self> {
        let kind = Parts::unpack(fid)?.kind;

        match kind {
            EntityKind::Critter => Critter::from_packed(fid).map(|v| v.into()),
            EntityKind::Head => Head::from_packed(fid).map(|v| v.into()),
            _ => Generic::from_packed(fid).map(|v| v.into()),
        }
    }

    pub fn packed(self) -> u32 {
        match self {
            FrameId::Critter(v) => v.packed(),
            FrameId::Head(v) => v.packed(),
            FrameId::Generic(v) => v.packed(),
        }
    }

    pub fn critter(self) -> Option<Critter> {
        if let FrameId::Critter(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn head(self) -> Option<Head> {
        if let FrameId::Head(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn generic(self) -> Option<Generic> {
        if let FrameId::Generic(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn kind(self) -> EntityKind {
        match self {
            FrameId::Critter(_) => EntityKind::Critter,
            FrameId::Head(_) => EntityKind::Head,
            FrameId::Generic(v) => v.kind(),
        }
    }

    pub fn direction(self) -> Option<Direction> {
        match self {
            FrameId::Critter(v) => v.direction(),
            FrameId::Head(v) => v.direction(),
            FrameId::Generic(_) => None,
        }
    }

    pub fn with_direction(self, direction: Option<Direction>) -> Option<Self> {
        match self {
            FrameId::Critter(v) => Some(FrameId::Critter(v.with_direction(direction))),
            FrameId::Head(v) => Some(FrameId::Head(v.with_direction(direction))),
            FrameId::Generic(_) => None,
        }
    }

    pub fn anim(self) -> u8 {
        match self {
            FrameId::Critter(v) => v.anim() as u8,
            FrameId::Head(v) => v.anim(),
            FrameId::Generic(_) => 0,
        }
    }

    pub fn sub_anim(self) -> u8 {
        match self {
            FrameId::Critter(v) => v.weapon() as u8,
            FrameId::Head(v) => v.sub_anim(),
            FrameId::Generic(_) => 0,
        }
    }

    pub fn idx(self) -> Idx {
        match self {
            FrameId::Critter(v) => v.idx(),
            FrameId::Head(v) => v.idx(),
            FrameId::Generic(v) => v.idx(),
        }
    }
}

impl fmt::Debug for FrameId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameId(0x{:08x})", self.packed())
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Critter(u32);

impl Critter {
    pub fn new(direction: Option<Direction>, anim: CritterAnim, weapon: WeaponKind, idx: Idx)
            -> Option<Self> {
        Parts {
            kind: EntityKind::Critter,
            direction,
            anim: anim as u8,
            sub_anim: weapon as u8,
            idx
        }.pack().map(Critter)
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let Parts {
            kind,
            anim,
            sub_anim,
            ..
        } = Parts::unpack(fid)?;

        if kind != EntityKind::Critter {
            return None;
        }
        CritterAnim::from_u8(anim)?;
        WeaponKind::from_u8(sub_anim)?;

        Some(Critter(fid))
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn direction(self) -> Option<Direction> {
        Parts::unpack(self.0).unwrap().direction
    }

    pub fn anim(self) -> CritterAnim {
        let anim = Parts::unpack(self.0).unwrap().anim;
        CritterAnim::from_u8(anim).unwrap()
    }

    pub fn weapon(self) -> WeaponKind {
        let sub_anim = Parts::unpack(self.0).unwrap().sub_anim;
        WeaponKind::from_u8(sub_anim).unwrap()
    }

    pub fn idx(self) -> Idx {
        Parts::unpack(self.0).unwrap().idx
    }

    pub fn with_direction(self, direction: Option<Direction>) -> Self {
        let mut parts = Parts::unpack(self.packed()).unwrap();
        parts.direction = direction;
        Critter(parts.pack().unwrap())
    }

    pub fn with_anim(self, anim: CritterAnim) -> Self {
        let mut parts = Parts::unpack(self.packed()).unwrap();
        parts.anim = anim as u8;
        Critter(parts.pack().unwrap())
    }

    pub fn with_weapon(self, weapon: WeaponKind) -> Self {
        let mut parts = Parts::unpack(self.packed()).unwrap();
        parts.sub_anim = weapon as u8;
        Critter(parts.pack().unwrap())
    }

    pub fn with_id(self, idx: Idx) -> Option<Self> {
        let mut parts = Parts::unpack(self.packed()).unwrap();
        parts.idx = idx;
        parts.pack().map(Critter)
    }
}

impl fmt::Debug for Critter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameId(0x{:08x})", self.packed())
    }
}

impl From<Critter> for FrameId {
    fn from(val: Critter) -> Self {
        Self::Critter(val)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Head(u32);

impl Head {
    pub fn new(anim: u8, sub_anim: u8, idx: Idx) -> Option<Self> {
        assert!(anim <= 12);
        assert!(sub_anim <= 9);
        Parts {
            kind: EntityKind::Head,
            direction: None,
            anim,
            sub_anim,
            idx,
        }.pack().map(Head)
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let Parts {
            kind,
            anim,
            sub_anim,
            ..
        } = Parts::unpack(fid)?;

        if kind != EntityKind::Head {
            return None;
        }
        if anim > 12 {
            return None;
        }
        if sub_anim > 9 {
            return None;
        }

        Some(Head(fid))
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn direction(self) -> Option<Direction> {
        let Parts { direction, .. } = Parts::unpack(self.0).unwrap();
        direction
    }

    pub fn with_direction(self, direction: Option<Direction>) -> Self {
        let mut parts = Parts::unpack(self.packed()).unwrap();
        parts.direction = direction;
        Head(parts.pack().unwrap())
    }

    pub fn anim(self) -> u8 {
        let Parts { anim, .. } = Parts::unpack(self.0).unwrap();
        anim
    }

    pub fn sub_anim(self) -> u8 {
        let Parts { sub_anim, .. } = Parts::unpack(self.0).unwrap();
        sub_anim
    }

    pub fn idx(self) -> Idx {
        Parts::unpack(self.0).unwrap().idx
    }
}

impl fmt::Debug for Head {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameId(0x{:08x})", self.packed())
    }
}

impl From<Head> for FrameId {
    fn from(val: Head) -> Self {
        Self::Head(val)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Generic(u32);

impl Generic {
    pub fn new(kind: EntityKind, idx: Idx) -> Option<Self> {
        assert!(kind != EntityKind::Critter && kind != EntityKind::Head);
        Parts {
            kind,
            direction: None,
            anim: 0,
            sub_anim: 0,
            idx,
        }.pack().map(Generic)
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let Parts {
            kind,
            direction,
            anim,
            sub_anim,
            idx,
        } = Parts::unpack(fid)?;

        if kind == EntityKind::Critter || kind == EntityKind::Head {
            return None;
        }
        if let Some(direction) = direction {
            warn!("ignoring direction component ({:?}) of a {:?} FID 0x{:08x}", direction, kind, fid);
        }
        if anim != 0 {
            warn!("ignoring animation ID 0x{:x} of a {:?} FID 0x{:08x}", anim, kind, fid);
        }
        if sub_anim != 0 {
            warn!("ignoring sub-animation ID 0x{:x} of a {:?} FID 0x{:08x}", sub_anim, kind, fid);
        }

        Self::new(kind, idx)
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn kind(self) -> EntityKind {
        let Parts { kind, .. } = Parts::unpack(self.0).unwrap();
        kind
    }

    pub fn idx(self) -> Idx {
        Parts::unpack(self.0).unwrap().idx
    }
}

impl fmt::Debug for Generic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameId(0x{:08x})", self.packed())
    }
}

impl From<Generic> for FrameId {
    fn from(val: Generic) -> Self {
        Self::Generic(val)
    }
}

struct Parts {
    kind: EntityKind,
    direction: Option<Direction>,
    anim: u8,
    sub_anim: u8,
    idx: Idx,
}

impl Parts {
    fn pack(self) -> Option<u32> {
        if self.sub_anim > 15 || self.idx > 4095 {
            return None;
        }
        Some(self.direction.map(|d| d as u32 + 1).unwrap_or(0) << 28
            | (self.kind as u32) << 24
            | (self.anim as u32) << 16
            | (self.sub_anim as u32) << 12
            | (self.idx as u32))
    }

    fn unpack(fid: u32) -> Option<Self> {
        let kind = EntityKind::from_u32((fid >> 24) & 0b1111)?;

        let direction = (fid >> 28) as u8 & 0b111;
        let direction = if direction == 0 {
            None
        } else {
            Some(Direction::from_u8(direction - 1)?)
        };

        let anim = (fid >> 16) as u8;
        let sub_anim = (fid >> 12) as u8 & 0b1111;
        let idx = fid as Idx & 0b1111_1111_1111;

        Some(Self {
            kind,
            direction,
            anim,
            sub_anim,
            idx,
        })
    }
}



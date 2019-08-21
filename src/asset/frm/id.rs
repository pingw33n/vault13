mod consts;

use byteorder::{BigEndian, ReadBytesExt};
use log::*;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::asset::{EntityKind, CritterAnim, WeaponKind};
use crate::graphics::geometry::hex::Direction;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum Fid {
    Critter(CritterFid),
    Head(HeadFid),
    Generic(GenericFid),
}

impl Fid {
    pub fn new(kind: EntityKind, direction: Option<Direction>, anim: u8, sub_anim: u8, id: u16)
            -> Option<Self> {
        Self::from_packed(pack(FidParts {
            kind,
            direction,
            anim,
            sub_anim,
            id,
        })?)
    }

    pub fn new_critter(direction: Option<Direction>, anim: CritterAnim, weapon: WeaponKind, id: u16)
            -> Option<Self> {
        CritterFid::new(direction, anim, weapon, id).map(|v| Fid::Critter(v))
    }

    pub fn new_head(anim: u8, sub_anim: u8, id: u16) -> Option<Self> {
        HeadFid::new(anim, sub_anim, id).map(|v| Fid::Head(v))
    }

    pub fn new_generic(kind: EntityKind, id: u16) -> Option<Self> {
        GenericFid::new(kind, id).map(|v| Fid::Generic(v))
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
        let FidParts { kind, .. } = unpack(fid)?;

        match kind {
            EntityKind::Critter => CritterFid::from_packed(fid).map(|v| v.into()),
            EntityKind::Head => HeadFid::from_packed(fid).map(|v| v.into()),
            _ => GenericFid::from_packed(fid).map(|v| v.into()),
        }
    }

    pub fn packed(self) -> u32 {
        match self {
            Fid::Critter(v) => v.packed(),
            Fid::Head(v) => v.packed(),
            Fid::Generic(v) => v.packed(),
        }
    }

    pub fn critter(self) -> Option<CritterFid> {
        if let Fid::Critter(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn head(self) -> Option<HeadFid> {
        if let Fid::Head(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn generic(self) -> Option<GenericFid> {
        if let Fid::Generic(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn kind(self) -> EntityKind {
        match self {
            Fid::Critter(_) => EntityKind::Critter,
            Fid::Head(_) => EntityKind::Head,
            Fid::Generic(v) => v.kind(),
        }
    }

    pub fn direction(self) -> Option<Direction> {
        match self {
            Fid::Critter(v) => v.direction(),
            Fid::Head(v) => v.direction(),
            Fid::Generic(_) => None,
        }
    }

    pub fn anim(self) -> u8 {
        match self {
            Fid::Critter(v) => v.anim() as u8,
            Fid::Head(v) => v.anim(),
            Fid::Generic(_) => 0,
        }
    }

    pub fn sub_anim(self) -> u8 {
        match self {
            Fid::Critter(v) => v.weapon() as u8,
            Fid::Head(v) => v.sub_anim(),
            Fid::Generic(_) => 0,
        }
    }

    pub fn id(self) -> u16 {
        match self {
            Fid::Critter(v) => v.id(),
            Fid::Head(v) => v.id(),
            Fid::Generic(v) => v.id(),
        }
    }
}

impl fmt::Debug for Fid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.packed())
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct CritterFid(u32);

impl CritterFid {
    pub fn new(direction: Option<Direction>, anim: CritterAnim, weapon: WeaponKind, id: u16)
            -> Option<Self> {
        pack(FidParts {
            kind: EntityKind::Critter,
            direction,
            anim: anim as u8,
            sub_anim: weapon as u8,
            id
        }).map(|v| CritterFid(v))
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let FidParts {
            kind,
            anim,
            sub_anim,
            ..
        } = unpack(fid)?;

        if kind != EntityKind::Critter {
            return None;
        }
        CritterAnim::from_u8(anim)?;
        WeaponKind::from_u8(sub_anim)?;

        Some(CritterFid(fid))
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn direction(self) -> Option<Direction> {
        let FidParts { direction, .. } = unpack(self.0).unwrap();
        direction
    }

    pub fn anim(self) -> CritterAnim {
        let FidParts { anim, .. } = unpack(self.0).unwrap();
        CritterAnim::from_u8(anim).unwrap()
    }

    pub fn weapon(self) -> WeaponKind {
        let FidParts { sub_anim, .. } = unpack(self.0).unwrap();
        WeaponKind::from_u8(sub_anim).unwrap()
    }

    pub fn id(self) -> u16 {
        let FidParts { id, .. } = unpack(self.0).unwrap();
        id
    }

    pub fn with_direction(self, direction: Option<Direction>) -> Self {
        let mut parts = unpack(self.packed()).unwrap();
        parts.direction = direction;
        CritterFid(pack(parts).unwrap())
    }

    pub fn with_anim(self, anim: CritterAnim) -> Self {
        let mut parts = unpack(self.packed()).unwrap();
        parts.anim = anim as u8;
        CritterFid(pack(parts).unwrap())
    }

    pub fn with_weapon(self, weapon: WeaponKind) -> Self {
        let mut parts = unpack(self.packed()).unwrap();
        parts.sub_anim = weapon as u8;
        CritterFid(pack(parts).unwrap())
    }

    pub fn with_id(self, id: u16) -> Option<Self> {
        let mut parts = unpack(self.packed()).unwrap();
        parts.id = id;
        pack(parts).map(|v| CritterFid(v))
    }
}

impl fmt::Debug for CritterFid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.packed())
    }
}

impl Into<Fid> for CritterFid {
    fn into(self) -> Fid {
        Fid::Critter(self)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct HeadFid(u32);

impl HeadFid {
    pub fn new(anim: u8, sub_anim: u8, id: u16) -> Option<Self> {
        assert!(anim <= 12);
        assert!(sub_anim <= 9);
        pack(FidParts {
            kind: EntityKind::Head,
            direction: None,
            anim,
            sub_anim,
            id,
        }).map(|v| HeadFid(v))
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let FidParts {
            kind,
            anim,
            sub_anim,
            ..
        } = unpack(fid)?;

        if kind != EntityKind::Head {
            return None;
        }
        if anim > 12 {
            return None;
        }
        if sub_anim > 9 {
            return None;
        }

        Some(HeadFid(fid))
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn direction(self) -> Option<Direction> {
        let FidParts { direction, .. } = unpack(self.0).unwrap();
        direction
    }

    pub fn anim(self) -> u8 {
        let FidParts { anim, .. } = unpack(self.0).unwrap();
        anim
    }

    pub fn sub_anim(self) -> u8 {
        let FidParts { sub_anim, .. } = unpack(self.0).unwrap();
        sub_anim
    }

    pub fn id(self) -> u16 {
        let FidParts { id, .. } = unpack(self.0).unwrap();
        id
    }
}

impl fmt::Debug for HeadFid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.packed())
    }
}

impl Into<Fid> for HeadFid {
    fn into(self) -> Fid {
        Fid::Head(self)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct GenericFid(u32);

impl GenericFid {
    pub fn new(kind: EntityKind, id: u16) -> Option<Self> {
        assert!(kind != EntityKind::Critter && kind != EntityKind::Head);
        pack(FidParts {
            kind,
            direction: None,
            anim: 0,
            sub_anim: 0,
            id,
        }).map(|v| GenericFid(v))
    }

    pub fn from_packed(fid: u32) -> Option<Self> {
        let FidParts {
            kind,
            direction,
            anim,
            sub_anim,
            id,
        } = unpack(fid)?;

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

        Self::new(kind, id)
    }

    pub fn packed(self) -> u32 {
        self.0
    }

    pub fn kind(self) -> EntityKind {
        let FidParts { kind, .. } = unpack(self.0).unwrap();
        kind
    }

    pub fn id(self) -> u16 {
        let FidParts { id, .. } = unpack(self.0).unwrap();
        id
    }
}

impl fmt::Debug for GenericFid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.packed())
    }
}

impl Into<Fid> for GenericFid {
    fn into(self) -> Fid {
        Fid::Generic(self)
    }
}

struct FidParts {
    kind: EntityKind,
    direction: Option<Direction>,
    anim: u8,
    sub_anim: u8,
    id: u16,
}

fn pack(parts: FidParts) -> Option<u32> {
    if parts.sub_anim > 15 {
        return None;
    }
    if parts.id > 4095 {
        return None;
    }
    Some(parts.direction.map(|d| d as u32 + 1).unwrap_or(0) << 28
        | (parts.kind as u32) << 24
        | (parts.anim as u32) << 16
        | (parts.sub_anim as u32) << 12
        | (parts.id as u32))
}

fn unpack(fid: u32) -> Option<FidParts> {
    let kind = EntityKind::from_u32((fid >> 24) & 0b1111)?;

    let direction = (fid >> 28) as u8 & 0b111;
    let direction = if direction == 0 {
        None
    } else {
        Some(Direction::from_u8(direction - 1)?)
    };

    let anim = (fid >> 16) as u8 & 0xff;
    let sub_anim = (fid >> 12) as u8 & 0b1111;
    let id = fid as u16 & 0b1111_1111_1111;

    Some(FidParts {
        kind,
        direction,
        anim,
        sub_anim,
        id,
    })
}


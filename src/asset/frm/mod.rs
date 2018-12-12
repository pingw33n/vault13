mod db;

pub use self::db::FrmDb;

use byteorder::{BigEndian, ReadBytesExt};
use enum_map::EnumMap;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use asset::{EntityKind, WeaponKind};
use graphics::frm::{Frame, FrameList, FrameSet};
use graphics::geometry::Direction;
use graphics::render::TextureFactory;
use graphics::Point;
use util::EnumExt;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum Fid {
    Critter(CritterFid),
    Head(HeadFid),
    Generic(GenericFid),
}

impl Fid {
    pub const MOUSE_HEX: Fid = Fid::Generic(GenericFid(0x6000_001));
    pub const MOUSE_HEX2: Fid = Fid::Generic(GenericFid(0x6000_0f9));
    pub const EGG: Fid = Fid::Generic(GenericFid(0x6000_002));
    pub const MAIN_HUD: Fid = Fid::Generic(GenericFid(0x6000_010));

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

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum CritterAnim {
    // basic animations  0-19
    Stand                   =  0, // AA, [D-M]A
    Walk                    =  1, // AB, [D-M]B
    JumpBegin               =  2, // AC?
    JumpEnd                 =  3, // AD?
    ClimbLadder             =  4, // AE
    Falling                 =  5, // AF?
    UpStairsRight           =  6, // AG
    UpStairsLeft            =  7, // AH
    DownStairsRight         =  8, // AI
    DownStairsLeft          =  9, // AJ
    MagicHandsGround        = 10, // AK
    MagicHandsMiddle        = 11, // AL
    MagicHandsUp            = 12, // AM?
    DodgeAnim               = 13, // AN
    HitFromFront            = 14, // AO
    HitFromBack             = 15, // AP
    ThrowPunch              = 16, // AQ
    KickLeg                 = 17, // AR
    ThrowAnim               = 18, // AS, DM, GM
    Running                 = 19, // AT
                                  // AU?

    // knockdown and death   20-35
    FallBack                = 20, // BA
    FallFront               = 21, // BB
    BadLanding              = 22, // BC
    BigHole                 = 23, // BD
    CharredBody             = 24, // BE
    ChunksOfFlesh           = 25, // BF
    DancingAutofire         = 26, // BG
    Electrify               = 27, // BH
    SlicedInHalf            = 28, // BI
    BurnedToNothing         = 29, // BJ
    ElectrifiedToNothing    = 30, // BK
    ExplodedToNothing       = 31, // BL
    MeltedToNothing         = 32, // BM
    FireDance               = 33, // BN
    FallBackBlood           = 34, // BO
    FallFrontBlood          = 35, // BP

    // change positions  36-37
    ProneToStanding         = 36, // CH
    BackToStanding          = 37, // CJ

    // weapon 38-47
    TakeOut                 = 38, // [D-M]C
    PutAway                 = 39, // [D-M]D
    ParryAnim               = 40, // [D-M]E
    ThrustAnim              = 41, // [D-G]F
    SwingAnim               = 42, // [D-F]G
    Point                   = 43, // [H-M]H
    Unpoint                 = 44, // [H-M]I
    FireSingle              = 45, // [H-M]J
    FireBurst               = 46, // [H-M]K
    FireContinuous          = 47, // [H-M]L

    // single-frame death animations = the last frame of knockdown and death animations)   48-63
    FallBackSf              = 48, // RA
    FallFrontSf             = 49, // RB
    BadLandingSf            = 50, // RC
    BigHoleSf               = 51, // RD
    CharredBodySf           = 52, // RE
    ChunksOfFleshSf         = 53, // RF
    DancingAutofireSf       = 54, // RG
    ElectrifySf             = 55, // RH
    SlicedInHalfSf          = 56, // RI
    BurnedToNothingSf       = 57, // RJ
    ElectrifiedToNothingSf  = 58, // RK
    ExplodedToNothingSf     = 59, // RL
    MeltedToNothingSf       = 60, // RM
    FallBackBloodSf         = 61, // RO
    FallFrontBloodSf        = 62, // RP

    Unknown63               = 63, // Fid(0x013f003f), Pid(0x01000004) seen on broken2.map at pos 18648.

    // called shot interface picture
    CalledShotPic           = 64, // NA
}

impl CritterAnim {
    pub fn code(self, base: Self, char_base: u8) -> char {
        let c = char_base + self as u8 - base as u8;
        assert!(c.is_ascii());
        c as char
    }
}

pub fn read_frm(rd: &mut impl Read, texture_factory: &TextureFactory) -> io::Result<FrameSet> {
    let _version = rd.read_u32::<BigEndian>()?;

    let fps = rd.read_u16::<BigEndian>()?;
    let fps = if fps == 0 {
        10
    } else {
        fps
    };

    let action_frame = rd.read_u16::<BigEndian>()?;
    let frames_per_direction = rd.read_u16::<BigEndian>()? as usize;
    assert!(frames_per_direction > 0);

    let mut centers_x = EnumMap::new();
    for dir in Direction::iter() {
        centers_x[dir] = rd.read_i16::<BigEndian>()? as i32;
    }
    let mut centers_y = EnumMap::new();
    for dir in Direction::iter() {
        centers_y[dir] = rd.read_i16::<BigEndian>()? as i32;
    }

    let mut frame_offsets = EnumMap::new();
    for dir in Direction::iter() {
        frame_offsets[dir] = rd.read_u32::<BigEndian>()?;
    }

    let _data_len = rd.read_u32::<BigEndian>()?;

    let mut loaded_offsets: EnumMap<Direction, Option<u32>> = EnumMap::new();
    let mut frame_lists: EnumMap<Direction, Option<FrameList>> = EnumMap::new();
    for dir in Direction::iter() {
        let offset = frame_offsets[dir];
        let already_loaded_dir = loaded_offsets
            .iter()
            .filter_map(|(d, o)| o.filter(|&o| o == offset).map(|_| d))
            .next();
        if let Some(already_loaded_dir) = already_loaded_dir {
            frame_lists[dir] = frame_lists[already_loaded_dir].clone();
            continue;
        }

        loaded_offsets[dir] = Some(offset);

        let mut frames = Vec::with_capacity(frames_per_direction);
        for _ in 0..frames_per_direction {
            let width = rd.read_i16::<BigEndian>()? as i32;
            let height = rd.read_i16::<BigEndian>()? as i32;
            let _len = rd.read_u32::<BigEndian>()?;
            let shift = Point::new(
                rd.read_i16::<BigEndian>()? as i32,
                rd.read_i16::<BigEndian>()? as i32,
            );

            let len = (width * height) as usize;
            let mut pixels = vec![0; len].into_boxed_slice();
            rd.read_exact(&mut pixels)?;

            let texture = texture_factory.new_texture(width, height, pixels);

            frames.push(Frame {
                shift,
                width,
                height,
                texture,
            });
        }
        frame_lists[dir] = Some(FrameList {
            center: Point::new(centers_x[dir], centers_y[dir]),
            frames,
        });
    }

    Ok(FrameSet {
        fps,
        action_frame,
        frame_lists: EnumMap::from(|k| frame_lists[k].take().unwrap()),
    })
}
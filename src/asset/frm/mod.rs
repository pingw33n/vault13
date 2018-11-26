mod db;

pub use self::db::FrmDb;

use byteorder::{BigEndian, ReadBytesExt};
use enum_map::EnumMap;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};

use asset::EntityKind;
use fs::FileSystem;
use graphics::frm::{Frame, FrameList, FrameSet};
use graphics::geometry::Direction;
use graphics::render::{Render, TextureHandle};
use graphics::Point;
use util::EnumExt;

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Fid(u32);

impl Fid {
    pub const EGG: Fid = Fid(0x6000_002);

    pub fn new(kind: EntityKind, id3: u8, id2: u8, id1: u8, id0: u16) -> Option<Self> {
        assert!(id3 >> 3 == 0);
        assert!(id1 >> 4 == 0);
        assert!(id0 >> 12 == 0);
        let v =
            (id3 as u32) << 28 |
            (kind as u32) << 24 |
            (id2 as u32) << 16 |
            (id1 as u32) << 12 |
            (id0 as u32);
        Some(Fid(v))
    }

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
        self.0 as u16 & 0b1111_1111_1111
    }

    pub fn with_id0(self, id0: u16) -> Option<Self> {
        Self::new(self.kind(), self.id3(), self.id2(), self.id1(), id0)
    }
}

impl fmt::Debug for Fid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fid(0x{:08x})", self.0)
    }
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

pub fn read_frm(rd: &mut impl Read, render: &mut Render) -> io::Result<FrameSet> {
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

    let data_len = rd.read_u32::<BigEndian>()?;

    let mut loaded_offsets: EnumMap<Direction, Option<u32>> = EnumMap::new();
    let mut frame_lists: EnumMap<Direction, Option<FrameList>> = EnumMap::new();
    for dir in Direction::iter() {
        let offset = frame_offsets[dir];
        let already_loaded_dir = loaded_offsets
            .iter()
            .filter_map(|(d, o)| o.filter(|&o| o == offset).map(|o| d))
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

            let texture = render.new_texture(width, height, pixels);

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
mod db;

pub use self::db::ProtoDb;

use byteorder::{BigEndian, ReadBytesExt};
use enumflags::BitFlags;
use enum_map::EnumMap;
use num_traits::FromPrimitive;
use std::fmt;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::ops::RangeInclusive;

use super::*;
use asset::frm::Fid;
use asset::EntityKind;
use util::{enum_iter, EnumIter};

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
pub struct Pid(u32);

impl Pid {
    pub const SHIV: Pid = Pid(0x17F);
    pub const EXIT_AREA_FIRST: Pid = Pid(0x5000010);
    pub const EXIT_AREA_LAST: Pid = Pid(0x5000017);

    pub fn new(kind: EntityKind, idx: u32) -> Self {
        assert!(idx > 0 && idx <= 0xffffff);
        Pid((kind as u32) << 24 | idx)
    }

    pub fn from_packed(v: u32) -> Option<Self> {
        let obj_kind = EntityKind::from_u32(v >> 24)?;
        let pid = Pid(v);
        if pid.id() == 0 {
            None
        } else {
            Some(pid)
        }
    }

    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let v = rd.read_u32::<BigEndian>()?;
        Ok(Self::from_packed(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("malformed PID: {:x}", v))).unwrap())
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

    pub fn id(self) -> u32 {
        self.0 & 0xffffff
    }

    pub fn is_exit_area(self) -> bool {
        self >= Self::EXIT_AREA_FIRST && self <= Self::EXIT_AREA_LAST
    }
}

impl fmt::Debug for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Pid(0x{:08x})", self.0)
    }
}

#[derive(Debug)]
pub struct Proto {
    pub pid: Pid,
    pub message_id: i32,
    pub fid: Fid,
    pub light_radius: i32,
    pub light_intensity: i32,
    pub flags: BitFlags<Flag>,
    pub flags_ext: BitFlags<FlagExt>,
    pub script_id: Option<u32>,
    pub proto: Variant,
}

impl Proto {
    pub fn kind(&self) -> ExactEntityKind {
        self.proto.kind()
    }
}

#[derive(Debug)]
pub enum Variant {
    Item(Item),
    Critter(Critter),
    Scenery(Scenery),
    Wall(Wall),
    SqrTile(SqrTile),
    Misc,
}

impl Variant {
    pub fn kind(&self) -> ExactEntityKind {
        use self::Variant::*;
        match self {
            Item(ref v) => ExactEntityKind::Item(v.item.kind()),
            Critter(_) => ExactEntityKind::Critter,
            Scenery(ref v) => ExactEntityKind::Scenery(v.scenery.kind()),
            Wall(_) => ExactEntityKind::Wall,
            SqrTile(_) => ExactEntityKind::SqrTile,
            Misc => ExactEntityKind::Misc,
        }
    }
}

#[derive(Debug)]
pub struct Item {
    pub material: Material,
    pub size: i32,
    pub weight: i32,
    pub price: i32,
    pub inventory_fid: Option<Fid>,
    pub sound_id: u8,
    pub item: ItemVariant,
}

#[derive(Debug)]
pub enum ItemVariant {
    Armor(Armor),
    Container(Container),
    Drug(Drug),
    Weapon(Weapon),
    Ammo(Ammo),
    Misc(MiscItem),
    Key(Key),
}

impl ItemVariant {
    pub fn kind(&self) -> ItemKind {
        use self::ItemVariant::*;
        match self {
            Armor(_)        => ItemKind::Armor,
            Container(_)    => ItemKind::Container,
            Drug(_)         => ItemKind::Drug,
            Weapon(_)       => ItemKind::Weapon,
            Ammo(_)         => ItemKind::Ammo,
            Misc(_)         => ItemKind::Misc,
            Key(_)          => ItemKind::Key,
        }
    }
}

#[derive(Debug)]
pub struct Armor {
  pub armor_class: i32,
  pub damage_resistance: EnumMap<DamageKind, i32>,
  pub damage_threshold: EnumMap<DamageKind, i32>,
  pub perk: Option<Perk>,
  pub male_fid: Fid,
  pub female_fid: Fid,
}

#[derive(Debug)]
pub struct Container {
    pub capacity: i32,
    pub flags: BitFlags<ContainerFlag>,
}

#[derive(Clone, Copy, Debug, EnumFlags, Eq, PartialEq)]
#[repr(u32)]
pub enum ContainerFlag {
    CannotPickUp = 1,
    MagicHandsGround = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrugEffectModifier {
    Fixed(i32),
    Random(i32, i32),
}

#[derive(Debug)]
pub struct DrugEffect {
    pub delay: u32,
    pub stat: Stat,
    pub modifier: DrugEffectModifier,
}

#[derive(Debug)]
pub struct DrugAddiction {
    pub chance: u32,
    pub perk: Option<Perk>,
    pub delay: u32,
}

#[derive(Debug)]
pub struct Drug {
    pub effects: Vec<DrugEffect>,
    pub addiction: DrugAddiction,
}

#[derive(Clone, Debug)]
pub struct Dual<T> {
    pub primary: T,
    pub secondary: T,
}

#[derive(Debug)]
pub struct Weapon {
    pub attack_kind: Dual<AttackKind>,
    pub animation_code: WeaponKind,
    pub damage: RangeInclusive<i32>,
    pub damage_kind: DamageKind,
    pub max_range: Dual<i32>,
    pub projectile_pid: Option<Pid>,
    pub min_strength: i32,
    pub ap_cost: Dual<i32>,
    pub crit_failure_table: i32,
    pub perk: Option<Perk>,
    // Number of bullets per burst shot.
    pub burst_bullet_count: i32,
    // proto.msg:300
    pub caliber: i32,
    pub ammo_pid: Option<Pid>,
    pub max_ammo: i32,
    pub sound_id: u8,
}

#[derive(Debug)]
pub struct Ammo {
    pub caliber: i32,
    pub magazine_size: i32,
    pub ac_modifier: i32,
    pub dr_modifier: i32,
    pub damage_mult: i32,
    pub damage_div: i32,
}

#[derive(Debug)]
pub struct MiscItem {
    pub charge_pid: Option<Pid>,
    pub charge_kind: u32,
    pub max_charge_count: i32,
}

#[derive(Debug)]
pub struct Key {
    pub id: i32,
}

#[derive(Debug)]
pub struct Critter {
    pub flags: BitFlags<CritterFlag>,
    pub base_stats: EnumMap<Stat, i32>,
    pub bonus_stats: EnumMap<Stat, i32>,
    pub skills: EnumMap<Skill, i32>,
    //proto.msg:400
    //0x0 - biped (двуногие)
    //0x1 - quadruped (четвероногие)
    //0x2 - robotic (роботы)
    pub body_kind: u32,
    pub experience: i32,
    //proto.msg:1450
    pub kill_kind: u32,
    pub damage_kind: DamageKind,
    pub head_fid: Option<Fid>,
    pub ai_packet: u32,
    pub team_id: u32,
}

#[derive(Clone, Copy, Debug, EnumFlags, Eq, PartialEq)]
#[repr(u32)]
pub enum CritterFlag {
    NoBarter        = 0x00000002, // Can barter with.
    NoSteal         = 0x00000020, // Can't steal from.
    NoDrop          = 0x00000040, // Doesn't drop items.
    NoLoseLimbs     = 0x00000080, // Can't shoot off limbs.
    Ages            = 0x00000100, // Dead body doesn't disappear.
    NoHeal          = 0x00000200, // HP doesn't restore over time.
    Invulnerable    = 0x00000400,
    NoDeadBody      = 0x00000800, // Dead body disappears immediately.
    SpecialDeath    = 0x00001000, // Has special death animation.
    RangedMelee     = 0x00002000, // Melee attack is possible at a distance.
    NoKnock         = 0x00004000, // Can't knock down.
}

#[derive(Debug)]
pub struct Scenery {
    pub material: Material,
    pub sound_id: u8,
    pub scenery: SceneryVariant,
}

#[derive(Debug)]
pub enum SceneryVariant {
    Door(Door),
    Stairs(Stairs),
    Elevator(Elevator),
    Ladder(Ladder),
    Misc,
}

impl SceneryVariant {
    pub fn kind(&self) -> SceneryKind {
        use self::SceneryVariant::*;
        match self {
            Door(_) => SceneryKind::Door,
            Stairs(_) => SceneryKind::Stairs,
            Elevator(_) => SceneryKind::Elevator,
            Ladder(ref l) => match l.kind {
                LadderKind::Up => SceneryKind::LadderUp,
                LadderKind::Down => SceneryKind::LadderDown,
            }
            Misc => SceneryKind::Misc,
        }
    }
}

#[derive(Debug)]
pub struct Door {
    pub flags: u32,
    pub key_id: u32,
}

#[derive(Debug)]
pub struct Stairs {
    pub elevation_and_tile: u32,
    pub map_id: u32,
}

#[derive(Debug)]
pub struct Elevator {
    pub kind: u32,
    pub level: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LadderKind {
    Up,
    Down,
}

#[derive(Debug)]
pub struct Ladder {
    pub kind: LadderKind,
    pub elevation_and_tile: u32,
}

#[derive(Debug)]
pub struct Wall {
    pub material: Material,
}

#[derive(Debug)]
pub struct SqrTile {
    pub material: Material,
}

// Subset that has prototypes.
pub fn proto_entity_kinds() -> EnumIter<EntityKind> {
    enum_iter(..=EntityKind::Misc)
}
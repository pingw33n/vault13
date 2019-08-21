pub mod font;
pub mod frame;
pub mod map;
pub mod message;
pub mod palette;
pub mod proto;
pub mod script;

use enumflags_derive::EnumFlags;
use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use log::*;
use std::io::{self, prelude::*};

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum EntityKind {
    Item = 0x0,
    Critter = 0x1,
    Scenery = 0x2,
    Wall = 0x3,
    SqrTile = 0x4,
    Misc = 0x5,
    Interface = 0x6,
    Inventory = 0x7,
    Head = 0x8,
    Background = 0x9,
    Skilldex = 0xa,
}

impl EntityKind {
    pub fn dir(self) -> &'static str {
        static DIRS: &'static [&'static str] = &[
            "items",
            "critters",
            "scenery",
            "walls",
            "tiles",
            "misc",
            "intrface",
            "inven",
            "heads",
            "backgrnd",
            "skilldex",
        ];
        DIRS[self as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactEntityKind {
    Item(ItemKind),
    Critter,
    Scenery(SceneryKind),
    Wall,
    SqrTile,
    Misc,
    Interface,
    Inventory,
    Head,
    Background,
    Skilldex,
}

impl ExactEntityKind {
    pub fn item(self) -> Option<ItemKind> {
        if let ExactEntityKind::Item(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn scenery(self) -> Option<SceneryKind> {
        if let ExactEntityKind::Scenery(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Enum, PartialEq, Ord, PartialOrd, Primitive)]
pub enum SceneryKind {
    Door = 0x0,
    Stairs = 0x1,
    Elevator = 0x2,
    LadderDown = 0x3,
    LadderUp = 0x4,
    Misc = 0x5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum ItemKind {
    Armor = 0x0,
    Container = 0x1,
    Drug = 0x2,
    Weapon = 0x3,
    Ammo = 0x4,
    Misc = 0x5,
    Key = 0x6,
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

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum Material {
    Glass       = 0,
    Metal       = 1,
    Plastic     = 2,
    Wood        = 3,
    Dirt        = 4,
    Stone       = 5,
    Cement      = 6,
    Leather     = 7,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum DamageKind {
    Melee       = 0,
    Laser       = 1,
    Fire        = 2,
    Plasma      = 3,
    Electric    = 4,
    Emp         = 5,
    Explosion   = 6,
    Radiation   = 100000,
    Poison      = 100001,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum Stat {
    Strength = 0x0,
    Perception = 0x1,
    Endurance = 0x2,
    Charisma = 0x3,
    Intelligence = 0x4,
    Agility = 0x5,
    Luck = 0x6,

    HitPoints = 0x7,
    ActionPoints = 0x8,
    ArmorClass = 0x9,

    UnarmedDamage = 0xA,
    MeleeDmg = 0xB,
    CarryWeight = 0xC,
    Sequence = 0xD,
    HealRate = 0xE,
    CritChance = 0xF,
    BetterCrit = 0x10,
    DmgThresh = 0x11,
    DmgThreshLaser = 0x12,
    DmgThreshFire = 0x13,
    DmgThreshPlasma = 0x14,
    DmgThreshElectrical = 0x15,
    DmgThreshEmp = 0x16,
    DmgThreshExplosion = 0x17,
    DmgResist = 0x18,
    DmgResistLaser = 0x19,
    DmgResistFire = 0x1A,
    DmgResistPlasma = 0x1B,
    DmgResistElectrical = 0x1C,
    DmgResistEmp = 0x1D,
    DmgResistExplosion = 0x1E,
    RadResist = 0x1F,
    PoisonResist = 0x20,
    Age = 0x21,
    Gender = 0x22,

    CurrentHp = 0x23,
    CurrentPoison = 0x24,
    CurrentRad = 0x25,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum Perk {
    BonusAwareness = 0x0,
    BonusHthAttacks = 0x1,
    BonusHthDamage = 0x2,
    BonusMove = 0x3,
    BonusRangedDamage = 0x4,
    BonusRateOfFire = 0x5,
    EarlierSequence = 0x6,
    FasterHealing = 0x7,
    MoreCriticals = 0x8,
    NightVision = 0x9,
    Presence = 0xA,
    RadResistance = 0xB,
    Toughness = 0xC,
    StrongBack = 0xD,
    Sharpshooter = 0xE,
    SilentRunning = 0xF,
    Survivalist = 0x10,
    MasterTrader = 0x11,
    Educated = 0x12,
    Healer = 0x13,
    FortuneFinder = 0x14,
    BetterCriticals = 0x15,
    Empathy = 0x16,
    Slayer = 0x17,
    Sniper = 0x18,
    SilentDeath = 0x19,
    ActionBoy = 0x1A,
    MentalBlock = 0x1B,
    Lifegiver = 0x1C,
    Dodger = 0x1D,
    Snakeater = 0x1E,
    MrFixit = 0x1F,
    Medic = 0x20,
    MasterThief = 0x21,
    Speaker = 0x22,
    HeaveHo = 0x23,
    FriendlyFoe = 0x24,
    Pickpocket = 0x25,
    Ghost = 0x26,
    CultOfPersonality = 0x27,
    Scrounger = 0x28,
    Explorer = 0x29,
    FlowerChild = 0x2A,
    Pathfinder = 0x2B,
    AnimalFriend = 0x2C,
    Scout = 0x2D,
    MysteriousStranger = 0x2E,
    Ranger = 0x2F,
    QuickPockets = 0x30,
    SmoothTalker = 0x31,
    SwiftLearner = 0x32,
    Tag = 0x33,
    Mutate = 0x34,
    AddNuka = 0x35,
    AddBuffout = 0x36,
    AddMentats = 0x37,
    AddPsycho = 0x38,
    AddRadaway = 0x39,
    WeaponLongRange = 0x3A,
    WeaponAccurate = 0x3B,
    WeaponPenetrate = 0x3C,
    WeaponKnockback = 0x3D,
    ArmorPowered = 0x3E,
    ArmorCombat = 0x3F,
    WeaponScopeRange = 0x40,
    WeaponFastReload = 0x41,
    WeaponNightSight = 0x42,
    WeaponFlameboy = 0x43,
    ArmorAdvanced1 = 0x44,
    ArmorAdvanced2 = 0x45,
    AddJet = 0x46,
    AddTragic = 0x47,
    ArmorCharisma = 0x48,
    GeckoSkinningPerk = 0x49,
    DermalArmorPerk = 0x4A,
    DermalEnhancementPerk = 0x4B,
    PhoenixArmorPerk = 0x4C,
    PhoenixEnhancementPerk = 0x4D,
    VaultCityInoculationsPerk = 0x4E,
    AdrenalineRushPerk = 0x4F,
    CautiousNaturePerk = 0x50,
    ComprehensionPerk = 0x51,
    DemolitionExpertPerk = 0x52,
    GamblerPerk = 0x53,
    GainStrengthPerk = 0x54,
    GainPerceptionPerk = 0x55,
    GainEndurancePerk = 0x56,
    GainCharismaPerk = 0x57,
    GainIntelligencePerk = 0x58,
    GainAgilityPerk = 0x59,
    GainLuckPerk = 0x5A,
    HarmlessPerk = 0x5B,
    HereAndNowPerk = 0x5C,
    HthEvadePerk = 0x5D,
    KamaSutraPerk = 0x5E,
    KarmaBeaconPerk = 0x5F,
    LightStepPerk = 0x60,
    LivingAnatomyPerk = 0x61,
    MagneticPersonalityPerk = 0x62,
    NegotiatorPerk = 0x63,
    PackRatPerk = 0x64,
    PyromaniacPerk = 0x65,
    QuickRecoveryPerk = 0x66,
    SalesmanPerk = 0x67,
    StonewallPerk = 0x68,
    ThiefPerk = 0x69,
    WeaponHandlingPerk = 0x6A,
    VaultCityTrainingPerk = 0x6B,
    AlcoholHpBonus1Perk = 0x6C,
    AlcoholHpBonus2Perk = 0x6D,
    AlcoholHpNeg1Perk = 0x6E,
    AlcoholHpNeg2Perk = 0x6F,
    AutodocHpBonus1Perk = 0x70,
    AutodocHpBonus2Perk = 0x71,
    AutodocHpNeg1Perk = 0x72,
    AutodocHpNeg2Perk = 0x73,
    ExpertExcrementExpediterPerk = 0x74,
    WeaponKnockoutPerk = 0x75,
    JinxedPerk = 0x76,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum Skill {
    SmallGuns = 0x0,
    BigGuns = 0x1,
    EnergyWeapons = 0x2,
    UnarmedCombat = 0x3,
    Melee = 0x4,
    Throwing = 0x5,
    FirstAid = 0x6,
    Doctor = 0x7,
    Sneak = 0x8,
    Lockpick = 0x9,
    Steal = 0xa,
    Traps = 0xb,
    Science = 0xc,
    Repair = 0xd,
    Conversant = 0xe,
    Barter = 0xf,
    Gambling = 0x10,
    Outdoorsman = 0x11,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum WeaponKind {
    Unarmed     = 0,
    Knife       = 1,
    Club        = 2,
    Hammer      = 3,
    Spear       = 4,
    Pistol      = 5,
    Smg         = 6,
    Rifle       = 7,
    BigGun      = 8,
    Minigun     = 9,
    Launcher    = 10,
}

impl WeaponKind {
    pub fn anim_code(self) -> char {
        if self == WeaponKind::Unarmed {
            'a'
        } else {
            (b'c' + self as u8) as char
        }
    }
}

#[derive(Clone, Copy, Debug, EnumFlags)]
#[repr(u32)]
pub enum Flag {
    TurnedOff       = 0x1,
    Unk2            = 0x2,
    WalkThru        = 0x4,
    Flat            = 0x8,
    NoBlock         = 0x10,
    Lighting        = 0x20,
    Unk40           = 0x40,
    Unk80           = 0x80,
    Unk100          = 0x100,
    Unk200          = 0x200,
    Temp            = 0x400,
    MultiHex        = 0x800,
    NoHighlight     = 0x1000,
    Used            = 0x2000,
    TransRed        = 0x4000,
    TransNone       = 0x8000,
    TransWall       = 0x10000,
    TransGlass      = 0x20000,
    TransSteam      = 0x40000,
    TransEnergy     = 0x80000,
    Unk100000       = 0x100000,
    Unk200000       = 0x200000,
    Unk400000       = 0x400000,
    Unk800000       = 0x800000,
    LeftHand        = 0x1000000,
    RightHand       = 0x2000000,
    Worn            = 0x4000000,
    HiddenItem      = 0x8000000,
    WallTransEnd    = 0x10000000,
    LightThru       = 0x20000000,
    Seen            = 0x40000000,
    ShootThru       = 0x80000000,
}

#[derive(Clone, Copy, Debug, EnumFlags)]
#[repr(u32)]
pub enum FlagExt {
    Unk1                            = 0x1,
    Unk2                            = 0x2,
    Unk4                            = 0x4,
    Unk8                            = 0x8,
    Unk10                           = 0x10,
    Unk20                           = 0x20,
    CanTalkToMaybe                  = 0x40,
    Unk80                           = 0x80,
    BigGun                          = 0x100,
    TwoHanded                       = 0x200,
    Unk400                          = 0x400,
    CanUse                          = 0x800,
    CanUseOn                        = 0x1000,
    CanLook                         = 0x2000,
    CanTalk                         = 0x4000,
    CanPickup                       = 0x8000,
    Unk10000                        = 0x10000,
    Unk20000                        = 0x20000,
    Unk40000                        = 0x40000,
    Unk80000                        = 0x80000,
    Unk100000                       = 0x100000,
    Unk200000                       = 0x200000,
    Unk400000                       = 0x400000,
    Unk800000                       = 0x800000,
    Unk1000000                      = 0x1000000,
    Unk2000000                      = 0x2000000,
    Unk4000000                      = 0x4000000,
    //    ItemHidden = 0x8000000,
    WallEastOrWest                 = 0x8000000,
    WallNorthCorner                = 0x10000000,
    WallSouthCorner                = 0x20000000,
    WallEastCorner                 = 0x40000000,
    WallWestCorner                 = 0x80000000,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
pub enum AttackKind {
    Stand           = 0,
    Punch           = 1,
    Kick            = 2,
    Swing           = 3,
    Thrust          = 4,
    Throw           = 5,
    FireSingle      = 6,
    FireBurst       = 7,
    FireContinuous  = 8,
}

pub struct LstEntry {
    pub fields: Vec<String>,
}

pub fn read_lst(rd: &mut impl BufRead) -> io::Result<Vec<LstEntry>> {
    let mut r = Vec::new();
    for l in rd.lines() {
        let l = l?;
        let l = l.splitn(2, |c|
                c == ' '
                || c == ';'
                || c == '\t'
            ).next().unwrap_or(&l);
        let fields = l.split(',').map(|s| s.to_owned()).collect();
        r.push(LstEntry {
            fields,
        });
    }
    Ok(r)
}

pub fn read_gam(rd: &mut impl BufRead, tag: &str) -> io::Result<Vec<i32>> {
    use atoi::atoi;

    let mut r = Vec::new();
    let mut lines = rd.lines();
    while let Some(l) = lines.next() {
        if l?.starts_with(tag) {
            break;
        }
    }
    for l in lines {
        let l = l?;
        let l = l.trim();
        if l.is_empty() || l.starts_with("//") {
            continue;
        }

        let l = l.splitn(2, |c| c == ';').next().unwrap_or(&l);
        let v = l.find(":=")
            .and_then(|i| {
                let v = l[i + 2..].trim();
                if let Some(v) = atoi::<i32>(v.as_bytes()) {
                    Some(v)
                } else {
                    warn!("couldn't parse .gam var value as i32: {}", v);
                    None
                }
            })
            .unwrap_or(0);
        r.push(v);
    }
    Ok(r)
}

pub fn read_game_global_vars(rd: &mut impl BufRead) -> io::Result<Vec<i32>> {
    read_gam(rd, "GAME_GLOBAL_VARS:")
}

pub fn read_map_global_vars(rd: &mut impl BufRead) -> io::Result<Vec<i32>> {
    read_gam(rd, "MAP_GLOBAL_VARS:")
}
pub mod font;
pub mod frame;
pub mod map;
pub mod message;
pub mod palette;
pub mod proto;
pub mod script;

use enumflags2_derive::EnumFlags;
use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::io::prelude::*;

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
        static DIRS: &[&str] = &[
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

    UnarmedDmg = 0xA,
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

    CurrentHitPoints = 0x23,
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
    GeckoSkinning = 0x49,
    DermalArmor = 0x4A,
    DermalEnhancement = 0x4B,
    PhoenixArmor = 0x4C,
    PhoenixEnhancement = 0x4D,
    VaultCityInoculations = 0x4E,
    AdrenalineRush = 0x4F,
    CautiousNature = 0x50,
    Comprehension = 0x51,
    DemolitionExpert = 0x52,
    Gambler = 0x53,
    GainStrength = 0x54,
    GainPerception = 0x55,
    GainEndurance = 0x56,
    GainCharisma = 0x57,
    GainIntelligence = 0x58,
    GainAgility = 0x59,
    GainLuck = 0x5A,
    Harmless = 0x5B,
    HereAndNow = 0x5C,
    HthEvade = 0x5D,
    KamaSutra = 0x5E,
    KarmaBeacon = 0x5F,
    LightStep = 0x60,
    LivingAnatomy = 0x61,
    MagneticPersonality = 0x62,
    Negotiator = 0x63,
    PackRat = 0x64,
    Pyromaniac = 0x65,
    QuickRecovery = 0x66,
    Salesman = 0x67,
    Stonewall = 0x68,
    Thief = 0x69,
    WeaponHandling = 0x6A,
    VaultCityTraining = 0x6B,
    AlcoholHpBonus1 = 0x6C,
    AlcoholHpBonus2 = 0x6D,
    AlcoholHpNeg1 = 0x6E,
    AlcoholHpNeg2 = 0x6F,
    AutodocHpBonus1 = 0x70,
    AutodocHpBonus2 = 0x71,
    AutodocHpNeg1 = 0x72,
    AutodocHpNeg2 = 0x73,
    ExpertExcrementExpediter = 0x74,
    WeaponKnockout = 0x75,
    Jinxed = 0x76,
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
pub enum Trait {
    FastMetabolism = 0,
    Bruiser = 1,
    SmallFrame = 2,
    OneHanded = 3,
    Finesse = 4,
    Kamikaze = 5,
    HeavyHanded = 6,
    FastShot = 7,
    BloodyMess = 8,
    Jinxed = 9,
    GoodNatured = 10,
    ChemReliant = 11,
    ChemResistant = 12,
    SexAppeal = 13,
    Skilled = 14,
    Gifted = 15,
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

impl FlagExt {
    pub const ITEM_HIDDEN: Self = FlagExt::WallEastOrWest;
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
        let i = l.find(":=")
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "couldn't parse .gam var line"))?;
        let v = l[i + 2..].trim();
        let v = btoi::btoi::<i32>(v.as_bytes())
            .map_err(|_| Error::new(ErrorKind::InvalidData,
                format!("couldn't parse .gam var value as i32: `{}`", v)))?;
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

#[derive(Default)]
struct IniParser {
    sections: HashMap<String, HashMap<String, String>>,
    section: Option<(String, HashMap<String, String>)>,
}

impl IniParser {
    fn line(&mut self, mut line: &str) {
        if let Some(i) = line.find(';') {
            line = &line[..i];
        }
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if line.starts_with('[') && line.ends_with(']') {
            self.flush();
            let name = line[1..line.len() - 1].trim().to_owned();
            self.section = Some((name, HashMap::new()));
            return;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap().trim().to_owned();
        let value = parts.next().map(|s| s.trim()).unwrap_or("").to_owned();
        let section = &mut self.section.as_mut().unwrap().1;
        section.insert(key, value);
    }

    fn flush(&mut self) {
        if let Some((name, map)) = self.section.take() {
            self.sections.insert(name, map);
        }
    }
}

pub fn read_ini(rd: &mut impl BufRead) -> io::Result<HashMap<String, HashMap<String, String>>> {
    let mut parser = IniParser::default();

    for l in rd.lines() {
        let l = l?;
        parser.line(&l);
    }
    parser.flush();

    Ok(parser.sections)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Cursor, BufReader};

    #[test]
    fn read_game_global_vars_() {
        let s = "


 // Comments

//MAP_GLOBAL_VARS:
GAME_GLOBAL_VARS:
//GLOBAL                                                NUMBER

GVAR_0                  :=0;    //      (0)
 \t  GVAR_1             :=100;  //      (1)
GVAR_2                    :=   -123;    //      (2) blah blah blah";
        assert_eq!(read_game_global_vars(&mut BufReader::new(Cursor::new(s))).unwrap(),
            [0, 100, -123]);
    }

    #[test]
    fn read_map_global_vars_() {
        let s = "


 // Comments

//MAP_GLOBAL_VARS:
MAP_GLOBAL_VARS:
//GLOBAL                                NUMBER

MVAR_0                  :=123;    //      (0) blah blah blah
MVAR_1             :=0;  //      (1)
MVAR_2:=-1;    //      (2)";
        assert_eq!(read_map_global_vars(&mut BufReader::new(Cursor::new(s))).unwrap(),
            [123, 0, -1]);
    }

    #[test]
    fn read_ini_() {
        let inp = "

; comment
   ;  comment

[ sec tion_1\t]
 \tkey = \t va \t lue \t\x20

key2

     [section2]
     key2=stuff
     key=
        ";

        let exp = &[
            ("sec tion_1", &[
                ("key", "va \t lue"),
                ("key2", ""),
            ]),
            ("section2", &[
                ("key", ""),
                ("key2", "stuff"),
            ]),
        ];

        let mut exp_map = HashMap::new();
        for &(sect, entries) in exp {
            let mut exp_entries = HashMap::new();
            for &(k, v) in entries {
                assert!(exp_entries.insert(k.to_owned(), v.to_owned()).is_none());
            }
            exp_map.insert(sect.to_owned(), exp_entries);
        }

        let act = read_ini(&mut BufReader::new(Cursor::new(inp))).unwrap();
        assert_eq!(act, exp_map);
    }
}
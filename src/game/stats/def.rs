use enum_map::enum_map;

use super::*;

#[derive(Clone)]
pub struct StatDef {
    pub image_fid_id: u32,
    pub min: i32,
    pub max: i32,
    pub default: i32,
}

impl StatDef {
    const fn new(
        image_fid_id: u32,
        min: i32,
        max: i32,
        default: i32,
    ) -> Self {
        Self {
            image_fid_id,
            min,
            max,
            default
        }
    }

    pub fn defaults() -> EnumMap<Stat, StatDef> {
        EnumMap::from(|stat| STAT_DEFS[stat as usize].clone())
    }
}

static STAT_DEFS: &[StatDef] = &[
    StatDef::new(0, 1, 10, 5), // Strength
    StatDef::new(1, 1, 10, 5), // Perception
    StatDef::new(2, 1, 10, 5), // Endurance
    StatDef::new(3, 1, 10, 5), // Charisma
    StatDef::new(4, 1, 10, 5), // Intelligence
    StatDef::new(5, 1, 10, 5), // Agility
    StatDef::new(6, 1, 10, 5), // Luck
    StatDef::new(10, 0, 999, 0), // HitPoints
    StatDef::new(75, 1, 99, 0), // ActionPoints
    StatDef::new(18, 0, 999, 0), // ArmorClass
    StatDef::new(31, 0, i32::max_value(), 0), // UnarmedDmg
    StatDef::new(32, 0, 500, 0), // MeleeDmg
    StatDef::new(20, 0, 999, 0), // CarryWeight
    StatDef::new(24, 0, 60, 0), // Sequence
    StatDef::new(25, 0, 30, 0), // HealRate
    StatDef::new(26, 0, 100, 0), // CritChance
    StatDef::new(94, -60, 100, 0), // BetterCrit
    StatDef::new(0, 0, 100, 0), // DmgThresh
    StatDef::new(0, 0, 100, 0), // DmgThreshLaser
    StatDef::new(0, 0, 100, 0), // DmgThreshFire
    StatDef::new(0, 0, 100, 0), // DmgThreshPlasma
    StatDef::new(0, 0, 100, 0), // DmgThreshElectrical
    StatDef::new(0, 0, 100, 0), // DmgThreshEmp
    StatDef::new(0, 0, 100, 0), // DmgThreshExplosion
    StatDef::new(22, 0, 90, 0), // DmgResist
    StatDef::new(0, 0, 90, 0), // DmgResistLaser
    StatDef::new(0, 0, 90, 0), // DmgResistFire
    StatDef::new(0, 0, 90, 0), // DmgResistPlasma
    StatDef::new(0, 0, 90, 0), // DmgResistElectrical
    StatDef::new(0, 0, 100, 0), // DmgResistEmp
    StatDef::new(0, 0, 90, 0), // DmgResistExplosion
    StatDef::new(83, 0, 95, 0), // RadResist
    StatDef::new(23, 0, 95, 0), // PoisonResist
    StatDef::new(0, 16, 101, 25), // Age
    StatDef::new(0, 0, 1, 0), // Gender
    StatDef::new(10, 0, 2000, 0), // CurrentHitPoints
    StatDef::new(11, 0, 2000, 0), // CurrentPoison
    StatDef::new(12, 0, 2000, 0), // CurrentRad
];

pub struct SkillDef {
    pub image_fid_id: u32,
    pub base: i32,
    pub stat_multiplier: i32,
    pub stat1: Stat,
    pub stat2: Option<Stat>,
    pub experience: i32,
    pub flags: u32,
}

impl SkillDef {
    pub fn defaults() -> EnumMap<Skill, Self> {
        use Skill::*;
        use Stat::*;
        enum_map! {
            SmallGuns => Self {
                image_fid_id: 0x1c,
                base: 5,
                stat_multiplier: 4,
                stat1: Agility,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            BigGuns => Self {
                image_fid_id: 0x1d,
                base: 0,
                stat_multiplier: 2,
                stat1: Agility,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            EnergyWeapons => Self {
                image_fid_id: 0x1e,
                base: 0,
                stat_multiplier: 2,
                stat1: Agility,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            UnarmedCombat => Self {
                image_fid_id: 0x1f,
                base: 30,
                stat_multiplier: 2,
                stat1: Agility,
                stat2: Some(Strength),
                experience: 0,
                flags: 0,
            },
            Melee => Self {
                image_fid_id: 0x20,
                base: 20,
                stat_multiplier: 2,
                stat1: Agility,
                stat2: Some(Strength),
                experience: 0,
                flags: 0,
            },
            Throwing => Self {
                image_fid_id: 0x21,
                base: 0,
                stat_multiplier: 4,
                stat1: Agility,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            FirstAid => Self {
                image_fid_id: 0x22,
                base: 0,
                stat_multiplier: 2,
                stat1: Perception,
                stat2: Some(Intelligence),
                experience: 25,
                flags: 0,
            },
            Doctor => Self {
                image_fid_id: 0x23,
                base: 5,
                stat_multiplier: 1,
                stat1: Perception,
                stat2: Some(Intelligence),
                experience: 50,
                flags: 0,
            },
            Sneak => Self {
                image_fid_id: 0x24,
                base: 5,
                stat_multiplier: 3,
                stat1: Agility,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Lockpick => Self {
                image_fid_id: 0x25,
                base: 10,
                stat_multiplier: 1,
                stat1: Perception,
                stat2: Some(Intelligence),
                experience: 25,
                flags: 1,
            },
            Steal => Self {
                image_fid_id: 0x26,
                base: 0,
                stat_multiplier: 3,
                stat1: Agility,
                stat2: None,
                experience: 25,
                flags: 1,
            },
            Traps => Self {
                image_fid_id: 0x27,
                base: 10,
                stat_multiplier: 1,
                stat1: Perception,
                stat2: Some(Agility),
                experience: 25,
                flags: 1,
            },
            Science => Self {
                image_fid_id: 0x28,
                base: 0,
                stat_multiplier: 4,
                stat1: Intelligence,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Repair => Self {
                image_fid_id: 0x29,
                base: 0,
                stat_multiplier: 3,
                stat1: Intelligence,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Conversant => Self {
                image_fid_id: 0x2a,
                base: 0,
                stat_multiplier: 5,
                stat1: Charisma,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Barter => Self {
                image_fid_id: 0x2b,
                base: 0,
                stat_multiplier: 4,
                stat1: Charisma,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Gambling => Self {
                image_fid_id: 0x2c,
                base: 0,
                stat_multiplier: 5,
                stat1: Luck,
                stat2: None,
                experience: 0,
                flags: 0,
            },
            Outdoorsman => Self {
                image_fid_id: 0x2d,
                base: 0,
                stat_multiplier: 2,
                stat1: Endurance,
                stat2: Some(Intelligence),
                experience: 100,
                flags: 0,
            },
        }
    }
}
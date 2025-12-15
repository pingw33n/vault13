use linearize::{static_map, StaticMap};
use crate::asset::Stat;

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

    pub fn defaults() -> StaticMap<Stat, Self> {
        use Stat::*;
        static_map! {
            Strength => Self::new(0, 1, 10, 5),
            Perception => Self::new(1, 1, 10, 5),
            Endurance => Self::new(2, 1, 10, 5),
            Charisma => Self::new(3, 1, 10, 5),
            Intelligence => Self::new(4, 1, 10, 5),
            Agility => Self::new(5, 1, 10, 5),
            Luck => Self::new(6, 1, 10, 5),
            HitPoints => Self::new(10, 0, 999, 0),
            ActionPoints => Self::new(75, 1, 99, 0),
            ArmorClass => Self::new(18, 0, 999, 0),
            UnarmedDmg => Self::new(31, 0, i32::MAX, 0),
            MeleeDmg => Self::new(32, 0, 500, 0),
            CarryWeight => Self::new(20, 0, 999, 0),
            Sequence => Self::new(24, 0, 60, 0),
            HealRate => Self::new(25, 0, 30, 0),
            CritChance => Self::new(26, 0, 100, 0),
            BetterCrit => Self::new(94, -60, 100, 0),
            DmgThresh => Self::new(0, 0, 100, 0),
            DmgThreshLaser => Self::new(0, 0, 100, 0),
            DmgThreshFire => Self::new(0, 0, 100, 0),
            DmgThreshPlasma => Self::new(0, 0, 100, 0),
            DmgThreshElectrical => Self::new(0, 0, 100, 0),
            DmgThreshEmp => Self::new(0, 0, 100, 0),
            DmgThreshExplosion => Self::new(0, 0, 100, 0),
            DmgResist => Self::new(22, 0, 90, 0),
            DmgResistLaser => Self::new(0, 0, 90, 0),
            DmgResistFire => Self::new(0, 0, 90, 0),
            DmgResistPlasma => Self::new(0, 0, 90, 0),
            DmgResistElectrical => Self::new(0, 0, 90, 0),
            DmgResistEmp => Self::new(0, 0, 100, 0),
            DmgResistExplosion => Self::new(0, 0, 90, 0),
            RadResist => Self::new(83, 0, 95, 0),
            PoisonResist => Self::new(23, 0, 95, 0),
            Age => Self::new(0, 16, 101, 25),
            Gender => Self::new(0, 0, 1, 0),
            CurrentHitPoints => Self::new(10, 0, 2000, 0),
            CurrentPoison => Self::new(11, 0, 2000, 0),
            CurrentRad => Self::new(12, 0, 2000, 0),
        }
    }
}


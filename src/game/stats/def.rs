use super::*;

#[derive(Clone)]
pub struct StatDef {
    pub image_fid_id: u32,
    pub min: i32,
    pub max: i32,
    pub default: i32,
}

impl StatDef {
    pub fn defaults() -> EnumMap<Stat, StatDef> {
        EnumMap::from(|stat| DEFS[stat as usize].clone())
    }
}

const DEFS: &[StatDef] = &[
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
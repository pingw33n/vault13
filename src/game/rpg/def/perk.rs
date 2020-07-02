use enum_map::{enum_map, EnumMap};

use crate::asset::{Perk, Skill, Stat};

#[derive(Clone, Copy, Debug)]
pub enum ReqTarget {
    GlobalVar(i32),
    Skill(Skill),
    Stat(Stat),
}

#[derive(Clone, Copy, Debug)]
pub enum ReqValue {
    GreaterOrEqual(i32),
    Less(i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Req {
    /// Group to which this requirement belongs. Used to select the `ReqOp` when combining.
    pub group: u32,
    pub target: ReqTarget,
    pub value: ReqValue,
}

#[derive(Clone, Copy, Debug)]
pub enum ReqOp {
    Any,
    All,
}

pub struct PerkDef {
    pub image_fid_id: u32,
    pub rank_count: u32,
    pub min_level: u32,
    pub stat_bonus: Option<(Stat, i32)>,
    pub reqs: &'static [Req],

    /// Combine operation per each `Req::group`.
    pub req_ops: &'static [ReqOp],
}

impl PerkDef {
    pub fn defaults() -> EnumMap<Perk, PerkDef> {
        use Perk::*;
        use ReqOp::*;
        use ReqTarget::*;
        use ReqValue::*;
        use crate::asset::Skill::*;
        use crate::asset::Stat::*;
        use crate::game::script::GVAR_PLAYER_REPUTATION;

        enum_map! {
            BonusAwareness => Self {
                image_fid_id: 72,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            BonusHthAttacks => Self {
                image_fid_id: 73,
                rank_count: 1,
                min_level: 15,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            BonusHthDamage => Self {
                image_fid_id: 74,
                rank_count: 3,
                min_level: 3,
                stat_bonus: Some((MeleeDmg, 2)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(6) }, Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            BonusMove => Self {
                image_fid_id: 75,
                rank_count: 2,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            BonusRangedDamage => Self {
                image_fid_id: 76,
                rank_count: 2,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            BonusRateOfFire => Self {
                image_fid_id: 77,
                rank_count: 1,
                min_level: 15,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(7) }],
                req_ops: &[All, All],
            },
            EarlierSequence => Self {
                image_fid_id: 78,
                rank_count: 3,
                min_level: 3,
                stat_bonus: Some((Sequence, 2)),
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            FasterHealing => Self {
                image_fid_id: 79,
                rank_count: 3,
                min_level: 3,
                stat_bonus: Some((HealRate, 2)),
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            MoreCriticals => Self {
                image_fid_id: 80,
                rank_count: 3,
                min_level: 6,
                stat_bonus: Some((CritChance, 5)),
                reqs: &[Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            NightVision => Self {
                image_fid_id: 81,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Presence => Self {
                image_fid_id: 82,
                rank_count: 3,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            RadResistance => Self {
                image_fid_id: 83,
                rank_count: 2,
                min_level: 6,
                stat_bonus: Some((RadResist, 15)),
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Toughness => Self {
                image_fid_id: 84,
                rank_count: 3,
                min_level: 3,
                stat_bonus: Some((DmgResist, 10)),
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            StrongBack => Self {
                image_fid_id: 85,
                rank_count: 3,
                min_level: 3,
                stat_bonus: Some((CarryWeight, 50)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Sharpshooter => Self {
                image_fid_id: 86,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(7) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            SilentRunning => Self {
                image_fid_id: 87,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Sneak), value: GreaterOrEqual(50) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Survivalist => Self {
                image_fid_id: 88,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Outdoorsman), value: GreaterOrEqual(40) },
                    Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) }, Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            MasterTrader => Self {
                image_fid_id: 89,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Barter), value: GreaterOrEqual(75) },
                    Req { group: 1, target: Stat(Charisma), value: GreaterOrEqual(7) }],
                req_ops: &[All, All],
            },
            Educated => Self {
                image_fid_id: 90,
                rank_count: 3,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Healer => Self {
                image_fid_id: 91,
                rank_count: 2,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(FirstAid), value: GreaterOrEqual(40) },
                    Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(7) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(5) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            FortuneFinder => Self {
                image_fid_id: 92,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(8) }],
                req_ops: &[All, All],
            },
            BetterCriticals => Self {
                image_fid_id: 93,
                rank_count: 1,
                min_level: 9,
                stat_bonus: Some((BetterCrit, 20)),
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(4) },
                    Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Empathy => Self {
                image_fid_id: 94,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(7) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            Slayer => Self {
                image_fid_id: 95,
                rank_count: 1,
                min_level: 24,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(UnarmedCombat), value: GreaterOrEqual(80) },
                    Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(8) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(8) }],
                req_ops: &[All, All],
            },
            Sniper => Self {
                image_fid_id: 96,
                rank_count: 1,
                min_level: 24,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(SmallGuns), value: GreaterOrEqual(80) },
                    Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(8) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(8) }],
                req_ops: &[All, All],
            },
            SilentDeath => Self {
                image_fid_id: 97,
                rank_count: 1,
                min_level: 18,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Sneak), value: GreaterOrEqual(80) },
                    Req { group: 0, target: Skill(UnarmedCombat), value: GreaterOrEqual(80) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(10) }],
                req_ops: &[All, All],
            },
            ActionBoy => Self {
                image_fid_id: 98,
                rank_count: 2,
                min_level: 12,
                stat_bonus: Some((ActionPoints, 1)),
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            MentalBlock => Self {
                image_fid_id: 99,
                rank_count: 1,
                min_level: 310,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            Lifegiver => Self {
                image_fid_id: 100,
                rank_count: 2,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Dodger => Self {
                image_fid_id: 101,
                rank_count: 1,
                min_level: 9,
                stat_bonus: Some((ArmorClass, 5)),
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Snakeater => Self {
                image_fid_id: 102,
                rank_count: 2,
                min_level: 6,
                stat_bonus: Some((PoisonResist, 25)),
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(3) }],
                req_ops: &[All, All],
            },
            MrFixit => Self {
                image_fid_id: 103,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Repair), value: GreaterOrEqual(40) },
                    Req { group: 0, target: Skill(Science), value: GreaterOrEqual(40) }],
                req_ops: &[Any, All],
            },
            Medic => Self {
                image_fid_id: 104,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(FirstAid), value: GreaterOrEqual(40) },
                    Req { group: 0, target: Skill(Doctor), value: GreaterOrEqual(40) }],
                req_ops: &[Any, All],
            },
            MasterThief => Self {
                image_fid_id: 105,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Steal), value: GreaterOrEqual(50) },
                    Req { group: 0, target: Skill(Lockpick), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            Speaker => Self {
                image_fid_id: 106,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Conversant), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            HeaveHo => Self {
                image_fid_id: 107,
                rank_count: 3,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(9) }],
                req_ops: &[All, All],
            },
            FriendlyFoe => Self {
                image_fid_id: 108,
                rank_count: 1,
                min_level: 310,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Pickpocket => Self {
                image_fid_id: 109,
                rank_count: 1,
                min_level: 15,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Steal), value: GreaterOrEqual(80) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(8) }],
                req_ops: &[All, All],
            },
            Ghost => Self {
                image_fid_id: 110,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Sneak), value: GreaterOrEqual(60) }],
                req_ops: &[All, All],
            },
            CultOfPersonality => Self {
                image_fid_id: 111,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: GreaterOrEqual(10) }],
                req_ops: &[All, All],
            },
            Scrounger => Self {
                image_fid_id: 112,
                rank_count: 1,
                min_level: 310,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(8) }],
                req_ops: &[All, All],
            },
            Explorer => Self {
                image_fid_id: 113,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            FlowerChild => Self {
                image_fid_id: 114,
                rank_count: 1,
                min_level: 310,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            Pathfinder => Self {
                image_fid_id: 115,
                rank_count: 2,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Outdoorsman), value: GreaterOrEqual(40) },
                    Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            AnimalFriend => Self {
                image_fid_id: 116,
                rank_count: 1,
                min_level: 310,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Outdoorsman), value: GreaterOrEqual(25) },
                    Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            Scout => Self {
                image_fid_id: 117,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(7) }],
                req_ops: &[All, All],
            },
            MysteriousStranger => Self {
                image_fid_id: 118,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Ranger => Self {
                image_fid_id: 119,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            QuickPockets => Self {
                image_fid_id: 120,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            SmoothTalker => Self {
                image_fid_id: 121,
                rank_count: 3,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            SwiftLearner => Self {
                image_fid_id: 122,
                rank_count: 3,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Tag => Self {
                image_fid_id: 123,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            Mutate => Self {
                image_fid_id: 124,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AddNuka => Self {
                image_fid_id: 125,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AddBuffout => Self {
                image_fid_id: 126,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(2) },
                    Req { group: 1, target: Stat(Endurance), value: Less(2) },
                    Req { group: 1, target: Stat(Agility), value: Less(3) }],
                req_ops: &[All, All],
            },
            AddMentats => Self {
                image_fid_id: 127,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: Less(3) },
                    Req { group: 1, target: Stat(Agility), value: Less(2) }],
                req_ops: &[All, All],
            },
            AddPsycho => Self {
                image_fid_id: 128,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: Less(2) }],
                req_ops: &[All, All],
            },
            AddRadaway => Self {
                image_fid_id: 129,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((RadResist, -20)),
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponLongRange => Self {
                image_fid_id: 130,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponAccurate => Self {
                image_fid_id: 131,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponPenetrate => Self {
                image_fid_id: 132,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponKnockback => Self {
                image_fid_id: 133,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            ArmorPowered => Self {
                image_fid_id: 134,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((RadResist, 30)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(3) }],
                req_ops: &[All, All],
            },
            ArmorCombat => Self {
                image_fid_id: 135,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((RadResist, 20)),
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponScopeRange => Self {
                image_fid_id: 136,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponFastReload => Self {
                image_fid_id: 137,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponNightSight => Self {
                image_fid_id: 138,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponFlameboy => Self {
                image_fid_id: 139,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            ArmorAdvanced1 => Self {
                image_fid_id: 140,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((RadResist, 60)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            ArmorAdvanced2 => Self {
                image_fid_id: 141,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((RadResist, 75)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            AddJet => Self {
                image_fid_id: 136,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((ActionPoints, -1)),
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(1) },
                    Req { group: 1, target: Stat(Perception), value: Less(1) }],
                req_ops: &[All, All],
            },
            AddTragic => Self {
                image_fid_id: 149,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: Less(2) },
                    Req { group: 1, target: Stat(Intelligence), value: Less(1) },
                    Req { group: 1, target: Stat(Luck), value: Less(1) }],
                req_ops: &[All, All],
            },
            ArmorCharisma => Self {
                image_fid_id: 154,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: GreaterOrEqual(2) }],
                req_ops: &[All, All],
            },
            GeckoSkinning => Self {
                image_fid_id: 158,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            DermalArmor => Self {
                image_fid_id: 157,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            DermalEnhancement => Self {
                image_fid_id: 157,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((Charisma, -1)),
                reqs: &[],
                req_ops: &[All, All],
            },
            PhoenixArmor => Self {
                image_fid_id: 168,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            PhoenixEnhancement => Self {
                image_fid_id: 168,
                rank_count: 0,
                min_level: 1,
                stat_bonus: Some((Charisma, -1)),
                reqs: &[],
                req_ops: &[All, All],
            },
            VaultCityInoculations => Self {
                image_fid_id: 172,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AdrenalineRush => Self {
                image_fid_id: 155,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(10) }],
                req_ops: &[All, All],
            },
            CautiousNature => Self {
                image_fid_id: 156,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Comprehension => Self {
                image_fid_id: 122,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            DemolitionExpert => Self {
                image_fid_id: 39,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Traps), value: GreaterOrEqual(75) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(4) }],
                req_ops: &[All, All],
            },
            Gambler => Self {
                image_fid_id: 44,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Gambling), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            GainStrength => Self {
                image_fid_id: 0,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainPerception => Self {
                image_fid_id: 1,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Perception), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainEndurance => Self {
                image_fid_id: 2,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Endurance), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainCharisma => Self {
                image_fid_id: 3,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainIntelligence => Self {
                image_fid_id: 4,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Intelligence), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainAgility => Self {
                image_fid_id: 5,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: Less(10) }],
                req_ops: &[All, All],
            },
            GainLuck => Self {
                image_fid_id: 6,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Luck), value: Less(10) }],
                req_ops: &[All, All],
            },
            Harmless => Self {
                image_fid_id: 160,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Steal), value: GreaterOrEqual(50) },
                    Req { group: 0, target: GlobalVar(GVAR_PLAYER_REPUTATION), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            HereAndNow => Self {
                image_fid_id: 161,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            HthEvade => Self {
                image_fid_id: 159,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(UnarmedCombat), value: GreaterOrEqual(75) }],
                req_ops: &[All, All],
            },
            KamaSutra => Self {
                image_fid_id: 163,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Endurance), value: GreaterOrEqual(5) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            KarmaBeacon => Self {
                image_fid_id: 162,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            LightStep => Self {
                image_fid_id: 164,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) },
                    Req { group: 1, target: Stat(Luck), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            LivingAnatomy => Self {
                image_fid_id: 165,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Doctor), value: GreaterOrEqual(60) }],
                req_ops: &[All, All],
            },
            MagneticPersonality => Self {
                image_fid_id: 166,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Charisma), value: Less(10) }],
                req_ops: &[All, All],
            },
            Negotiator => Self {
                image_fid_id: 43,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Barter), value: GreaterOrEqual(50) },
                    Req { group: 0, target: Skill(Conversant), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            PackRat => Self {
                image_fid_id: 167,
                rank_count: 1,
                min_level: 6,
                stat_bonus: Some((CarryWeight, 50)),
                reqs: &[],
                req_ops: &[All, All],
            },
            Pyromaniac => Self {
                image_fid_id: 169,
                rank_count: 1,
                min_level: 9,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(BigGuns), value: GreaterOrEqual(75) }],
                req_ops: &[All, All],
            },
            QuickRecovery => Self {
                image_fid_id: 170,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            Salesman => Self {
                image_fid_id: 121,
                rank_count: 1,
                min_level: 6,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Barter), value: GreaterOrEqual(50) }],
                req_ops: &[All, All],
            },
            Stonewall => Self {
                image_fid_id: 171,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: GreaterOrEqual(6) }],
                req_ops: &[All, All],
            },
            Thief => Self {
                image_fid_id: 38,
                rank_count: 1,
                min_level: 3,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponHandling => Self {
                image_fid_id: 173,
                rank_count: 1,
                min_level: 12,
                stat_bonus: None,
                reqs: &[Req { group: 1, target: Stat(Strength), value: Less(7) },
                    Req { group: 1, target: Stat(Agility), value: GreaterOrEqual(5) }],
                req_ops: &[All, All],
            },
            VaultCityTraining => Self {
                image_fid_id: 104,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[Req { group: 0, target: Skill(Doctor), value: GreaterOrEqual(75) }],
                req_ops: &[All, All],
            },
            AlcoholHpBonus1 => Self {
                image_fid_id: 142,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AlcoholHpBonus2 => Self {
                image_fid_id: 142,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AlcoholHpNeg1 => Self {
                image_fid_id: 52,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AlcoholHpNeg2 => Self {
                image_fid_id: 52,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AutodocHpBonus1 => Self {
                image_fid_id: 104,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AutodocHpBonus2 => Self {
                image_fid_id: 104,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AutodocHpNeg1 => Self {
                image_fid_id: 35,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            AutodocHpNeg2 => Self {
                image_fid_id: 35,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            ExpertExcrementExpediter => Self {
                image_fid_id: 154,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            WeaponKnockout => Self {
                image_fid_id: 154,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
            Jinxed => Self {
                image_fid_id: 64,
                rank_count: 0,
                min_level: 1,
                stat_bonus: None,
                reqs: &[],
                req_ops: &[All, All],
            },
        }
    }
}
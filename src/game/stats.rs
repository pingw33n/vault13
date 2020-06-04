mod def;

use bstring::bstr;
use enum_map::EnumMap;
use num_traits::clamp;
use std::cmp;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io;

use crate::asset::{Perk, Skill, Stat, Trait};
use crate::asset::message::{Messages, MessageId};
use crate::asset::proto::{self, ProtoId};
use crate::game::object::{DamageFlag, Object, Objects};
use crate::fs::FileSystem;
use crate::util::random::*;

use def::*;

const STAT_NAME_MSG_BASE: MessageId = 100;
const STAT_DESCR_MSG_BASE: MessageId = 200;
const STAT_LEVEL_DESCR_BASE: MessageId = 300;
const PC_STAT_NAME_MSG_BASE: MessageId = 400;
const PC_STAT_DESCR_MSG_BASE: MessageId = 500;
const SKILL_NAME_MSG_BASE: MessageId = 100;
const SKILL_DESCR_MSG_BASE: MessageId = 200;
const SKILL_FORMULA_MSG_BASE: MessageId = 300;
const LEVEL_UP_MSG: MessageId = 600;
const PERK_NAME_MSG_BASE: MessageId = 101;
const PERK_DESCR_MSG_BASE: MessageId = 1101;

struct Tagged {
    tagged: bool,
    inc_base: bool,
}

impl Default for Tagged {
    fn default() -> Self {
        Self {
            tagged: false,
            inc_base: true,
        }
    }
}

pub struct Stats {
    stat_msgs: Messages,
    skill_msgs: Messages,
    perk_msgs: Messages,
    stat_defs: EnumMap<Stat, StatDef>,
    skill_defs: EnumMap<Skill, SkillDef>,
    perk_defs: EnumMap<Perk, PerkDef>,
    traits: EnumMap<Trait, bool>,
    perks: HashMap<ProtoId, EnumMap<Perk, bool>>,
    tagged: EnumMap<Skill, Tagged>,
}

impl Stats {
    pub fn new(fs: &FileSystem, language: &str) -> io::Result<Self> {
        let stat_msgs = Messages::read_file(fs, language, "game/stat.msg")?;
        let stat_defs = StatDef::defaults();

        let skill_msgs = Messages::read_file(fs, language, "game/skill.msg")?;
        let skill_defs = SkillDef::defaults();

        let perk_msgs = Messages::read_file(fs, language, "game/perk.msg")?;
        let perk_defs = PerkDef::defaults();

        let mut perks = HashMap::new();
        perks.insert(ProtoId::DUDE, Default::default());
        Ok(Self {
            stat_msgs,
            skill_msgs,
            perk_msgs,
            stat_defs,
            skill_defs,
            perk_defs,
            traits: Default::default(),
            perks,
            tagged: Default::default(),
        })
    }

    pub fn skill_name(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_NAME_MSG_BASE + skill as MessageId).unwrap().text
    }

    pub fn skill_description(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_DESCR_MSG_BASE + skill as MessageId).unwrap().text
    }

    pub fn skill_formula(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_FORMULA_MSG_BASE + skill as MessageId).unwrap().text
    }

    pub fn perk_name(&self, perk: Perk) -> &bstr {
        &self.perk_msgs.get(PERK_NAME_MSG_BASE + perk as MessageId).unwrap().text
    }

    pub fn perk_description(&self, perk: Perk) -> &bstr {
        &self.perk_msgs.get(PERK_DESCR_MSG_BASE + perk as MessageId).unwrap().text
    }

    pub fn has_perk(&self, perk: Perk, pid: ProtoId) -> bool {
        self.perks.get(&pid).map(|m| m[perk]).unwrap_or(false)
    }

    pub fn has_trait(&self, tr: Trait) -> bool {
        self.traits[tr]
    }

    pub fn is_tagged(&self, skill: Skill) -> bool {
        self.tagged[skill].tagged
    }

    // stat_level()
    pub fn stat(&self, stat: Stat, obj: &Object, objs: &Objects) -> i32 {
        use Perk::*;
        use Stat::*;

        let pei = |p| self.has_perk(p, obj.proto_id().unwrap()) as i32;

        if stat == Age {
            // TODO
            return 35;
        }

        let mut r = self.base_stat(stat, obj) + self.bonus_stat(stat, obj);
        if stat == ArmorClass /* in_combat && whose_turn != obj */ {
            // TODO
        }
        if stat == Perception && obj.sub.as_critter().unwrap().combat.damage_flags.contains(DamageFlag::Blind) {
            r -= 5;
        }
        if stat == ActionPoints {
            let carry_weight = self.stat(CarryWeight, obj, objs);
            let left = carry_weight - i32::try_from(obj.inventory.weight(objs)).unwrap();
            if left < 0 {
                r -= -left / 40 + 1;
            }
        }
        if obj.proto_id() == Some(ProtoId::DUDE) {
            r += self.trait_stat_mod(stat, obj);

            r += match stat {
                Strength => {
                    pei(GainStrength) +
                        if self.has_perk(AdrenalineRush, obj.proto_id().unwrap()) &&
                            self.stat(CurrentHitPoints, obj, objs) < self.stat(HitPoints, obj, objs) / 2
                        {
                            1
                        } else {
                            0
                        }
                }
                Perception => pei(GainPerception),
                Endurance => pei(GainEndurance),
                Charisma => {
                    pei(GainCharisma)

                    // TODO
//                    bonus_for_shades = 0;
//                  obj_in_right_hand = inven_right_hand_(obj);
//                  if ( obj_in_right_hand && obj_in_right_hand->_.pid == PID_MIRRORED_SHADES )
//                    bonus_for_shades = 1;
//                  obj_in_left_hand = inven_left_hand_(obj);
//                  if ( obj_in_left_hand && obj_in_left_hand->_.pid == PID_MIRRORED_SHADES )
//                    bonus_for_shades = 1;
//                  if ( bonus_for_shades )
//                    ++v7;
                }
                Intelligence => pei(GainIntelligence),
                Agility => pei(GainAgility),
                Luck => pei(GainLuck),
                PoisonResist => 10 * pei(VaultCityInoculations),
                RadResist => 10 * pei(VaultCityInoculations),
                HitPoints =>
                    2 * pei(AlcoholHpBonus1) +
                    4 * pei(AlcoholHpBonus2) +
                    -2 * pei(AlcoholHpNeg1) +
                    -4 * pei(AlcoholHpNeg2) +
                    2 * pei(AutodocHpBonus1) +
                    4 * pei(AutodocHpBonus2) +
                    -2 * pei(AutodocHpNeg1) +
                    -4 * pei(AutodocHpNeg2),
                DmgResistLaser | DmgResistFire | DmgResistPlasma =>
                    if self.has_perk(PhoenixArmor, obj.proto_id().unwrap()) {
                        5
                    } else if self.has_perk(PhoenixEnhancement, obj.proto_id().unwrap()) {
                        10
                    } else {
                        0
                    }
                DmgResist | DmgResistExplosion =>
                    if self.has_perk(DermalArmor, obj.proto_id().unwrap()) {
                        5
                    } else if self.has_perk(DermalEnhancement, obj.proto_id().unwrap()) {
                        10
                    } else {
                        0
                    }
                _ => 0,
            }
        }

        let stat_def = &self.stat_defs[stat];
        clamp(r, stat_def.min, stat_def.max)
    }

    // skill_level
    pub fn skill(&self, skill: Skill, obj: &Object, objs: &Objects) -> i32 {
        let level = obj.proto().unwrap().sub.as_critter().unwrap().skills[skill];

        let def = &self.skill_defs[skill];

        let mut from_stats = self.stat(def.stat1, obj, objs);
        if let Some(stat) = def.stat2 {
            from_stats += self.stat(stat, obj, objs);
        }

        let mut r = def.base + def.stat_multiplier * from_stats + level;

        if obj.proto_id().unwrap().is_dude() {
            if self.tagged[skill].tagged {
                r += level;
            }
            if self.tagged[skill].inc_base {
                r += 20;
            }
            r += self.trait_skill_mod(skill) + self.perk_skill_mod(skill, obj);
            // TODO r+= skill_game_difficulty()
        }

        cmp::min(r, 300)
    }

    // stat_result
    pub fn roll_check_stat(&self,
        stat: Stat,
        bonus: i32,
        obj: &Object,
        objs: &Objects,
    ) -> (RollCheckResult, i32) {
        let level = self.stat(stat, obj, objs) + bonus;
        let rnd = random(1, 10);
        let diff = level - rnd;
        let r = if rnd <= level {
            RollCheckResult::Success
        } else {
            RollCheckResult::Failure
        };
        (r, diff)
    }

    // skill_result
    pub fn roll_check_skill(&self,
        skill: Skill,
        bonus: i32,
        roll_checker: RollChecker,
        obj: &Object,
        objs: &Objects,
    ) -> (RollCheckResult, i32) {
        if obj.proto_id().unwrap().is_dude() && skill != Skill::Steal {
            // TODO
            // pm = partyMemberWithHighestSkill_(skill);
            // v10 = pm;
            // if ( pm )
            // {
            //   if ( partyMemberSkill_(pm) == skill )
            //     critter = v10;
            // }
        }
        let level = self.skill(skill, obj, objs);

        // TODO
        // if ( critter == g_obj_dude && skill == SKILL_STEAL && is_pc_flag_(0) )
        // {
        //   if ( is_pc_sneak_working_() )
        //     skill_level += 30;
        // }

        let crit_level = self.stat(Stat::CritChance, obj, objs);
        roll_checker.roll_check(bonus + level, crit_level)
    }

    // trait_adjust_stat()
    fn trait_stat_mod(&self, stat: Stat, obj: &Object) -> i32 {
        let tr = |tr| {
            self.has_trait(tr) as i32
        };
        let st = |stat| {
            self.base_stat(stat, obj)
        };
        use Stat::*;
        use Trait::*;
        match stat {
            Strength => tr(Gifted) + tr(Bruiser),
            Perception => tr(Gifted),
            Endurance => tr(Gifted),
            Charisma => tr(Gifted),
            Intelligence => tr(Gifted),
            Agility => tr(Gifted) + tr(SmallFrame),
            Luck => tr(Gifted),
            ActionPoints => -2 * tr(Bruiser),
            ArmorClass => -st(ArmorClass) * tr(Kamikaze),
            MeleeDmg => 4 * tr(HeavyHanded),
            CarryWeight => -10 * st(ArmorClass) * tr(SmallFrame),
            Sequence => 5 * tr(Kamikaze),
            HealRate => 2 * tr(FastMetabolism),
            CritChance => 10 * tr(Finesse),
            BetterCrit => 30 * tr(HeavyHanded),
            RadResist => -st(RadResist) * tr(FastMetabolism),
            PoisonResist => -st(PoisonResist) * tr(FastMetabolism),
            _ => 0,
        }
    }

    // trait_adjust_skill
    fn trait_skill_mod(&self, skill: Skill) -> i32 {
        let mut r = 0;
        if self.has_trait(Trait::Gifted) {
            r -= 10;
        }
        if self.has_trait(Trait::GoodNatured) {
            match skill {
                | Skill::SmallGuns
                | Skill::BigGuns
                | Skill::EnergyWeapons
                | Skill::UnarmedCombat
                | Skill::Melee
                | Skill::Throwing
                => r -= 10,

                | Skill::FirstAid
                | Skill::Doctor
                | Skill::Conversant
                | Skill::Barter
                => r += 15,

                _ => {}
            }
        }
        r
    }

    // perk_adjust_skill
    fn perk_skill_mod(&self, skill: Skill, obj: &Object) -> i32 {
        let pid = obj.proto_id().unwrap();
        let p = |p| {
            self.has_perk(p, pid) as i32
        };

        let thief = || p(Thief) * 10;
        let master_thief = || p(MasterThief) * 15;
        let harmless = || p(Harmless) * 20;
        let negotiator = || p(Negotiator) * 10;

        use Skill::*;
        use Perk::*;
        match skill {
            Science | Repair => p(MrFixit) * 10,
            FirstAid => p(Medic) * 10 + p(VaultCityTraining) * 5,
            Doctor => p(Medic) * 10 + p(VaultCityTraining) * 5 + p(LivingAnatomy) * 10,
            Sneak => (self.has_perk(Ghost, pid) /* && TODO obj_get_visible_light_(g_obj_dude) <= 45875 */ as i32) * 20
                + thief() + harmless(),
            Lockpick => thief() + master_thief(),
            Steal => thief() + master_thief() + harmless(),
            Traps => thief(),
            Conversant => p(Speaker) * 20 + p(ExpertExcrementExpediter) * 5 + negotiator(),
            Barter => negotiator() + p(Salesman) * 20,
            Outdoorsman => p(Ranger) * 15 + p(Survivalist) * 25,
            _ => 0,
        }
    }

    // stat_get_base_direct()
    fn base_stat(&self, stat: Stat, obj: &Object) -> i32 {
        let critter = || obj.sub.as_critter().unwrap();
        match stat {
            Stat::CurrentHitPoints => critter().health,
            Stat::CurrentPoison => critter().poison,
            Stat::CurrentRad => critter().radiation,
            _ =>  self.with_critter_proto(obj, |c| c.base_stats[stat]),
        }
    }

    fn bonus_stat(&self, stat: Stat, obj: &Object) -> i32 {
        self.with_critter_proto(obj, |c| c.bonus_stats[stat])
    }

    fn with_critter_proto<F, R>(&self, obj: &Object, f: F) -> R
        where F: FnOnce(&proto::Critter) -> R
    {
        let proto = obj.proto().unwrap();
        f(proto.sub.as_critter().unwrap())
    }
}

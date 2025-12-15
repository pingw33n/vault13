mod def;

use bstring::bstr;
use linearize::{static_map, StaticMap};
use num_traits::clamp;
use std::cmp;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io;
use crate::asset::{DamageKind, ExactEntityKind, Perk, PCStat, Skill, Stat, Trait};
use crate::asset::message::{Messages, MessageId};
use crate::asset::proto::ProtoId;
use crate::game::object::{DamageFlag, EquipmentSlot, Hand, Object, Objects};
use crate::fs::FileSystem;
use crate::util::random::*;

use def::perk::*;
use def::pc_stat::*;
use def::skill::*;
use def::stat::*;

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

pub struct Rpg {
    stat_msgs: Messages,
    skill_msgs: Messages,
    perk_msgs: Messages,
    stat_defs: StaticMap<Stat, StatDef>,
    skill_defs: StaticMap<Skill, SkillDef>,
    perk_defs: StaticMap<Perk, PerkDef>,
    traits: StaticMap<Trait, bool>,
    perks: HashMap<ProtoId, StaticMap<Perk, u32>>,
    tagged: StaticMap<Skill, Tagged>,
    pc_stat_defs: StaticMap<PCStat, PCStatDef>,
    pc_stats: StaticMap<PCStat, i32>,
}

impl Rpg {
    pub fn new(fs: &FileSystem, language: &str) -> io::Result<Self> {
        let stat_msgs = Messages::read_file(fs, language, "game/stat.msg")?;
        let stat_defs = StatDef::defaults();

        let skill_msgs = Messages::read_file(fs, language, "game/skill.msg")?;
        let skill_defs = SkillDef::defaults();

        let perk_msgs = Messages::read_file(fs, language, "game/perk.msg")?;
        let perk_defs = PerkDef::defaults();

        let mut perks = HashMap::new();
        perks.insert(ProtoId::DUDE, Default::default());

        let pc_stat_defs = PCStatDef::defaults();
        let pc_stats: StaticMap<PCStat, i32> = static_map! {
            s => pc_stat_defs[s].default
        };

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
            pc_stat_defs,
            pc_stats,
        })
    }

    pub fn skill_msgs(&self) -> &Messages {
        &self.skill_msgs
    }

    // skill_name
    pub fn skill_name(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_NAME_MSG_BASE + skill as MessageId).unwrap().text
    }

    // skill_description
    pub fn skill_description(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_DESCR_MSG_BASE + skill as MessageId).unwrap().text
    }

    // skill_attribute
    pub fn skill_formula(&self, skill: Skill) -> &bstr {
        &self.skill_msgs.get(SKILL_FORMULA_MSG_BASE + skill as MessageId).unwrap().text
    }

    // perk_name
    pub fn perk_name(&self, perk: Perk) -> &bstr {
        &self.perk_msgs.get(PERK_NAME_MSG_BASE + perk as MessageId).unwrap().text
    }

    // perk_description
    pub fn perk_description(&self, perk: Perk) -> &bstr {
        &self.perk_msgs.get(PERK_DESCR_MSG_BASE + perk as MessageId).unwrap().text
    }

    // stat_pc_name
    pub fn pc_stat_name(&self, pc_stat: PCStat) -> &bstr {
        &self.stat_msgs.get(PC_STAT_NAME_MSG_BASE + pc_stat as MessageId).unwrap().text
    }

    // stat_pc_description
    pub fn pc_stat_description(&self, pc_stat: PCStat) -> &bstr {
        &self.stat_msgs.get(PC_STAT_DESCR_MSG_BASE + pc_stat as MessageId).unwrap().text
    }

    // perk_level
    pub fn perk(&self, perk: Perk, pid: ProtoId) -> u32 {
        self.perks.get(&pid).map(|m| m[perk]).unwrap_or(0)
    }

    pub fn has_perk(&self, perk: Perk, pid: ProtoId) -> bool {
        self.perk(perk, pid) > 0
    }

    // perk_can_add
    pub fn can_add_perk(&self,
        perk: Perk,
        obj: &Object,
        objs: &Objects,
        global_vars: &[i32],
    ) -> bool {
        let def = &self.perk_defs[perk];
        let max_rank = if let Some(v) = def.max_rank {
            v
        } else {
            return false;
        };
        if self.perk(perk, obj.proto_id().unwrap()) >= max_rank
            || obj.is_dude() && (self.pc_stat(PCStat::Level) as u32) < def.min_level
        {
            return false;
        }
        let req_test = |(req, value)| {
            let v = match req {
                ReqTarget::Stat(stat) => self.stat(stat, obj, objs),
                ReqTarget::Skill(skill) => self.skill(skill, obj, objs),
                ReqTarget::GlobalVar(gvar) => global_vars[gvar],
            };
            match value {
                ReqValue::GreaterOrEqual(rv) => v >= rv,
                ReqValue::Less(rv) => v < rv,
            }
        };
        let any = def.any.map(|any| any.iter().copied().any(req_test)).unwrap_or(true);
        let all = def.all.iter().copied().all(req_test);
        any && all
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

        let pei = |p| self.perk(p, obj.proto_id().unwrap()) as i32;

        if stat == Age {
            // TODO
            return 35;
        }

        let mut r = self.stat_base(stat, obj) + self.bonus_stat(stat, obj);
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
                    let wearing_shades = [
                        EquipmentSlot::Hand(Hand::Left),
                        EquipmentSlot::Hand(Hand::Right)
                    ].iter()
                        .flat_map(|&s| obj.equipment(s, objs).into_iter())
                        .flat_map(|o| objs.get(o).proto_id().into_iter())
                        .any(|pid| pid == ProtoId::MIRRORED_SHADES);
                    pei(GainCharisma) + wearing_shades as i32
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

    // stat_recalc_derived
    pub fn recalc_derived_stats(&self, obj: &mut Object, objs: &Objects) {
        use Stat::*;

        let str = self.stat(Strength, obj, objs);
        let per = self.stat(Perception, obj, objs);
        let end = self.stat(Endurance, obj, objs);
        let agi = self.stat(Agility, obj, objs);
        let luck = self.stat(Luck, obj, objs);

        let hp = self.stat_base(Endurance, obj) * 2 + self.stat_base(Strength, obj) + 15;

        let mut proto = obj.proto_mut().unwrap();
        let bs = &mut proto.sub.as_critter_mut().unwrap().base_stats;

        bs[ActionPoints] = agi / 2 + 5;
        bs[ArmorClass] = agi;
        bs[CarryWeight] = 25 * str + 25;
        bs[CritChance] = luck;
        bs[HealRate] = cmp::max(end / 3, 1);
        bs[HitPoints] = hp;
        bs[MeleeDmg] = cmp::max(str - 5, 1);
        bs[PoisonResist] = 5 * end;
        bs[RadResist] = 2 * end;
        bs[Sequence] = 2 * per;
    }

    // adjust_ac
    pub fn apply_armor_change(&self,
        obj: &mut Object,
        new_armor: Option<&Object>,
        old_armor: Option<&Object>,
        objs: &Objects,
    ) {
        let armor_stat = |obj: Option<&Object>, stat| obj.as_ref().map(|o|
            o.proto().unwrap().sub.as_armor().unwrap().stat(stat).unwrap())
            .unwrap_or(0);
        for stat in
            [Stat::ArmorClass].iter().copied()
                .chain(DamageKind::basic().iter().map(|d| d.resist_stat()))
                .chain(DamageKind::basic().iter().map(|d| d.thresh_stat().unwrap()))
        {
            let new = self.bonus_stat(stat, obj)
                - armor_stat(old_armor, stat)
                + armor_stat(new_armor, stat);
            self.set_bonus_stat(stat, obj, new, objs);
        }
        if obj.is_dude() { // TODO isPartyMember
            if let Some(old_perk) = old_armor.as_ref()
                .and_then(|o| o.proto().unwrap().sub.as_armor().unwrap().perk)
            {
                self.remove_perk_effect(old_perk, obj, objs)
            }
            if let Some(new_perk) = new_armor.as_ref()
                .and_then(|o| o.proto().unwrap().sub.as_armor().unwrap().perk)
            {
                self.add_perk_effect(new_perk, obj, objs);
            }
        }
    }

    // perk_add_effect
    fn add_perk_effect(&self, perk: Perk, obj: &mut Object, objs: &Objects) {
        assert_eq!(obj.proto().unwrap().kind(), ExactEntityKind::Critter);
        let def = &self.perk_defs[perk];
        for &(stat, bonus) in def.stat_bonuses {
            let v = self.bonus_stat(stat, obj);
            self.set_bonus_stat(stat, obj, v + bonus, objs);
        }
        if perk == Perk::HereAndNow {
            // TODO
            // v7 = (char *) & perkGetLevelData_(obj) -> levels[PERK_here_and_now_perk];
            // --*(_DWORD *)
            // v7;
            // player_level = stat_pc_get_(PCSTAT_level);
            // v9 = get_experience_for_level(player_level + 1);
            // dword_51C124 = v9 - stat_pc_get_(PCSTAT_experience);
            // statPCAddExperienceCheckPMs_(dword_51C124, 0);
            // + + *(_DWORD *)
            // v7;
        }
    }

    // perk_add_effect
    fn remove_perk_effect(&self, perk: Perk, obj: &mut Object, objs: &Objects) {
        assert_eq!(obj.proto().unwrap().kind(), ExactEntityKind::Critter);
        let def = &self.perk_defs[perk];
        for &(stat, bonus) in def.stat_bonuses {
            let v = self.bonus_stat(stat, obj);
            self.set_bonus_stat(stat, obj, v - bonus, objs);
        }
        if perk == Perk::HereAndNow {
            // TODO does this make sense at all?
            //let v = self.pc_stat(PCStat::Experience);
            //self.try_set_pc_stat(PCStat::Experience, v - HereAndNow_experience_boost);
        }
    }

    // trait_adjust_stat()
    fn trait_stat_mod(&self, stat: Stat, obj: &Object) -> i32 {
        let tr = |tr| {
            self.has_trait(tr) as i32
        };
        let st = |stat| {
            self.stat_base_direct(stat, obj)
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

    // stat_pc_get
    pub fn pc_stat(&self, pc_stat: PCStat) -> i32 {
        self.pc_stats[pc_stat]
    }

    // stat_pc_set
    pub fn try_set_pc_stat(&mut self, pc_stat: PCStat, value: i32) -> bool {
        let def = &self.pc_stat_defs[pc_stat];
        if value < def.min || value > def.max {
            return false;
        }
        if pc_stat != PCStat::Experience || value >= self.pc_stat(PCStat::Experience) {
            self.pc_stats[pc_stat] = value;
            if pc_stat == PCStat::Experience {
                // TODO  statPCAddExperienceCheckPMs_(0, 1);
            }
        } else {
            // TODO statPcResetExperience_(value)
        }
        true
    }

    // stat_pc_min_exp
    pub fn next_level_experience(&self) -> u32 {
        level_experience(self.pc_stat(PCStat::Level) as u32 + 1)
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
        let p = |p| self.perk(p, pid) as i32;

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
    fn stat_base_direct(&self, stat: Stat, obj: &Object) -> i32 {
        let critter = || obj.sub.as_critter().unwrap();
        match stat {
            Stat::CurrentHitPoints => critter().hit_points,
            Stat::CurrentPoison => critter().poison,
            Stat::CurrentRad => critter().radiation,
            _ => obj.proto().unwrap().sub.as_critter().unwrap().base_stats[stat],
        }
    }

    // stat_get_base
    fn stat_base(&self, stat: Stat, obj: &Object) -> i32 {
        let mut r = self.stat_base_direct(stat, obj);
        if obj.proto_id() == Some(ProtoId::DUDE) {
            r += self.trait_stat_mod(stat, obj);
        }
        r
    }

    // stat_get_bonus
    fn bonus_stat(&self, stat: Stat, obj: &Object) -> i32 {
        obj.proto().unwrap().sub.as_critter().unwrap().bonus_stats[stat]
    }

    // stat_set_bonus
    fn set_bonus_stat(&self, stat: Stat, obj: &mut Object, v: i32, objs: &Objects) {
        match stat {
            Stat::CurrentHitPoints => unimplemented!("TODO"),
            Stat::CurrentPoison => unimplemented!("TODO"),
            Stat::CurrentRad => unimplemented!("TODO"),
            _ => {
                obj.proto_mut().unwrap()
                    .sub.as_critter_mut().unwrap().bonus_stats[stat] = v;
                if stat.is_base() {
                    self.recalc_derived_stats(obj, objs);
                }
            }
        }
    }
}

pub fn level_experience(level: u32) -> u32 {
    try_level_experience(level).expect("level experience overflow/underflow")
}

// get_experience_for_level
fn try_level_experience(level: u32) -> Option<u32> {
    (level.checked_mul(level.checked_sub(1)?)? / 2).checked_mul(1000)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn try_level_experience_() {
        let f = try_level_experience;
        assert_eq!(f(0), None);
        assert_eq!(f(1), Some(0));
        assert_eq!(f(2), Some(1000));
        assert_eq!(f(3), Some(3000));
        assert_eq!(f(20), Some(190_000));
        assert_eq!(f(21), Some(210_000));
        assert_eq!(f(98), Some(4_753_000));
        assert_eq!(f(99), Some(4_851_000));
    }
}
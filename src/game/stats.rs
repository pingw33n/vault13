mod def;

use enum_map::EnumMap;
use num_traits::clamp;
use std::collections::HashMap;
use std::rc::Rc;
use std::io;

use crate::asset::{Stat, Trait, Perk};
use crate::asset::message::Messages;
use crate::asset::proto::{self, ProtoDb};
use crate::game::object::{DamageFlag, Object, ObjectProtoId};
use crate::fs::FileSystem;

use def::StatDef;

const STAT_NAME_MSG_BASE: u32 = 100;
const STAT_DESCR_MSG_BASE: u32 = 200;
const STAT_LEVEL_DESCR_BASE: u32 = 300;
const PC_STAT_NAME_MSG_BASE: u32 = 400;
const PC_STAT_DESCR_MSG_BASE: u32 = 500;
const LEVEL_UP_MSG: u32 = 600;

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
}

pub struct Stats {
    proto_db: Rc<ProtoDb>,
    stat_msgs: Messages,
    stat_defs: EnumMap<Stat, StatDef>,
    traits: Vec<Trait>,
    perks: HashMap<ObjectProtoId, EnumMap<Perk, bool>>,
}

impl Stats {
    pub fn new(fs: &FileSystem, proto_db: Rc<ProtoDb>, language: &str) -> io::Result<Self> {
        let stat_msgs = Messages::read_file(fs, language, "game/stat.msg")?;
        let stat_defs = StatDef::defaults();

        let mut perks = HashMap::new();
        perks.insert(ObjectProtoId::Dude, Default::default());
        Ok(Self {
            proto_db,
            stat_msgs,
            stat_defs,
            traits: Vec::new(),
            perks,
        })
    }

    pub fn has_perk(&self, perk: Perk, pid: ObjectProtoId) -> bool {
        self.perks.get(&pid).map(|m| m[perk]).unwrap_or(false)
    }

    pub fn has_trait(&self, tr: Trait) -> bool {
        self.traits.contains(&tr)
    }

    // stat_level()
    pub fn stat(&self, stat: Stat, obj: &Object) -> i32 {
        use Perk::*;
        use Stat::*;

        let pei = |p| self.has_perk(p, obj.pid) as i32;

        if stat == Age {
            // TODO
            return 35;
        }

        let mut r = self.base_stat(stat, obj) + self.bonus_stat(stat, obj);
        if stat == ArmorClass /* in_combat && whose_turn != obj */ {
            // TODO
        }
        if stat == Perception && obj.sub.critter().unwrap().combat.damage_flags.contains(DamageFlag::Blind) {
            r -= 5;
        }
        if stat == ActionPoints {
            let carry_weight = self.stat(CarryWeight, obj);
            let left = carry_weight - /* TODO item_total_weight_(obj) */ 0;
            if left < 0 {
                r -= -left / 40 + 1;
            }
        }
        if obj.pid == ObjectProtoId::Dude {
            r += self.trait_modifier(stat, obj);

            r += match stat {
                Strength => {
                    pei(GainStrength) +
                        if self.has_perk(AdrenalineRush, obj.pid) &&
                            self.stat(CurrentHitPoints, obj) < self.stat(HitPoints, obj) / 2
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
                    if self.has_perk(PhoenixArmor, obj.pid) {
                        5
                    } else if self.has_perk(PhoenixEnhancement, obj.pid) {
                        10
                    } else {
                        0
                    }
                DmgResist | DmgResistExplosion =>
                    if self.has_perk(DermalArmor, obj.pid) {
                        5
                    } else if self.has_perk(DermalEnhancement, obj.pid) {
                        10
                    } else {
                        0
                    }
                _ => 0,
            }
        }

        let stat_def = &self.stat_defs[stat];
        let r = clamp(r, stat_def.min, stat_def.max);

        r
    }

    // trait_adjust_stat()
    fn trait_modifier(&self, stat: Stat, obj: &Object) -> i32 {
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

    // stat_get_base_direct()
    fn base_stat(&self, stat: Stat, obj: &Object) -> i32 {
        let critter = || obj.sub.critter().unwrap();
        match stat {
            Stat::CurrentHitPoints => critter().health,
            Stat::CurrentPoison => critter().poison,
            Stat::CurrentRad => critter().radiation,
            _ => self.with_critter_proto(obj, |c| c.base_stats[stat]),
        }
    }

    fn bonus_stat(&self, stat: Stat, obj: &Object) -> i32 {
        self.with_critter_proto(obj, |c| c.bonus_stats[stat])
    }

    fn with_critter_proto<F, R>(&self, obj: &Object, f: F) -> R
        where F: FnOnce(&proto::Critter) -> R
    {
        let proto = self.proto_db.proto(obj.pid.proto_id().unwrap()).unwrap();
        let proto = proto.borrow();
        f(proto.sub.critter().unwrap())
    }
}

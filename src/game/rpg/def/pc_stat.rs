use enum_map::{enum_map, EnumMap};

use crate::asset::PCStat;

pub struct PCStatDef {
    pub min: i32,
    pub max: i32,
    pub default: i32,
}

impl PCStatDef {
    pub fn defaults() -> EnumMap<PCStat, Self> {
        use PCStat::*;
        enum_map! {
            UnspentSkillPoints => Self {
                min: 0,
                max: i32::max_value(),
                default: 0,
            },
            Level => Self {
                min: 1,
                max: 99,
                default: 1,
            },
            Experience => Self {
                min: 0,
                max: i32::max_value(),
                default: 0,
            },
            Reputation => Self {
                min: -20,
                max: 20,
                default: 0,
            },
            Karma => Self {
                min: 0,
                max: i32::max_value(),
                default: 0,
            },
        }
    }
}
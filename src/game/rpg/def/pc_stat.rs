use linearize::{static_map, StaticMap};

use crate::asset::PCStat;

pub struct PCStatDef {
    pub min: i32,
    pub max: i32,
    pub default: i32,
}

impl PCStatDef {
    pub fn defaults() -> StaticMap<PCStat, Self> {
        use PCStat::*;
        static_map! {
            UnspentSkillPoints => Self {
                min: 0,
                max: i32::MAX,
                default: 0,
            },
            Level => Self {
                min: 1,
                max: 99,
                default: 1,
            },
            Experience => Self {
                min: 0,
                max: i32::MAX,
                default: 0,
            },
            Reputation => Self {
                min: -20,
                max: 20,
                default: 0,
            },
            Karma => Self {
                min: 0,
                max: i32::MAX,
                default: 0,
            },
        }
    }
}
use rand::{thread_rng, Rng};

// roll_random()
pub fn random(from_inclusive: i32, to_inclusive: i32) -> i32 {
    thread_rng().gen_range(from_inclusive..=to_inclusive)
}

#[derive(Clone, Copy, Debug, PartialEq, enum_primitive_derive::Primitive)]
pub enum RollCheckResult {
    CriticalFailure = 0,
    Failure = 1,
    Success = 2,
    CriticalSuccess = 3,
}

impl RollCheckResult {
    pub fn is_success(self) -> bool {
        match self {
            Self::Success | Self::CriticalSuccess => true,
            Self::Failure | Self::CriticalFailure => false,
        }
    }

    pub fn is_critical(self) -> bool {
        match self {
            Self::CriticalSuccess | Self::CriticalFailure => true,
            Self::Success | Self::Failure => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RollChecker {
    disable_crits: bool,
}

impl RollChecker {
    pub fn new(disable_crits: bool) -> Self {
        Self {
            disable_crits,
        }
    }

    pub fn roll_check(self, target: i32, crit: i32) -> (RollCheckResult, i32) {
        let roll = target - random(1, 100);
        let r = if roll < 0 {
            if !self.disable_crits && random(1, 100) <= -roll / 10 {
                RollCheckResult::CriticalFailure
            } else {
                RollCheckResult::Failure
            }
        } else if !self.disable_crits && random(1, 100) <= roll / 10 + crit {
            RollCheckResult::CriticalSuccess
        } else {
            RollCheckResult::Success
        };
        (r, roll)
    }
}

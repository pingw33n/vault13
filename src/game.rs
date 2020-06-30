pub mod dialog;
pub mod fidget;
pub mod inventory;
pub mod object;
pub mod rpg;
pub mod script;
pub mod sequence;
pub mod skilldex;
pub mod state;
pub mod ui;
pub mod world;

use crate::util::random::RollChecker;

#[derive(Clone, Copy)]
pub struct GameTime(u32);

impl GameTime {
    pub const fn from_decis(decis: u32) -> Self {
        Self(decis)
    }

    pub fn as_decis(self) -> u32 {
        self.0
    }

    pub fn as_seconds(self) -> u32 {
        self.0 / 10
    }

    pub fn as_minutes(self) -> u32 {
        self.as_seconds() / 60
    }

    pub fn as_hours(self) -> u32 {
        self.as_minutes() / 60
    }

    pub fn year(self) -> u16 {
        self.ydm().0
    }

    pub fn month(self) -> u8 {
        self.ydm().1
    }

    pub fn day(self) -> u8 {
        self.ydm().2
    }

    pub fn hour(self) -> u8 {
        (self.as_hours() % 24) as u8
    }

    pub fn minute(self) -> u8 {
        (self.as_minutes() % 60) as u8
    }

    pub fn second(self) -> u8 {
        (self.as_seconds() % 60) as u8
    }

    pub fn decisecond(self) -> u8 {
        (self.as_decis() % 10) as u8
    }

    pub fn roll_checker(self) -> RollChecker {
        RollChecker::new(self.as_hours() >= 1)
    }

    fn ydm(self) -> (u16, u8, u8) {
        const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        let days = self.0 / 864000 + 24;
        let mut year = days / 365 + 2241;
        let mut month = 6;
        let mut day = days % 365;
        loop {
            let days_in_month = DAYS_IN_MONTH[month as usize].into();
            if day < days_in_month {
                break;
            }
            day -= days_in_month;
            month += 1;
            if month >= 12 {
                year += 1;
                month = 0;
            }
        }
        (year as u16, month as u8 + 1, day as u8 + 1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(test)]
    mod game_time {
        use super::*;

        #[test]
        fn test() {
            let t = GameTime::from_decis(302412);
            assert_eq!(t.year(), 2241);
            assert_eq!(t.month(), 7);
            assert_eq!(t.day(), 25);
            assert_eq!(t.hour(), 8);
            assert_eq!(t.minute(), 24);
            assert_eq!(t.second(), 1);
            assert_eq!(t.decisecond(), 2);
        }
    }
}
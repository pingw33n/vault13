pub mod hex;
pub mod map;
pub mod sqr;

use util::EnumExt;

#[derive(Clone, Copy, Debug, Enum, PartialEq, PartialOrd, Primitive)]
pub enum Direction {
    NE  = 0,
    E   = 1,
    SE  = 2,
    SW  = 3,
    W   = 4,
    NW  = 5,
}

impl Direction {
    pub const LEN: usize = 6;

    pub fn values() -> &'static [Direction] {
        static VALUES: [Direction; Direction::LEN] =
            [Direction::NE, Direction::E, Direction::SE, Direction::SW, Direction::W, Direction::NW];
        &VALUES[..]
    }

    pub fn rotate_cw(self) -> Self {
        Self::from_ordinal((self.ordinal() + 1) % Self::len())
    }

    pub fn rotate_ccw(self) -> Self {
        let mut o = self.ordinal() as isize - 1;
        if o < 0 {
            o += Self::len() as isize;
        }
        Self::from_ordinal(o as usize)
    }
}


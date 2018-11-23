pub mod hex;
pub mod sqr;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Primitive)]
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
}


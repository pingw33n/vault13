pub mod hex;
pub mod map;
pub mod sqr;

use graphics::{Point, Rect};
use util::EnumExt;

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd, Primitive)]
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

fn from_screen_rect(rect: &Rect, clip: bool, from_screen: impl Fn(Point) -> Point,
        clip_fn: impl Fn(Point) -> Point) -> Rect {
    let right = rect.right - 1;
    let bottom = rect.bottom - 1;

    let x = from_screen(Point::new(rect.left, bottom)).x;
    let y = from_screen(Point::new(rect.left, rect.top)).y;
    let top_left = if clip {
        clip_fn(Point::new(x, y))
    } else {
        Point::new(x, y)
    };

    let x = from_screen(Point::new(right, rect.top)).x;
    let y = from_screen(Point::new(right, bottom)).y;
    let bottom_right_incl = if clip {
        clip_fn(Point::new(x, y))
    } else {
        Point::new(x, y)
    };

    Rect::with_points(top_left, bottom_right_incl + Point::new(1, 1))
}
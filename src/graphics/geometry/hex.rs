pub mod path_finder;

use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use num_traits::{clamp, FromPrimitive};
use std::cmp;
use std::f64::consts::PI;

use crate::graphics::{Point, Rect};
use crate::util::EnumExt;

const TILE_WIDTH: i32 = 32;
const TILE_HEIGHT: i32 = 16;
const TILE_INNER_HEIGHT: i32 = 8;

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

/// Offset in screen coordinates between to adjacent hexes when going in `direction`.
pub fn screen_offset(direction: Direction) -> Point {
    const H: i32 = TILE_INNER_HEIGHT + (TILE_HEIGHT - TILE_INNER_HEIGHT) / 2;
    match direction {
        Direction::NE   => (TILE_HEIGHT, -H),
        Direction::E    => (TILE_WIDTH, 0),
        Direction::SE   => (TILE_HEIGHT, H),
        Direction::SW   => (-TILE_HEIGHT, H),
        Direction::W    => (-TILE_WIDTH, 0),
        Direction::NW   => (-TILE_HEIGHT, -H),
    }.into()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TileHit {
    Inside,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn tile_hit_test(p: impl Into<Point>) -> TileHit {
    let p = p.into();
    let line_test = |x1: i32, y1: i32, x2: i32, y2: i32| -> i32 {
        (p.x - x1) * (y2 - y1) - (p.y - y1) * (x2 - x1)
    };

    if line_test(0, TILE_INNER_HEIGHT / 2, TILE_WIDTH / 2, 0) > 0 {
        return TileHit::TopLeft;
    }
    if line_test(TILE_WIDTH / 2, 0, TILE_WIDTH, TILE_INNER_HEIGHT / 2) > 0 {
        return TileHit::TopRight;
    }
    if line_test(0, TILE_HEIGHT - TILE_INNER_HEIGHT / 2, TILE_WIDTH / 2, TILE_HEIGHT) <= 0 {
        return TileHit::BottomLeft;
    }
    if line_test(TILE_WIDTH / 2, TILE_HEIGHT, TILE_WIDTH, TILE_HEIGHT - TILE_INNER_HEIGHT / 2) <= 0 {
        return TileHit::BottomRight;
    }
    TileHit::Inside
}

#[derive(Clone, Debug)]
pub struct TileGrid {
    // Position in screen coordinates.
    // Tile at `pos` will be mapped to screen at screen_pos.
    screen_pos: Point,

    // Width in tiles.
    width: i32,

    // Height in tiles.
    height: i32,
}

impl TileGrid {
    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn screen_pos(&self) -> Point {
        self.screen_pos
    }

    pub fn screen_pos_mut(&mut self) -> &mut Point {
        &mut self.screen_pos
    }

    pub fn set_screen_pos(&mut self, pos: impl Into<Point>) {
        self.screen_pos = pos.into();
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn is_on_edge(&self, p: impl Into<Point>) -> bool {
        let p = p.into();
        p.x == 0 ||
            p.x == self.width - 1 ||
            p.y == 0 ||
            p.y == self.height - 1
    }

    pub fn go(&self, p: impl Into<Point>, direction: Direction, distance: u32) -> Option<Point> {
        let p = self.go0(p, direction, distance, false);
        if self.is_in_bounds(p) {
            Some(p)
        } else {
            None
        }
    }

    pub fn go_unbounded(&self, p: impl Into<Point>, direction: Direction, distance: u32) -> Point {
        self.go0(p, direction, distance, false)
    }

    pub fn go_clipped(&self, p: impl Into<Point>, direction: Direction, distance: u32) -> Point {
        self.go0(p, direction, distance, true)
    }

    // tile_num()
    /// Returns tile coordinates.
    pub fn from_screen(&self, p: impl Into<Point>) -> Point {
        let p = p.into();

        let abs_screen_y = p.y - self.screen_pos.y;

        // 12 is vertical hex advance
        let mut tile_y = if abs_screen_y >= 0 {
            abs_screen_y / 12
        } else {
            (abs_screen_y + 1) / 12 - 1
        };

        // 16 is horizontal hex advance
        let screen_x_in_tile_hrow = p.x - self.screen_pos.x - 16 * tile_y;

        let screen_y_in_tile = abs_screen_y - 12 * tile_y;

        let tile_hx = if screen_x_in_tile_hrow >= 0 {
            screen_x_in_tile_hrow / 64
        } else {
            (screen_x_in_tile_hrow + 1) / 64 - 1
        };

        tile_y += tile_hx;
        let mut screen_x_in_tile = screen_x_in_tile_hrow - tile_hx * 64;
        let mut tile_x = 2 * tile_hx;
        if screen_x_in_tile >= 32 {
            screen_x_in_tile -= 32;
            tile_x += 1;
        }

        match tile_hit_test((screen_x_in_tile, screen_y_in_tile)) {
            TileHit::TopRight => {
                tile_x += 1;
                if tile_x % 2 == 1 {
                    tile_y -= 1;
                }
            }
            TileHit::TopLeft => {
                tile_y -= 1;
            }
            TileHit::BottomLeft => {
                tile_x -= 1;
                if tile_x % 2 == 0 {
                    tile_y += 1;
                }
            }
            TileHit::BottomRight => {
                tile_y += 1;
            }
            TileHit::Inside => {}
        }

        Point::new(tile_x, tile_y)
    }

    /// Returns minimal rectangle in local coordinates that encloses the specified screen `rect`.
    /// Clips the resulting rectangle if `clip` is `true`.
    pub fn from_screen_rect(&self, rect: &Rect, clip: bool) -> Rect {
        super::from_screen_rect(rect, clip, |p| self.from_screen(p), |p| self.clip(p))
    }

    // tile_coord()
    pub fn to_screen(&self, p: impl Into<Point>) -> Point {
        let p = p.into();
        let mut r = self.screen_pos;
        let dx = p.x / 2;
        r.x += 48 * dx;
        r.y += 12 * -dx;
        if p.x % 2 != 0 {
            if p.x <= 0 {
                r.x -= 16;
                r.y += 12;
            } else {
                r.x += 32;
            }
        }
        let dy = p.y;
        r.x += 16 * dy;
        r.y += 12 * dy;

        r
    }

    // tile_dir()
    pub fn direction(&self, from: impl Into<Point>, to: impl Into<Point>) -> Direction {
        let from = from.into();
        let to = to.into();
        assert_ne!(from, to);
        let from_scr = self.to_screen(from);
        let to_scr = self.to_screen(to);
        let d = to_scr - from_scr;
        if d.x != 0 {
            let angle_degrees = (-d.y as f64).atan2(d.x as f64) * 180.0 / PI;
            let a = 90 - angle_degrees as i32;
            let direction = cmp::min((a + 360) % 360 / 60, 5);
            Direction::from_usize(direction as usize).unwrap()
        } else if d.y < 0 {
            Direction::NE
        } else {
            Direction::SE
        }
    }

    // Is p1 located in front of p2 if looking in SE direction?
    // tile_in_front_of()
    pub fn is_in_front_of(&self, p1: impl Into<Point>, p2: impl Into<Point>) -> bool {
        let sp1 = self.to_screen(p1);
        let sp2 = self.to_screen(p2);
        sp2.x - sp1.x <= (sp2.y - sp1.y) * -4
    }

    // Is p1 located to right of p2 if looking in SE direction?
    // tile_to_right_of()
    pub fn is_to_right_of(&self, p1: impl Into<Point>, p2: impl Into<Point>) -> bool {
        let sp1 = self.to_screen(p1);
        let sp2_ = self.to_screen(p2);
        sp1.x - sp2_.x <= (sp1.y - sp2_.y) * 32 / (12 * 2)
    }

    // tile_dist()
    pub fn distance(&self, p1: impl Into<Point>, p2: impl Into<Point>) -> i32 {
        let mut p1 = p1.into();
        let p2 = p2.into();
        let mut distance = 0;
        while p1 != p2 {
            let dir = self.direction(p1, p2);
            p1 = self.go_unbounded(p1, dir, 1);
            distance += 1;
        }
        distance
    }

    /// Casts line between two tile centers and returns coordinates of tile that is `n`-th distinct
    /// intersection of line and tiles that lie beyond `to` if going from `from`,
    /// where `n` is the `distance`. Clips the result to grid bounds.
    // tile_num_beyond()
    pub fn beyond(&self, from: impl Into<Point>, to: impl Into<Point>, distance: i32) -> Point {
        let from = from.into();
        let to = to.into();

        assert_ne!(from, to);
        assert!(distance >= 0);

        if distance == 0 {
            return from;
        }

        let froms = self.to_screen(from).add((16, 8));
        let tos = self.to_screen(to).add((16, 8));

        let delta_x = tos.x - froms.x;
        let abs_delta_x_mult_2 = 2 * delta_x.abs();
        let x_inc = delta_x.signum();

        let delta_y = tos.y - froms.y;
        let abs_delta_y_mult_2 = 2 * delta_y.abs();
        let y_inc = delta_y.signum();

        let mut cur = from;
        let mut curs = froms;
        let mut cur_distance = 0;

        if abs_delta_x_mult_2 > abs_delta_y_mult_2 {
            let mut j = abs_delta_y_mult_2 - abs_delta_x_mult_2 / 2;
            loop {
                let next = self.from_screen(curs);
                if next != cur {
                    cur_distance += 1;
                    if cur_distance == distance || self.is_on_edge(next) {
                        return next;
                    }
                    cur = next;
                }
                if j >= 0 {
                    j -= abs_delta_x_mult_2;
                    curs.y += y_inc;
                }
                j += abs_delta_y_mult_2;
                curs.x += x_inc;
            }
        }

        let mut j = abs_delta_x_mult_2 - abs_delta_y_mult_2 / 2;
        loop {
            let next = self.from_screen(curs);
            if next != cur {
                cur_distance += 1;
                if cur_distance == distance || self.is_on_edge(next) {
                    return next;
                }
                cur = next;
            }
            if j >= 0 {
                j -= abs_delta_y_mult_2;
                curs.x += x_inc;
            }
            j += abs_delta_x_mult_2;
            curs.y += y_inc;
        }
    }

    /// Linear to rectangular coordinates.
    /// Note this is different from original since the `x` axis is not inverted,
    /// see `from_linear_inv()` for the inverted variant.
    pub fn from_linear(&self, num: u32) -> Point {
        Point::new(num as i32 % self.width, num as i32 / self.width)
    }

    /// Rectangular to linear coordinates.
    /// Note this is different from original since the `x` axis is not inverted,
    /// see `to_linear_inv()` for the inverted variant.
    pub fn to_linear(&self, p: impl Into<Point>) -> Option<u32> {
        let p = p.into();
        if self.is_in_bounds(p) {
            Some((self.width * p.y + p.x) as u32)
        } else {
            None
        }
    }

    /// Rectangular to linear coordinates with `x` axis inverted.
    /// This method should be used when converting linears for use in the original assets
    /// (maps, scripts etc).
    pub fn to_linear_inv(&self, p: impl Into<Point>) -> Option<u32> {
        let p = p.into();
        if self.is_in_bounds(p) {
            let x = self.width - 1 - p.x;
            Some((self.width * p.y + x) as u32)
        } else {
            None
        }
    }

    /// Linear to rectangular coordinates with `x` axis inverted.
    /// This method should be used when converting linears for use in the original assets
    /// (maps, scripts etc).
    pub fn from_linear_inv(&self, num: u32) -> Point {
        let x = self.width - 1 - num as i32 % self.width;
        let y = num as i32 / self.width;
        Point::new(x, y)
    }

    /// Verifies the tile coordinates `p` are within (0, 0, width, height) boundaries.
    pub fn is_in_bounds(&self, p: impl Into<Point>) -> bool {
        let p = p.into();
        p.x >= 0 && p.x < self.width && p.y >= 0 && p.y < self.height
    }

    pub fn clip(&self, p: impl Into<Point>) -> Point {
        let p = p.into();
        Point {
            x: clamp(p.x, 0, self.width - 1),
            y: clamp(p.y, 0, self.height - 1),
        }
    }

    /// Inverts `x` coordinate. 0 becomes `width - 1` and `width - 1` becomes 0.
    pub fn invert_x(&self, x: i32) -> i32 {
        self.width - 1 - x
    }

    // tile_num_in_direction_()
    fn go0(&self, p: impl Into<Point>, direction: Direction, distance: u32, clip: bool) -> Point {
        // Advance per each direction for even/odd hex.
        static ADVANCE_MAP: [[(i32, i32); enum_len!(Direction)]; 2] = [
            [(1, -1), (1, 0), (0, 1), (-1, 0), (-1, -1), (0, -1)],
            [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (0, -1)],
        ];
        let mut p = p.into();
        for _ in 0..distance {
            let advance = ADVANCE_MAP[p.x as usize % 2][direction as usize].into();
            let next = p + advance;
            if clip && !self.is_in_bounds(next) {
                break;
            }
            p = next;
        }
        p
    }
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            screen_pos: Point::default(),
            width: 200,
            height: 200,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tile_hit_test_() {
        let expected = [
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
            [1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2],
            [1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 4, 4],
            [3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4],
            [3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4]];
        for y in 0..16 {
            for x in 0..32 {
                assert_eq!(tile_hit_test(Point::new(x, y)) as usize, expected[y as usize][x as usize]);
            }
        }
    }

    #[test]
    fn from_screen1() {
        let t = TileGrid {
            screen_pos: Point::new(272, 182),
            .. Default::default()
        };
        assert_eq!(t.from_screen((-320, -240)), Point::new(-1, -37));
        assert_eq!(t.from_screen((-320, 620)), Point::new(-37, 17));
        assert_eq!(t.from_screen((256, -242)), Point::new(17, -28));
    }

    #[test]
    fn from_screen2() {
        let mut t = TileGrid::default();

        for spos in &[Point::new(0, 0), Point::new(100, 200)] {
            t.set_screen_pos(spos);
            assert_eq!(t.from_screen(spos.add((0, 0))), Point::new(0, -1));
            assert_eq!(t.from_screen(spos.add((16, 0))), Point::new(0, 0));
            assert_eq!(t.from_screen(spos.add((48, 0))), Point::new(1, 0));
            assert_eq!(t.from_screen(spos.add((48, -1))), Point::new(2, 0));
            assert_eq!(t.from_screen(spos.add((0, 4))), Point::new(0, 0));
        }
    }

    #[test]
    fn to_screen() {
        let t = TileGrid::default();

        assert_eq!(t.to_screen((0, 0)), Point::new(0, 0));
        assert_eq!(t.to_screen(t.from_linear_inv(12702)), Point::new(3344, 180));
    }

    #[test]
    fn to_screen1() {
        let t = TileGrid {
            screen_pos: Point::new(272, 182),
            .. Default::default()
        };

        assert_eq!(t.to_screen(t.from_linear_inv(12702)), Point::new(3616, 362));
    }

    #[test]
    fn to_screen2() {
        let t = TileGrid::default();
        assert_eq!(t.to_screen((0, 0)), Point::new(0, 0));
    }

    #[test]
    fn go() {
        let t = TileGrid::default();
        assert_eq!(t.go_unbounded((0, 0), Direction::W, 1), Point::new(-1, -1));
        assert_eq!(t.go((0, 0), Direction::W, 1), None);
        assert_eq!(t.go_clipped((0, 0), Direction::W, 1), Point::new(0, 0));
        assert_eq!(t.go_unbounded((22, 11), Direction::E, 0), Point::new(22, 11));
        assert_eq!(t.go_unbounded((22, 11), Direction::E, 1), Point::new(23, 11));
    }

    #[test]
    fn direction() {
        let t = TileGrid::default();

        for dir in Direction::iter() {
            for dist in 1..=10 {
                let from = (100, 100);
                let to = t.go_unbounded(from, dir, dist);
                assert_eq!(t.direction(from, to), dir);
            }
        }

        assert_eq!(t.direction(t.from_linear_inv(21101), t.from_linear_inv(18488)), Direction::NE);
    }

    #[test]
    fn distance() {
        let t = TileGrid::default();
        assert_eq!(t.distance((1234, -5678), (1234, -5678)), 0);

        assert_eq!(t.distance(t.from_linear_inv(0x4838), t.from_linear_inv(0x526d)), 19);
        assert_eq!(t.distance(t.from_linear_inv(0x526d), t.from_linear_inv(0x4838)), 19);

        assert_eq!(t.distance(t.from_linear_inv(0x7023), t.from_linear_inv(0x5031)), 52);
        assert_eq!(t.distance(t.from_linear_inv(0x5031), t.from_linear_inv(0x7023)), 52);
    }

    #[test]
    fn is_in_front_of() {
        let t = TileGrid::default();
        assert_eq!(t.is_in_front_of(t.from_linear_inv(0x4450), t.from_linear_inv(0x3e10)), true);
        assert_eq!(t.is_in_front_of((100, 100), (100, 100)), true);
        assert_eq!(t.is_in_front_of((101, 100), (100, 100)), true);
        assert_eq!(t.is_in_front_of((100, 101), (100, 100)), true);
        assert_eq!(t.is_in_front_of((100, 99), (100, 100)), false);
    }

    #[test]
    fn is_to_right_of() {
        let t = TileGrid::default();
        assert_eq!(t.is_to_right_of((100, 100), (100, 100)), true);
        assert_eq!(t.is_to_right_of((99, 100), (100, 100)), true);
        assert_eq!(t.is_to_right_of((100, 99), (100, 100)), true);
        assert_eq!(t.is_to_right_of((100, 101), (100, 100)), true);
        assert_eq!(t.is_to_right_of((99, 99), (100, 100)), true);

        assert_eq!(t.is_to_right_of((101, 100), (100, 100)), false);
        assert_eq!(t.is_to_right_of((101, 99), (100, 100)), false);
        assert_eq!(t.is_to_right_of((101, 101), (100, 100)), false);
    }

    #[test]
    fn beyond() {
        let t = TileGrid {
            screen_pos: Point::new(0x130, 0xb6),
            .. Default::default()
        };

        assert_eq!(t.beyond(t.from_linear_inv(0x65f8), t.from_linear_inv(0x5b03), 0x19),
            t.from_linear_inv(0x571a));
        assert_eq!(t.beyond(t.from_linear_inv(0x65f8), t.from_linear_inv(0x5d67), 0x19),
            t.from_linear_inv(0x5982));
    }
}
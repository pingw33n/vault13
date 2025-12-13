pub mod path_finder;

use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use num_traits::{clamp, FromPrimitive};
use std::cmp;
use std::f64::consts::PI;

use crate::graphics::{Point, Rect};
use crate::util::EnumExt;
use super::TileGridView;

pub const TILE_WIDTH: i32 = 32;
pub const TILE_HEIGHT: i32 = 16;
pub const TILE_INNER_HEIGHT: i32 = 8;
pub const TILE_CENTER: Point = Point::new(TILE_WIDTH / 2, TILE_HEIGHT / 2);

#[derive(Clone, Copy, Debug, Default, Enum, Eq, Hash, Ord, PartialEq, PartialOrd, Primitive)]
pub enum Direction {
    #[default]
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

pub fn go(p: Point, direction: Direction, distance: u32) -> Point {
    go0(p, direction, distance, |_| true)
}

// tile_num_in_direction_()
fn go0(mut p: Point, direction: Direction, distance: u32, is_in_bounds: impl Fn(Point) -> bool)
    -> Point
{
    // Advance per each direction for even/odd hex.
    static ADVANCE_MAP: [[(i32, i32); enum_len!(Direction)]; 2] = [
        [(1, -1), (1, 0), (0, 1), (-1, 0), (-1, -1), (0, -1)],
        [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (0, -1)],
    ];
    for _ in 0..distance {
        let advance = ADVANCE_MAP[p.x as usize % 2][direction as usize].into();
        let next = p + advance;
        if !is_in_bounds(next) {
            break;
        }
        p = next;
    }
    p
}

/// Returns tile that is directly above or below the tile at `p` in screen space.
/// The `offset` defines the number of steps to go up if negative or down if positive.
pub fn go_vert(p: Point, offset: i32) -> Point {
    let i = offset / 2;
    let j = offset % 2;
    let k = if offset > 0 {
        1 + p.x.abs() % 2
    } else {
        2 - p.x.abs() % 2
    };
    p + Point::new(-offset, i * 3 + j * k)
}

#[test]
fn go_vert_() {
    let d = &[
        ((123, -456), 0, (123, -456)),
        ((0, 0), 1, (-1, 1)),
        ((0, 0), -1, (1, -2)),
        ((0, 0), 2, (-2, 3)),
        ((1, 0), 1, (0, 2)),
        ((111, 87), 1, (110, 89)),
        ((111, 87), -1, (112, 86)),
        ((111, 87), 5, (106, 95)),
    ];
    for &((ix, iy), o, (ex, ey)) in d {
        let inp = (ix, iy).into();
        let exp = (ex, ey).into();
        assert_eq!(go_vert(inp, o), exp);
        assert_eq!(go_vert(exp, -o), inp);
    }
}

/// Creates iterator over positions of distinct tiles that intersect the ray cast from `from` tile
/// center via `via` tile center.
///
/// # Panics
///
/// * Panics if `from == via`.
/// * Might panic if internal values overflow `i32`.
pub fn ray(from: Point, via: Point) -> Ray {
    Ray::new(from, via)
}

pub struct Ray {
    first: bool,
    pos_scr: Point,
    pos_hex: Point,
    delta_x: i32,
    delta_y: i32,
    i: i32,
}

impl Ray {
    fn new(from: Point, via: Point) -> Self {
        assert_ne!(from, via);
        let from_scr = center_to_screen(from);
        let via_scr = center_to_screen(via);

        let delta_x = via_scr.x - from_scr.x;
        let delta_y = via_scr.y - from_scr.y;
        let i = if delta_x.abs() > delta_y.abs() {
            2 * delta_y.abs() - delta_x.abs()
        } else {
            2 * delta_x.abs() - delta_y.abs()
        };

        Self {
            first: true,
            pos_scr: from_scr,
            pos_hex: from,
            delta_x,
            delta_y,
            i,
        }
    }
}

impl Iterator for Ray {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            return Some(self.pos_hex);
        }
        let dec_i = 2 * self.delta_x.abs();
        let inc_x = self.delta_x.signum();
        let inc_i = 2 * self.delta_y.abs();
        let inc_y = self.delta_y.signum();
        if dec_i > inc_i {
            loop {
                let next_hex = from_screen(self.pos_scr);
                if next_hex != self.pos_hex {
                    self.pos_hex = next_hex;
                    break Some(self.pos_hex);
                }
                if self.i >= 0 {
                    self.i -= dec_i;
                    self.pos_scr.y += inc_y;
                }
                self.i += inc_i;
                self.pos_scr.x += inc_x;
            }
        } else {
            loop {
                let next_hex = from_screen(self.pos_scr);
                if next_hex != self.pos_hex {
                    self.pos_hex = next_hex;
                    break Some(self.pos_hex);
                }
                if self.i >= 0 {
                    self.i -= inc_i;
                    self.pos_scr.x += inc_x;
                }
                self.i += dec_i;
                self.pos_scr.y += inc_y;
            }
        }
    }
}

/// Casts line between two tile centers and returns coordinates of tile that is `n`-th distinct
/// intersection of line and tiles that lie beyond and including `from`
/// if going straight from `from` to `to`, where `n` is the `distance`.
pub fn beyond(from: Point, to: Point, distance: u32) -> Point {
    beyond0(from, to, distance, |_| true)
}

// tile_num_beyond()
fn beyond0(from: Point, to: Point, distance: u32, is_in_bounds: impl Fn(Point) -> bool) -> Point {
    if distance == 0 {
        return from;
    }

    ray(from, to)
        .take(distance as usize + 1)
        .take_while(|&p| is_in_bounds(p))
        .last()
        .unwrap()
}

// tile_num()
/// Returns tile coordinates.
pub fn from_screen(p: Point) -> Point {
    let abs_screen_y = p.y;

    // 12 is vertical hex advance
    let mut tile_y = if abs_screen_y >= 0 {
        abs_screen_y / 12
    } else {
        (abs_screen_y + 1) / 12 - 1
    };

    // 16 is horizontal hex advance
    let screen_x_in_tile_hrow = p.x - 16 * tile_y;

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

    match tile_hit_test(Point::new(screen_x_in_tile, screen_y_in_tile)) {
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
pub fn from_screen_rect(rect: Rect) -> Rect {
    super::enclose(rect, from_screen)
}

// tile_coord()
pub fn to_screen(p: Point) -> Point {
    let dx = p.x / 2;
    let mut r = Point::new(48 * dx, 12 * -dx);
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

pub fn center_to_screen(p: Point) -> Point {
    to_screen(p) + TILE_CENTER
}

#[derive(Clone, Copy, Default)]
pub struct View {
    pub origin: Point,
}

impl View {
    pub fn new(origin: Point) -> Self {
        Self {
            origin,
        }
    }
}

impl TileGridView for View {
    fn screen_to_tile(&self, p: Point) -> Point {
        from_screen(p - self.origin)
    }

    fn tile_to_screen(&self, p: Point) -> Point {
        to_screen(p) + self.origin
    }

    fn center_to_screen(&self, p: Point) -> Point {
        center_to_screen(p) + self.origin
    }
}

// tile_dir()
pub fn direction(from: Point, to: Point) -> Direction {
    assert_ne!(from, to);
    let from_scr = to_screen(from);
    let to_scr = to_screen(to);
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

/// Returns smallest number of steps needed to reach tile `p2` from `p1` or `None` if the result
/// would be larger than `max`.
// tile_dist()
pub fn try_distance(mut p1: Point, p2: Point, max: u32) -> Option<u32> {
    let mut distance = 0;
    while p1 != p2 {
        if distance == max {
            return None;
        }
        let dir = direction(p1, p2);
        p1 = go(p1, dir, 1);
        distance += 1;
    }
    Some(distance)
}

pub fn distance(p1: Point, p2: Point) -> u32 {
    try_distance(p1, p2, u32::MAX).unwrap()
}

/// Is `p1` located in front of `p2` if looking in SE direction?
// tile_in_front_of()
pub fn is_in_front_of(p1: Point, p2: Point) -> bool {
    let sp1 = to_screen(p1);
    let sp2 = to_screen(p2);
    sp2.x - sp1.x <= (sp2.y - sp1.y) * -4
}

/// Is `p1` located to right of `p2` if looking in SE direction?
// tile_to_right_of()
pub fn is_to_right_of(p1: Point, p2: Point) -> bool {
    let sp1 = to_screen(p1);
    let sp2_ = to_screen(p2);
    sp1.x - sp2_.x <= (sp1.y - sp2_.y) * 32 / (12 * 2)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TileHit {
    Inside,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn tile_hit_test(p: Point) -> TileHit {
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
    // Width in tiles.
    width: i32,

    // Height in tiles.
    height: i32,
}

impl TileGrid {
    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn go(&self, p: Point, direction: Direction, distance: u32) -> Option<Point> {
        let p = go0(p, direction, distance, |_| true);
        if self.is_in_bounds(p) {
            Some(p)
        } else {
            None
        }
    }

    pub fn go_clipped(&self, p: Point, direction: Direction, distance: u32) -> Point {
        go0(p, direction, distance, |next| self.is_in_bounds(next))
    }

    /// Similar to top-level `beyond()` but also clips the result to grid bounds.
    pub fn beyond(&self, from: Point, to: Point, distance: u32) -> Point {
        assert!(self.is_in_bounds(from));
        beyond0(from, to, distance, |p| self.is_in_bounds(p))
    }

    /// Linear to rectangular coordinates.
    /// Note this is different from original since the `x` axis is not inverted,
    /// see `from_linear_inv()` for the inverted variant.
    pub fn linear_to_rect(&self, num: u32) -> Point {
        Point::new(num as i32 % self.width, num as i32 / self.width)
    }

    /// Rectangular to linear coordinates.
    /// Note this is different from original since the `x` axis is not inverted,
    /// see `to_linear_inv()` for the inverted variant.
    pub fn rect_to_linear(&self, p: Point) -> Option<u32> {
        if self.is_in_bounds(p) {
            Some((self.width * p.y + p.x) as u32)
        } else {
            None
        }
    }

    /// Rectangular to linear coordinates with `x` axis inverted.
    /// This method should be used when converting linears for use in the original assets
    /// (maps, scripts etc).
    pub fn rect_to_linear_inv(&self, p: Point) -> Option<u32> {
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
    pub fn linear_to_rect_inv(&self, num: u32) -> Point {
        let x = self.width - 1 - num as i32 % self.width;
        let y = num as i32 / self.width;
        Point::new(x, y)
    }

    /// Verifies the tile coordinates `p` are within (0, 0, width, height) boundaries.
    pub fn is_in_bounds(&self, p: Point) -> bool {
        p.x >= 0 && p.x < self.width && p.y >= 0 && p.y < self.height
    }

    pub fn clip(&self, p: Point) -> Point {
        Point {
            x: clamp(p.x, 0, self.width - 1),
            y: clamp(p.y, 0, self.height - 1),
        }
    }

    /// Inverts `x` coordinate. 0 becomes `width - 1` and `width - 1` becomes 0.
    pub fn invert_x(&self, x: i32) -> i32 {
        self.width - 1 - x
    }
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            width: 200,
            height: 200,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(non_snake_case)]
    fn P(x: i32, y: i32) -> Point {
        Point::new(x, y)
    }

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
                assert_eq!(tile_hit_test(P(x, y)) as usize, expected[y as usize][x as usize]);
            }
        }
    }

    #[test]
    fn view_from_screen() {
        let t = View::new(P(272, 182));
        assert_eq!(t.screen_to_tile(P(-320, -240)), P(-1, -37));
        assert_eq!(t.screen_to_tile(P(-320, 620)), P(-37, 17));
        assert_eq!(t.screen_to_tile(P(256, -242)), P(17, -28));
    }

    #[test]
    fn from_screen_() {
        let data = &[
            ((-592, -422), (-1, -37)),
            ((-592, 438), (-37, 17)),
            ((-16, -424), (17, -28)),
            ((0, 0), (0, -1)),
            ((16, 0), (0, 0)),
            ((48, 0), (1, 0)),
            ((48, -1), (2, 0)),
            ((0, 4), (0, 0)),
        ];
        for &(inp, exp) in data {
            assert_eq!(from_screen(inp.into()), exp.into());
        }
    }

    #[test]
    fn to_screen_() {
        assert_eq!(to_screen(P(0, 0)), P(0, 0));
        assert_eq!(to_screen(P(97, 63)), P(3344, 180));
    }

    #[test]
    fn view_from_screen2() {
        let mut t = View::default();

        for &o in &[P(0, 0), P(100, 200)] {
            t.origin = o;
            assert_eq!(t.screen_to_tile(o + P(0, 0)), P(0, -1));
            assert_eq!(t.screen_to_tile(o + P(16, 0)), P(0, 0));
            assert_eq!(t.screen_to_tile(o + P(48, 0)), P(1, 0));
            assert_eq!(t.screen_to_tile(o + P(48, -1)), P(2, 0));
            assert_eq!(t.screen_to_tile(o + P(0, 4)), P(0, 0));
        }
    }

    #[test]
    fn view_to_screen1() {
        let t = TileGrid::default();
        let v = View::new(P(272, 182));
        assert_eq!(v.tile_to_screen(t.linear_to_rect_inv(12702)), P(3616, 362));
    }

    #[test]
    fn view_to_screen2() {
        let t = View::default();
        assert_eq!(t.tile_to_screen(P(0, 0)), P(0, 0));
    }

    #[test]
    fn go_() {
        let t = TileGrid::default();
        assert_eq!(go(P(0, 0), Direction::W, 1), P(-1, -1));
        assert_eq!(t.go(P(0, 0), Direction::W, 1), None);
        assert_eq!(t.go_clipped(P(0, 0), Direction::W, 1), P(0, 0));
        assert_eq!(go(P(22, 11), Direction::E, 0), P(22, 11));
        assert_eq!(go(P(22, 11), Direction::E, 1), P(23, 11));
    }

    #[test]
    fn direction_() {
        for dir in Direction::iter() {
            for dist in 1..=10 {
                let from = (100, 100);
                let to = go(from.into(), dir, dist);
                assert_eq!(direction(from.into(), to), dir);
            }
        }

        assert_eq!(direction(P(98, 105), P(111, 92)), Direction::NE);
    }

    #[test]
    fn distance_() {
        assert_eq!(distance(P(1234, -5678), P(1234, -5678)), 0);

        assert_eq!(distance(P(111, 92), P(98, 105)), 19);
        assert_eq!(distance(P(98, 105), P(111, 92)), 19);

        assert_eq!(distance(P(92, 143), P(70, 102)), 52);
        assert_eq!(distance(P(70, 102), P(92, 143)), 52);
    }

    #[test]
    fn try_distance_() {
        assert_eq!(try_distance(P(111, 92), P(98, 105), 19), Some(19));
        assert_eq!(try_distance(P(111, 92), P(98, 105), 18), None);
    }

    #[test]
    fn is_in_front_of_() {
        assert!(is_in_front_of(P(111, 87), P(111, 79)));
        assert!(is_in_front_of(P(100, 100), P(100, 100)));
        assert!(is_in_front_of(P(101, 100), P(100, 100)));
        assert!(is_in_front_of(P(100, 101), P(100, 100)));
        assert!(!is_in_front_of(P(100, 99), P(100, 100)));
    }

    #[test]
    fn is_to_right_of_() {
        assert!(is_to_right_of(P(100, 100), P(100, 100)));
        assert!(is_to_right_of(P(99, 100), P(100, 100)));
        assert!(is_to_right_of(P(100, 99), P(100, 100)));
        assert!(is_to_right_of(P(100, 101), P(100, 100)));
        assert!(is_to_right_of(P(99, 99), P(100, 100)));

        assert!(!is_to_right_of(P(101, 100), P(100, 100)));
        assert!(!is_to_right_of(P(101, 99), P(100, 100)));
        assert!(!is_to_right_of(P(101, 101), P(100, 100)));
    }

    #[test]
    fn beyond_() {
        let data = &[
            (((0, 0), (0, 0), 0), (0, 0)),
            (((123, 456), (123, 456), 0), (123, 456)),
            (((0, 0), (0, 1), 1), (0, 1)),
            (((0, 0), (0, 5), 1), (0, 1)),
            (((0, 0), (0, 5), 5), (0, 5)),
            (((0, -1), (0, 5), 0), (0, -1)),
            (((0, -1), (0, 5), 1), (0, 0)),
            (((0, -1), (0, 5), 6), (0, 5)),
            (((95, 130), (100, 116), 25), (101, 111)),
            (((95, 130), (88, 119), 25), (85, 114)),
        ];

        for &((from, to, distance), exp) in data {
            assert_eq!(beyond(from.into(), to.into(), distance), exp.into());
        }
    }

    #[test]
    fn tg_beyond() {
        let tg = TileGrid::default();

        let data = &[
            (((0, 1), (0, -1), 1), (0, 0)),
            (((0, 1), (0, -1), 2), (0, 0)),
            (((0, 1), (0, -1), 100), (0, 0)),
        ];

        for &((from, to, distance), exp) in data {
            assert_eq!(tg.beyond(from.into(), to.into(), distance), exp.into());
        }
    }
}

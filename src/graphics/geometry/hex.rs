use num_traits::FromPrimitive;
use std::cmp;
use std::f64::consts::PI;

use super::Direction;
use graphics::Point;

const TILE_WIDTH: i32 = 32;
const TILE_HEIGHT: i32 = 16;
const TILE_INNER_HEIGHT: i32 = 8;

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

    // Position in rectangular XY coordinates.
    // Rectangular coordinates span from top to bottom, left to right.
    // Tile with this coordinates will be mapped to screen at screen_pos.
    pos: Point,

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

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn set_pos(&mut self, pos: impl Into<Point>) {
        self.pos = pos.into();
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

    // tile_num_in_direction_()
    pub fn go(&self, p: impl Into<Point>, direction: Direction, distance: u32,
            check_bounds: bool) -> Point {
        // Advance per each direction for even/odd hex.
        static ADVANCE_MAP: [[(i32, i32); Direction::LEN]; 2] = [
            [(1, -1), (1, 0), (0, 1), (-1, 0), (-1, -1), (0, -1)],
            [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (0, -1)],
        ];
        let mut p = p.into();
        for _ in 0..distance {
            let advance = ADVANCE_MAP[p.x as usize % 2][direction as usize].into();
            let next = p + advance;
            if check_bounds && !self.is_in_bounds(next) {
                break;
            }
            p = next;
        }
        p
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
        tile_x += self.pos.x;
        tile_y += self.pos.y;

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

    // tile_coord()
    pub fn to_screen(&self, p: impl Into<Point>) -> Point {
        let p = p.into();
        let mut r = self.screen_pos;
        let dx = (p.x - self.pos.x) / 2;
        r.x += 48 * dx;
        r.y += 12 * -dx;
        if p.x % 2 != 0 {
            if p.x <= self.pos.x {
                r.x -= 16;
                r.y += 12;
            } else {
                r.x += 32;
            }
        }
        let dy = p.y - self.pos.y;
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
            p1 = self.go(p1, dir, 1, false);
            distance += 1;
        }
        distance
    }

    // tile_num_beyond()
    pub fn beyond(&self, from: impl Into<Point>, to: impl Into<Point>, distance: i32) -> Point {
        assert!(distance >= 0);

        let from = from.into();
        if distance == 0 {
            return from;
        }

        let froms = self.to_screen(from).add((16, 18));
        let tos = self.to_screen(to.into()).add((16, 18));

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
                    if cur_distance == distance || !self.is_in_bounds(next) {
                        return cur;
                    }
                    cur = next;
                }
                if j >= 0 {
                    j -= abs_delta_x_mult_2;
                    curs.y += y_inc;
                }
                j += abs_delta_y_mult_2;
                curs.y += x_inc;
            }
        }

        let mut j = abs_delta_x_mult_2 - abs_delta_y_mult_2 / 2;
        loop {
            let next = self.from_screen(curs);
            if next != cur {
                cur_distance += 1;
                if cur_distance == distance || !self.is_in_bounds(next) {
                    return cur;
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
    pub fn from_linear(&self, num: i32) -> Point {
        Point::new(num % self.width, num / self.width)
    }

    /// Rectangular to linear coordinates.
    /// Note this is different from original since the `x` axis is not inverted,
    /// see `to_linear_inv()` for the inverted variant.
    pub fn to_linear(&self, p: impl Into<Point>) -> Option<i32> {
        let p = p.into();
        if self.is_in_bounds(p) {
            Some(self.width * p.y + p.x)
        } else {
            None
        }
    }

    /// Rectangular to linear coordinates with `x` axis inverted.
    /// This method should be used when converting linears for use in the original assets
    /// (maps, scripts etc).
    pub fn to_linear_inv(&self, p: impl Into<Point>) -> Option<i32> {
        let p = p.into();
        if self.is_in_bounds(p) {
            let x = self.width - 1 - p.x;
            Some(self.width * p.y + x)
        } else {
            None
        }
    }

    /// Linear to rectangular coordinates with `x` axis inverted.
    /// This method should be used when converting linears for use in the original assets
    /// (maps, scripts etc).
    pub fn from_linear_inv(&self, num: i32) -> Point {
        let x = self.width - 1 - num % self.width;
        let y = num / self.width;
        Point::new(x, y)
    }

    /// Verifies the tile coordinates `p` are within (0, 0, width, height) boundaries.
    pub fn is_in_bounds(&self, p: impl Into<Point>) -> bool {
        let p = p.into();
        p.x >= 0 && p.x < self.width && p.y >= 0 && p.y < self.height
    }
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            screen_pos: Point::default(),
            pos: Point::default(),
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
            pos: Point::new(98, 100),
            .. Default::default()
        };
        assert_eq!(t.from_screen((-320, -240)), t.from_linear_inv(12702));
        assert_eq!(t.from_screen((-320, 620)), t.from_linear_inv(23538));
        assert_eq!(t.from_screen((256, -242)), t.from_linear_inv(14484));
    }

    #[test]
    fn from_screen2() {
        let mut t = TileGrid::default();

        for tpos in &[Point::new(0, 0), Point::new(30, 50)] {
            t.set_pos(tpos);
            for spos in &[Point::new(0, 0), Point::new(100, 200)] {
                t.set_screen_pos(spos);
                assert_eq!(t.from_screen(spos.add((0, 0))), tpos.add((0, -1)));
                assert_eq!(t.from_screen(spos.add((16, 0))), tpos.add((0, 0)));
                assert_eq!(t.from_screen(spos.add((48, 0))), tpos.add((1, 0)));
                assert_eq!(t.from_screen(spos.add((48, -1))), tpos.add((2, 0)));
                assert_eq!(t.from_screen(spos.add((0, 4))), tpos.add((0, 0)));
            }
        }
    }

    #[test]
    fn to_screen1() {
        let mut t = TileGrid {
            screen_pos: Point::new(272, 182),
            pos: Point::new(98, 100),
            .. Default::default()
        };

        assert_eq!(t.to_screen(t.from_linear_inv(12702)), Point::new(-336, -250));

        t.set_pos((96, 100));
        assert_eq!(t.to_screen(t.from_linear_inv(20704)), Point::new(304, 230));
    }

    #[test]
    fn to_screen2() {
        let t = TileGrid::default();
        assert_eq!(t.to_screen((0, 0)), Point::new(0, 0));
    }

    #[test]
    fn go() {
        let t = TileGrid::default();
        assert_eq!(t.go((0, 0), Direction::W, 1, false), Point::new(-1, -1));
        assert_eq!(t.go((0, 0), Direction::W, 1, true), Point::new(0, 0));
        assert_eq!(t.go((22, 11), Direction::E, 0, false), Point::new(22, 11));
        assert_eq!(t.go((22, 11), Direction::E, 1, false), Point::new(23, 11));
    }

    #[test]
    fn direction() {
        let t = TileGrid::default();

        for &dir in Direction::values() {
            for dist in 1..=10 {
                let from = (100, 100);
                let to = t.go(from, dir, dist, false);
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
}
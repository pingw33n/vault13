use std::cmp;

use graphics::{Point, Rect};

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

    pub fn set_pos_lin(&mut self, num: i32) {
        let lin = self.from_linear(num);
        self.set_pos(lin);
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    // square_xy()
    pub fn from_screen(&self, p: impl Into<Point>) -> Point {
        let p = p.into();
        let abs_x = p.x - self.screen_pos.x;
        let abs_y = p.y - self.screen_pos.y - 12;

        let dx = 3 * abs_x - 4 * abs_y;
        let square_x = self.pos.x + if dx >= 0  {
            dx / 192
        } else {
            (dx + 1) / 192 - 1
        };

        let dy = 4 * abs_y + abs_x;
        let square_y = self.pos.y + if dy >= 0 {
            dy / 128
        } else {
            ((dy + 1) / 128) - 1
        };

        Point::new(square_x, square_y)
    }

    // square_coord_()
    pub fn to_screen(&self, p: impl Into<Point>) -> Point {
        let p = p.into();
        let x = p.x - self.pos.x;
        let y = p.y - self.pos.y;
        let screen_x = self.screen_pos.x + 48 * x + 32 * y;
        let screen_y = self.screen_pos.y - 12 * x + 24 * y;
        Point::new(screen_x, screen_y)
    }

    /// Returns minimal rectangle in local coordinates that encloses the specified screen `rect`.
    /// Clips the resulting rectangle if `clip` is `true`.
    pub fn from_screen_rect(&self, rect: &Rect, clip: bool) -> Rect {
        let right = rect.right - 1;
        let bottom = rect.bottom - 1;

        let x = self.from_screen((rect.left, bottom)).x;
        let y = self.from_screen((rect.left, rect.top)).y;
        let top_left = if clip {
            self.clip((x, y))
        } else {
            Point::new(x, y)
        };

        let x = self.from_screen((right, rect.top)).x;
        let y = self.from_screen((right, bottom)).y;
        let bottom_right_incl = if clip {
            self.clip((x, y))
        } else {
            Point::new(x, y)
        };

        Rect::with_points(top_left, bottom_right_incl + Point::new(1, 1))
    }

    /// Rectangular to linear coordinates.
    pub fn to_linear(&self, p: impl Into<Point>) -> Option<i32> {
        let p = p.into();
        if self.is_in_bounds(p) {
            let x = self.width - 1 - p.x;
            Some(self.width * p.y + x)
        } else {
            None
        }
    }

    /// Linear to rectangular coordinates.
    pub fn from_linear(&self, num: i32) -> Point {
        let x = self.width - 1 - num % self.width;
        let y = num / self.width;
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
            x: cmp::min(cmp::max(p.x, 0), self.width - 1),
            y: cmp::min(cmp::max(p.y, 0), self.height - 1),
        }
    }
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            screen_pos: Point::default(),
            pos: Point::default(),
            width: 100,
            height: 100,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_screen1() {
        let t = TileGrid {
            screen_pos: Point::new(0xf0, 0xa8),
            pos: Point::new(0x1a, 0x3e),
            .. Default::default()
        };
        let square_xy = |x, y| {
            let p = t.from_screen((x, y));
            Point::new(t.width - 1 - p.x, p.y)
        };
        assert_eq!(square_xy(0, 0), Point::new(0x49, 0x36));
        assert_eq!(square_xy(0x27f, 0x17b), Point::new(0x47, 0x47));
    }

    #[test]
    fn from_screen2() {
        let t = TileGrid::default();
        assert_eq!(t.from_screen((0, 0)), Point::new(0, -1));
        assert_eq!(t.from_screen((0, 12)), Point::new(0, 0));
        assert_eq!(t.from_screen((0, 13)), Point::new(-1, 0));
        assert_eq!(t.from_screen((79, 0)), Point::new(1, 0));
        assert_eq!(t.from_screen((79, 25)), Point::new(0, 1));
    }

    #[test]
    fn to_screen1() {
        let t = TileGrid {
            screen_pos: Point::new(0x100, 0xb4),
            pos: Point::new(0x31, 0x32),
            .. Default::default()
        };
        assert_eq!(t.to_screen(t.from_linear(0x1091)), Point::new(0x1b0, -120));
    }

    #[test]
    fn to_screen2() {
        let t = TileGrid::default();
        assert_eq!(t.to_screen((0, 0)), Point::new(0, 0));
        assert_eq!(t.to_screen((1, 0)), Point::new(48, -12));
        assert_eq!(t.to_screen((0, 1)), Point::new(32, 24));
        assert_eq!(t.to_screen((0, -1)), Point::new(-32, -24));
        assert_eq!(t.to_screen((-1, 0)), Point::new(-48, 12));
    }
}
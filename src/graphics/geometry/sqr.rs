use num_traits::clamp;

use crate::graphics::Point;
use crate::graphics::geometry::TileGridView;

pub const TILE_WIDTH: i32 = 80;
pub const TILE_HEIGHT: i32 = 36;
pub const TILE_CENTER: Point = Point::new(TILE_WIDTH / 2, TILE_HEIGHT / 2);

// square_xy()
pub fn from_screen(p: Point) -> Point {
    let x = p.x;
    let y = p.y - 12;

    let dx = 3 * x - 4 * y;
    let square_x = if dx >= 0  {
        dx / 192
    } else {
        (dx + 1) / 192 - 1
    };

    let dy = 4 * y + x;
    let square_y = if dy >= 0 {
        dy / 128
    } else {
        ((dy + 1) / 128) - 1
    };

    Point::new(square_x, square_y)
}

// square_coord_()
pub fn to_screen(p: Point) -> Point {
    let screen_x = 48 * p.x + 32 * p.y;
    let screen_y = -12 * p.x + 24 * p.y;
    Point::new(screen_x, screen_y)
}

pub fn center_to_screen(p: Point) -> Point {
    p + TILE_CENTER
}

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
            width: 100,
            height: 100,
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
    fn view_from_screen() {
        let t = View::new(P(0xf0, 0xa8));
        let square_xy = |x, y| {
            let p = t.screen_to_tile(P(x, y));
            P(100 - 1 - p.x, p.y)
        };
        assert_eq!(square_xy(0, 0), P(99, -8));
        assert_eq!(square_xy(0x27f, 0x17b), P(97, 9));
    }

    #[test]
    fn from_screen_() {
        assert_eq!(from_screen(P(0, 0)), P(0, -1));
        assert_eq!(from_screen(P(0, 12)), P(0, 0));
        assert_eq!(from_screen(P(0, 13)), P(-1, 0));
        assert_eq!(from_screen(P(79, 0)), P(1, 0));
        assert_eq!(from_screen(P(79, 25)), P(0, 1));
    }

    #[test]
    fn view_to_screen() {
        let t = TileGrid::default();
        let v = View::new(P(0x100, 0xb4));
        assert_eq!(v.tile_to_screen(t.linear_to_rect_inv(0x1091)), P(4384, 492));
    }

    #[test]
    fn to_screen_() {
        assert_eq!(to_screen(P(0, 0)), P(0, 0));
        assert_eq!(to_screen(P(1, 0)), P(48, -12));
        assert_eq!(to_screen(P(0, 1)), P(32, 24));
        assert_eq!(to_screen(P(0, -1)), P(-32, -24));
        assert_eq!(to_screen(P(-1, 0)), P(-48, 12));
    }
}

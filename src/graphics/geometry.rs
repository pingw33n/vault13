pub mod camera;
pub mod hex;
pub mod sqr;

use crate::graphics::{Point, Rect};

/// Provides mapping between tile and screen coordinates.
pub trait TileGridView {
    /// Converts screen coordinates to tile coordinates.
    fn screen_to_tile(&self, p: Point) -> Point;

    /// Converts tile coordinates to screen coordinates.
    fn tile_to_screen(&self, p: Point) -> Point;

    /// Converts tile coordinates with origin at the tile center, to screen coordinates.
    fn center_to_screen(&self, p: Point) -> Point;

    /// Returns minimal rectangle in tile coordinates that encloses the specified screen `rect`.
    fn enclose(&self, rect: Rect) -> Rect {
        enclose(rect, |p| self.screen_to_tile(p))
    }
}

fn enclose(rect: Rect, from_screen: impl Fn(Point) -> Point) -> Rect {
    let right = rect.right - 1;
    let bottom = rect.bottom - 1;

    let x = from_screen(Point::new(rect.left, bottom)).x;
    let y = from_screen(Point::new(rect.left, rect.top)).y;
    let top_left = Point::new(x, y);

    let x = from_screen(Point::new(right, rect.top)).x;
    let y = from_screen(Point::new(right, bottom)).y;
    let bottom_right_incl = Point::new(x, y);

    Rect::with_points(top_left, bottom_right_incl + Point::new(1, 1))
}

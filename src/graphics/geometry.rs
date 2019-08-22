pub mod hex;
pub mod map;
pub mod sqr;

use crate::graphics::{Point, Rect};

/// Provides mapping between tile and screen coordinates.
pub trait TileGridView {
    /// Converts screen coordinates to tile coordinates.
    fn from_screen(&self, p: impl Into<Point>) -> Point;

    /// Converts tile coordinates to screen coordinates.
    fn to_screen(&self, p: impl Into<Point>) -> Point;

    /// Returns minimal rectangle in tile coordinates that encloses the specified screen `rect`.
    fn from_screen_rect(&self, rect: &Rect) -> Rect {
        from_screen_rect(rect, |p| self.from_screen(p))
    }
}

fn from_screen_rect(rect: &Rect, from_screen: impl Fn(Point) -> Point) -> Rect {
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
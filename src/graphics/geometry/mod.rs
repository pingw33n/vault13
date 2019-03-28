pub mod hex;
pub mod map;
pub mod sqr;

use crate::graphics::{Point, Rect};

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
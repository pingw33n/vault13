use num_traits::clamp;
use std::cmp;
use std::ops;

pub mod color;
pub mod font;
pub mod geometry;
pub mod lighting;
pub mod map;
pub mod render;
pub mod sprite;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
        }
    }

    pub fn add(self, p: impl Into<Self>) -> Self {
        let p = p.into();
        Self::new(self.x + p.x, self.y + p.y)
    }

    pub fn add_assign(&mut self, p: impl Into<Self>) {
        let p = p.into();
        self.x += p.x;
        self.y += p.y;
    }

    pub fn sub_assign(&mut self, p: impl Into<Self>) {
        let p = p.into();
        self.x -= p.x;
        self.y -= p.y;
    }

    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    pub fn tuple(self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn elevated(self, elevation: u32) -> EPoint {
        EPoint {
            elevation,
            point: self,
        }
    }

    pub fn clamp_in_rect(self, rect: &Rect) -> Self {
        Self::new(
            clamp(self.x, rect.left, rect.right - 1),
            clamp(self.y, rect.top, rect.bottom - 1))
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl ops::Add for Point {
    type Output = Self;

    fn add(self, o: Self) -> Self {
        Point::add(self, o)
    }
}

impl ops::AddAssign for Point {
    fn add_assign(&mut self, o: Self) {
        Point::add_assign(self, o)
    }
}

impl ops::Neg for Point {
    type Output = Point;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl ops::Sub for Point {
    type Output = Self;

    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl ops::SubAssign for Point {
    fn sub_assign(&mut self, o: Self) {
        Point::sub_assign(self, o)
    }
}

impl<'a> From<&'a Point> for Point {
    fn from(v: &'a Point) -> Self {
        *v
    }
}

impl From<(i32, i32)> for Point {
    fn from(v: (i32, i32)) -> Self {
        Self::new(v.0, v.1)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EPoint {
    pub elevation: u32,
    pub point: Point,
}

impl EPoint {
    pub fn new(elevation: u32, point: impl Into<Point>) -> Self {
        Self {
            elevation,
            point: point.into(),
        }
    }

    pub fn with_point(self, point: impl Into<Point>) -> Self {
        Self::new(self.elevation, point)
    }
}

impl<'a> From<&'a EPoint> for EPoint {
    fn from(v: &'a EPoint) -> Self {
        *v
    }
}

impl From<(u32, Point)> for EPoint {
    fn from(v: (u32, Point)) -> Self {
        Self::new(v.0, v.1)
    }
}

impl From<(u32, (i32, i32))> for EPoint {
    fn from(v: (u32, (i32, i32))) -> Self {
        Self::new(v.0, Point::from(v.1))
    }
}

impl Default for EPoint {
    fn default() -> Self {
        Self {
            elevation: 0,
            point: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    pub fn full() -> Self {
        Self {
            left: i32::min_value(),
            top: i32::min_value(),
            right: i32::max_value(),
            bottom: i32::max_value(),
        }
    }

    pub fn with_size(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    pub fn with_points(top_left: impl Into<Point>, bottom_right: impl Into<Point>) -> Self {
        let top_left = top_left.into();
        let bottom_right = bottom_right.into();
        Self {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let left = cmp::max(self.left, other.left);
        let top = cmp::max(self.top, other.top);
        let right = cmp::min(self.right, other.right);
        let bottom = cmp::min(self.bottom, other.bottom);
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn translate(&self, x: i32, y: i32) -> Self {
        Self {
            left: self.left + x,
            top: self.top + y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.left >= self.right &&
            self.top >= self.bottom
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right &&
            y >= self.top && y < self.bottom
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.left < other.right &&
            self.right > other.left &&
            self.top < other.bottom &&
            self.bottom > other.top
    }

    pub fn top_left(&self) -> Point {
        Point::new(self.left, self.top)
    }

    pub fn bottom_right(&self) -> Point {
        Point::new(self.bottom, self.right)
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn center(&self) -> Point {
        Point::new(self.left + self.width() / 2, self.top + self.height() / 2)
    }
}


use num_traits::clamp;
use std::cmp;
use std::ops;
use std::ops::MulAssign;

pub mod color;
pub mod font;
pub mod geometry;
pub mod lighting;
pub mod map;
pub mod render;
pub mod sprite;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
        }
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

    pub fn clamp_in_rect(self, rect: Rect) -> Self {
        Self::new(
            clamp(self.x, rect.left, rect.right - 1),
            clamp(self.y, rect.top, rect.bottom - 1))
    }
}

impl ops::Add for Point {
    type Output = Self;

    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl ops::AddAssign for Point {
    fn add_assign(&mut self, o: Self) {
        self.x += o.x;
        self.y += o.y;
    }
}

impl ops::Div<i32> for Point {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl ops::DivAssign<i32> for Point {
    fn div_assign(&mut self, rhs: i32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Mul<i32> for Point {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<i32> for Point {
    fn mul_assign(&mut self, rhs: i32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl ops::Neg for Point {
    type Output = Self;

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
        self.x -= o.x;
        self.y -= o.y;
    }
}

impl std::iter::Sum for Point {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        iter.fold(Point::new(0, 0), ops::Add::add)
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct EPoint {
    pub elevation: u32,
    pub point: Point,
}

impl EPoint {
    pub fn new(elevation: u32, point: Point) -> Self {
        Self {
            elevation,
            point,
        }
    }

    pub fn with_point(self, point: Point) -> Self {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

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
            left: i32::MIN,
            top: i32::MIN,
            right: i32::MAX,
            bottom: i32::MAX,
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

    pub fn with_points(top_left: Point, bottom_right: Point) -> Self {
        Self {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        }
    }

    pub fn intersect(&self, other: Self) -> Self {
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

    pub fn translate(&self, offset: Point) -> Self {
        Self {
            left: self.left + offset.x,
            top: self.top + offset.y,
            right: self.right + offset.x,
            bottom: self.bottom + offset.y,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.left >= self.right &&
            self.top >= self.bottom
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left && p.x < self.right &&
            p.y >= self.top && p.y < self.bottom
    }

    pub fn contains_rect(&self, other: Self) -> bool {
        self.intersect(other) == other
    }

    pub fn intersects(&self, other: Self) -> bool {
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

    pub fn with_width(mut self, width: i32) -> Self {
        self.right = self.left + width;
        self
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn with_height(mut self, height: i32) -> Self {
        self.bottom = self.top + height;
        self
    }

    pub fn center(&self) -> Point {
        Point::new(self.left + self.width() / 2, self.top + self.height() / 2)
    }
}


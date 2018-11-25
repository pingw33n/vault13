use slotmap::{DefaultKey, SlotMap};
use std::cell::RefCell;
use std::cmp;
use std::ops;
use std::rc::Rc;

pub mod color;
pub mod frm;
pub mod geometry;
pub mod lightmap;
pub mod map;
pub mod render;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

//
//    pub fn add_xy(self, x: i32, y: i32) -> Self {
//        Self::new(self.x + x, self.y + y)
//    }
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

impl ops::Sub for Point {
    type Output = Self;

    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
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
        self.left < self.right &&
            self.top < self.bottom
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right &&
            y >= self.top && y < self.bottom
    }
}


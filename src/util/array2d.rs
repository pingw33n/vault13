use std::ops::{Deref, DerefMut};

use crate::util::VecExt;

pub struct Array2d<T> {
    arr: Box<[T]>,
    width: usize,
}

impl<T> Array2d<T> {
    pub fn new(arr: Box<[T]>, width: usize) -> Self {
        assert!(width > 0);
        assert_eq!(arr.len() % width, 0);
        Self {
            arr,
            width,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.arr.len() / self.width
    }

    pub fn len(&self) -> usize {
        self.arr.len()
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        self.arr.get(self.lin(x, y))
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let i = self.lin(x, y);
        self.arr.get_mut(i)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.arr
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        &mut self.arr
    }

    fn lin(&self, x: usize, y: usize) -> usize {
        y.checked_mul(self.width).expect("y overflow")
            .checked_add(x).expect("x overflow")
    }
}

impl<T: Default> Array2d<T> {
    pub fn with_default(width: usize, height: usize) -> Self {
        Self::new(Vec::with_default(width * height).into_boxed_slice(), width)
    }
}

impl<T> Deref for Array2d<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.arr
    }
}

impl<T> DerefMut for Array2d<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.arr
    }
}
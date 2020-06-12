pub mod array2d;
pub mod random;
#[cfg(test)]
pub mod test;

use bstring::{bstr, BString};
use enum_map::Enum;
use std::fmt;
use std::marker::PhantomData;
use std::ops::RangeBounds;

pub trait VecExt<T> {
    fn with_default(len: usize) -> Vec<T>
        where T: Default
    {
        Self::from_fn(len, |_| T::default())
    }

    fn from_fn(len: usize, f: impl Fn(usize) -> T) -> Vec<T> {
        let mut r = Vec::with_capacity(len);
        for i in 0..len {
            r.push(f(i));
        }
        r
    }

    fn remove_first(&mut self, item: &T) -> Option<T>
        where T: PartialEq<T>;
}

impl<T> VecExt<T> for Vec<T> {
    fn remove_first(&mut self, item: &T) -> Option<T>
        where T: PartialEq<T>
    {
        self.iter().position(|v| v == item)
            .map(|i| self.remove(i))
    }
}

pub fn enum_iter<T: Enum<()> + Copy, R: RangeBounds<T>>(r: R) -> EnumIter<T> {
    use std::ops::Bound;
    let i = match r.start_bound() {
        Bound::Included(b) => b.to_usize(),
        Bound::Excluded(_) => unreachable!(),
        Bound::Unbounded => 0,
    };
    let end = match r.end_bound() {
        Bound::Included(b) => b.to_usize().checked_add(1).unwrap(),
        Bound::Excluded(b) => b.to_usize(),
        Bound::Unbounded => T::POSSIBLE_VALUES,
    };
    if i > end {
        panic!("slice index starts at ordinal {} but ends at ordinal {}", i, end);
    } else if end > T::POSSIBLE_VALUES {
        panic!("ordinal {} out of range for enum of length {}", i, T::POSSIBLE_VALUES);
    }
    EnumIter::new(i, end)
}

pub trait EnumExt: Enum<()> + Copy {
    fn len() -> usize {
        Self::POSSIBLE_VALUES
    }

    fn iter() -> EnumIter<Self> {
        enum_iter(..)
    }

    fn from_ordinal(v: usize) -> Self {
        Enum::from_usize(v)
    }

    fn try_from_ordinal(v: usize) -> Option<Self> {
        if v < Self::len() {
            Some(Self::from_ordinal(v))
        } else {
            None
        }
    }

    fn ordinal(self) -> usize {
        Enum::to_usize(self)
    }
}

impl<T: Enum<()> + Copy> EnumExt for T {}

pub struct EnumIter<T> {
    i: usize,
    end: usize,
    _t: PhantomData<T>,
}

impl<T: Enum<()>> EnumIter<T> {
    fn new(i: usize, end: usize) -> Self {
        Self {
            i,
            end,
            _t: PhantomData,
        }
    }

    fn empty() -> Self {
        Self::new(0, 0)
    }
}

impl<T: Enum<()>> Iterator for EnumIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i == self.end {
            return None;
        }
        let r = T::from_usize(self.i);
        self.i += 1;
        Some(r)
    }
}

#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SmKey(slotmap::KeyData);

impl From<slotmap::KeyData> for SmKey {
    fn from(k: slotmap::KeyData) -> Self {
        Self(k)
    }
}

impl From<SmKey> for slotmap::KeyData {
    fn from(k: SmKey) -> Self {
        k.0
    }
}

impl slotmap::Key for SmKey { }

impl fmt::Debug for SmKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = self.0.as_ffi();
        let ver = (v >> 32) as u32;
        let idx = v as u32;
        write!(f, "{}:{}", idx, ver)
    }
}

pub fn sprintf(fmt: &bstr, args: &[&bstr]) -> BString {
    let mut r = BString::with_capacity(fmt.len());
    let mut args = args.iter();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        match c {
            b'%' => {
                i += 1;
                if i >= fmt.len() {
                    panic!("truncated format spec");
                }
                let c = fmt[i];
                match c {
                    b's' => r.push_str(args.next().expect("no more args")),
                    b'%' => r.push(b'%'),
                    _ => panic!("unsupported format spec: {}", c as char),
                }
            }
            _ => r.push(c),
        }
        i += 1;
    }
    assert!(args.next().is_none(), "too many args");
    r
}

#[cfg(test)]
mod test_ {
    use super::*;

    #[test]
    fn sprintf_() {
        let f = sprintf;
        fn bs(s: &str) -> BString {
            s.into()
        }

        assert_eq!(f("".into(), &[]), bs(""));
        assert_eq!(f("no args".into(), &[]), bs("no args"));
        assert_eq!(f("%s one arg".into(), &["arg1".into()]), bs("arg1 one arg"));
        assert_eq!(f("one arg %s".into(), &["arg1".into()]), bs("one arg arg1"));
        assert_eq!(f("%s two args %s".into(), &["arg1".into(), "arg2".into()]),
            bs("arg1 two args arg2"));
        assert_eq!(f("%%s escape %%".into(), &[]), bs("%s escape %"));
    }
}
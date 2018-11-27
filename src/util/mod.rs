use enum_map::Enum;
use std::cmp;
use std::io;
use std::marker::PhantomData;
use std::ops::RangeBounds;

#[derive(Debug)]
pub struct Limited<T> {
    inner: T,
    pos: u64,
    limit: u64,
}

impl<T> Limited<T> {
    pub fn new(inner: T, limit: u64) -> Limited<T> {
        Limited {
            inner: inner,
            pos: 0,
            limit: limit,
        }
    }

    pub fn pos(&self) -> u64 {
        self.pos
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

fn read_limited(buf: &mut [u8], pos: u64, limit: u64, reader: &mut io::Read) -> io::Result<usize> {
    let can_read = cmp::min(buf.len() as u64, limit - pos);
    if can_read != 0 {
        reader.read(&mut buf[..(can_read as usize)])
    } else {
        Ok(0)
    }
}

impl<T: io::Read> io::Read for Limited<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = try!(read_limited(buf, self.pos, self.limit, self.get_mut()));
        self.pos += read as u64;
        Ok(read)
    }
}

impl<T: io::Write> io::Write for Limited<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let can_write = cmp::min(buf.len() as u64, self.limit - self.pos);
        if can_write != 0 {
            match self.inner.write(&buf[..(can_write as usize)]) {
                Ok(written) => {
                    self.pos += written as u64;
                    Ok(written)
                }
                e @ _ => e,
            }
        } else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub fn vec_with_default<T: Default>(len: usize) -> Vec<T> {
    let mut r = Vec::with_capacity(len);
    for _ in 0..len {
        r.push(T::default());
    }
    r
}

//pub fn ensure_trailing(s: &str, chars: &str) -> String {
//    assert!(!chars.is_empty());
//    let mut ends_with_any = false;
//    for c in chars.chars() {
//        if s.ends_with(c) {
//            ends_with_any = true;
//            break;
//        }
//    }
//    if ends_with_any {
//        s.to_owned()
//    } else {
//        let mut r = String::with_capacity(s.len() + 1);
//        r.push_str(s);
//        r.push(chars.chars().next().unwrap());
//        r
//    }
//}

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

#[cfg(test)]
mod tests {
    mod limited {
        use std::io::prelude::*;
        use std::io::Cursor;

        use util::Limited;

        #[test]
        fn limits_reads() {
            let mut l = Limited::new(Cursor::new(vec![1, 2, 3]), 1);

            assert_eq!(l.pos(), 0);
            assert_eq!(l.limit(), 1);
            let mut buf = [42; 3];
            assert_eq!(l.read(&mut buf).unwrap(), 1);
            assert_eq!(buf, [1, 42, 42]);
            assert_eq!(l.pos(), 1);
            assert_eq!(l.get_mut().position(), 1);

            buf = [42; 3];
            assert_eq!(l.read(&mut buf).unwrap(), 0);
            assert_eq!(buf, [42, 42, 42]);
            assert_eq!(l.pos(), 1);
            assert_eq!(l.into_inner().position(), 1);
        }

        #[test]
        fn limits_writes() {
            let mut l = Limited::new(Cursor::new(vec![]), 1);

            assert_eq!(l.pos(), 0);
            assert_eq!(l.limit(), 1);
            let buf = [41, 42, 43];
            assert_eq!(l.write(&buf).unwrap(), 1);
            assert_eq!(&l.get_ref().get_ref()[..], [41]);
            assert_eq!(l.pos(), 1);
            assert_eq!(l.get_mut().position(), 1);

            assert_eq!(l.write(&buf).unwrap(), 0);
            assert_eq!(&l.get_ref().get_ref()[..], [41]);
            assert_eq!(l.pos(), 1);
            assert_eq!(l.into_inner().position(), 1);
        }
    }
}

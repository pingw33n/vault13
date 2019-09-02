use std::cmp;
use std::io::Result;
use std::io::prelude::*;

#[derive(Debug)]
pub struct Limited<T> {
    inner: T,
    pos: u64,
    limit: u64,
}

impl<T> Limited<T> {
    pub fn new(inner: T, limit: u64) -> Limited<T> {
        Limited {
            inner,
            pos: 0,
            limit,
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

fn read_limited(buf: &mut [u8], pos: u64, limit: u64, reader: &mut Read) -> Result<usize> {
    let can_read = cmp::min(buf.len() as u64, limit - pos);
    if can_read != 0 {
        reader.read(&mut buf[..(can_read as usize)])
    } else {
        Ok(0)
    }
}

impl<T: Read> Read for Limited<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read = read_limited(buf, self.pos, self.limit, self.get_mut())?;
        self.pos += read as u64;
        Ok(read)
    }
}

impl<T: Write> Write for Limited<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
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

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::cmp;
use std::io::prelude::*;
use std::io::{self, Cursor, Error, ErrorKind, Result};

pub struct LzssDecoder<R> {
    reader: R,
    buf: Cursor<Vec<u8>>,
    buf_avail: usize,
    written: u64,
    expected_output_size: u64,
    state: State,
}

enum State {
    Ok,
    Done,
    Err,
}

impl<R: Read> LzssDecoder<R> {
    pub fn new(reader: R, expected_output_size: u64) -> Self {
        LzssDecoder {
            reader,
            buf: Cursor::new(Vec::with_capacity(16 * 1024)),
            buf_avail: 0,
            written: 0,
            expected_output_size,
            state: State::Ok,
        }
    }

    fn fill_buf(&mut self) -> Result<usize> {
        match self.state {
            State::Ok => {}
            State::Done if self.buf_avail == 0 => return Ok(0),
            State::Done => {}
            State::Err => return Err(Error::new(ErrorKind::InvalidData, "Malformed LZSS stream")),
        }
        if self.buf_avail == 0 {
            self.buf.set_position(0);
            let block_written = lzss_decode_block(&mut self.reader, &mut self.buf)?;
            self.buf_avail = block_written as usize;
            self.written += block_written;
            if self.written == self.expected_output_size {
                self.state = State::Done;
            } else if block_written == 0 || self.written > self.expected_output_size {
                self.state = State::Err;
            }
            if let State::Err = self.state {
                return Err(Error::new(ErrorKind::InvalidData, "Malformed LZSS stream"));
            }
            self.buf.set_position(0);
        }
        Ok(self.buf_avail)
    }
}

impl<R: Read> Read for LzssDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut buf_written = 0;
        while buf_written < buf.len() && self.fill_buf()? != 0 {
            let to_write = cmp::min(self.buf_avail, buf.len() - buf_written);
            buf_written += self.buf.read(&mut buf[buf_written..(buf_written + to_write)])?;
            self.buf_avail -= to_write;
        }
        Ok(buf_written)
    }
}

pub fn lzss_decode_block(inp: &mut impl Read, out: &mut impl Write) -> Result<u64> {
    let block_descr = inp.read_i16::<BigEndian>()? as i32;
    if block_descr == 0 {
        return Ok(0);
    }

    let block_size = u64::from(block_descr.unsigned_abs());
    let block_written;
    if block_descr < 0 {
        block_written = io::copy(inp, out)?;
        if block_written != block_size {
            return Err(Error::new(ErrorKind::InvalidData, "Malformed LZSS stream"));
        }
    } else { // block_descr > 0
        block_written = lzss_decode_block_content(inp, block_size, out)?;
    }

    Ok(block_written)
}

fn lzss_decode_block_content(
    inp: &mut impl Read,
    block_size: u64,
    out: &mut impl Write,
) -> Result<u64> {
    const N: usize = 4096;
    const F: usize = 18;
    const THRESHOLD: usize = 2;

    let mut text_buf = [0x20; N + F - 1];
    let mut r = N - F;
    let mut flags = 0i32;

    let mut block_read = 0u64;
    let mut block_written = 0u64;

    loop {
        flags >>= 1;
        if flags & 0x100 == 0 {
            if block_read >= block_size {
                break;
            }
            let b = inp.read_u8()? as i32;
            block_read += 1;

            if block_read >= block_size {
                break;
            }

            flags = b | 0xff00;
        }

        if (flags & 1) != 0 {
            let b = inp.read_u8()?;
            block_read += 1;

            out.write_u8(b)?;
            block_written += 1;

            if block_read >= block_size {
                break;
            }

            text_buf[r] = b;
            r = (r + 1) & (N - 1);
        } else {
            if block_read >= block_size {
                break;
            }

            let mut i = inp.read_u8()? as usize;
            block_read += 1;

            if block_read >= block_size {
                break;
            }

            let mut j = inp.read_u8()? as usize;
            block_read += 1;

            i |= (j & 0xf0) << 4;
            j = (j & 0x0f) + THRESHOLD;

            for k in 0..=j {
                let b = text_buf[(i + k) & (N - 1)];

                out.write_u8(b)?;
                block_written += 1;

                text_buf[r] = b;
                r = (r + 1) & (N - 1);
            }
        }
    }

    Ok(block_written)
}

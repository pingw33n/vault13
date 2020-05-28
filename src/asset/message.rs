use bstring::BString;
use byteorder::ReadBytesExt;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::str;
use std::collections::HashMap;

use crate::fs::FileSystem;

/// Bullet character used in message panel.
pub const BULLET: u8 = b'\x95';
pub const BULLET_STR: &[u8] = b"\x95";

pub type MessageId = i32;

#[derive(Debug, Default)]
pub struct Messages {
    map: HashMap<MessageId, Message>,
}

impl Messages {
    pub fn read(rd: &mut impl Read) -> io::Result<Self> {
        let mut map = HashMap::new();
        loop {
            match Message::read(rd) {
                Ok(Some(m)) => map.insert(m.id, m),
                Ok(None) => break,
                Err(e) => return Err(e),
            };
        }
        Ok(Self {
            map,
        })
    }

    pub fn read_file(fs: &FileSystem, language: &str, path: &str) -> io::Result<Self> {
        let path = format!("text/{}/{}", language, path);
        Self::read(&mut fs.reader(&path)?)
    }

    pub fn get(&self, id: MessageId) -> Option<&Message> {
        self.map.get(&id)
    }
}

#[derive(Debug)]
pub struct Message {
    pub id: MessageId,
    pub audio: BString,
    pub text: BString,
}

impl Message {
    fn read(rd: &mut impl Read) -> io::Result<Option<Self>> {
        let id = maybe_read_field(rd)?;
        Ok(if let Some(id) = id {
            let id = id.parse().map_err(|_| Error::new(ErrorKind::InvalidData, "error reading ID field"))?;
            let audio = read_field(rd)?;
            let text = read_field(rd)?;
            Some(Self {
                id,
                audio,
                text,
            })
        } else {
            None
        })
    }
}


fn maybe_read_field(rd: &mut impl Read) -> io::Result<Option<BString>> {
    loop {
        match rd.read_u8() {
            Ok(c) => match c {
                b'{' => break,
                b'}' => return Err(Error::new(ErrorKind::InvalidData, "misplaced delimiter")),
                _ => {}
            }
            Err(e) => if e.kind() == ErrorKind::UnexpectedEof {
                return Ok(None);
            } else {
                return Err(e);
            }
        }
    }
    let mut r = BString::new();
    loop {
        let c = rd.read_u8()?;
        if c == b'}' {
            break;
        }
        if c != b'\n' {
            if r.len() == 1024 {
                return Err(Error::new(ErrorKind::InvalidData, "field too long"));
            }
            r.push(c);
        }
    }
    Ok(Some(r))
}

fn read_field(rd: &mut impl Read) -> io::Result<BString> {
    match maybe_read_field(rd) {
        Ok(Some(v)) => Ok(v),
        Ok(None) => Err(Error::new(ErrorKind::InvalidData, "unexpected eof")),
        Err(e) => Err(e),
    }
}
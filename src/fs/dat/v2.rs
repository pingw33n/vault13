use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Result, SeekFrom};
use std::io::prelude::*;

use std::path::{Path, PathBuf};

use super::super::{Metadata, Provider};
use super::util::{build_normalized_path, normalize_path};

pub fn new_provider<P: AsRef<Path>>(path: P) -> Result<Box<dyn Provider>> {
    Ok(Box::new(Dat::new(path)?))
}

#[derive(Debug)]
struct Dat {
    path: PathBuf,
    files: HashMap<String, DatFile>,
}

#[derive(Debug)]
struct DatFile {
    offset: u32,
    size: u32,
    compressed_size: u32,
}

impl Dat {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut reader = BufReader::new(File::open(path.as_ref())?);

        reader.seek(SeekFrom::End(-8))?;
        let file_list_size = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;

        if path.as_ref().metadata()?.len() != size as u64 {
            return Err(Error::new(ErrorKind::InvalidData,
                                      "Actual file size and in-file size differ"));
        }

        if file_list_size > size - 8 {
            return Err(Error::new(ErrorKind::InvalidData, "File list size is too big"));
        }

        let file_list_offset = size - file_list_size - 8;
        reader.seek(SeekFrom::Start(file_list_offset as u64))?;

        let file_count = reader.read_u32::<LittleEndian>()?;

        let mut files = HashMap::with_capacity(file_count as usize);

        for _ in 0..file_count {
            let path = read_path(&mut reader)?;
            let compressed = (reader.read_u8()? & 1) != 0;
            let size = reader.read_u32::<LittleEndian>()?;
            let compressed_size = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u32::<LittleEndian>()?;

            files.insert(path,
                DatFile {
                    offset,
                    size,
                    compressed_size: if compressed {
                        compressed_size
                    } else {
                        0
                    },
                });
        }

        Ok(Dat {
            path: path.as_ref().to_path_buf(),
            files,
        })
    }

    fn file(&self, path: &str) -> Result<&DatFile> {
        self.files.get(&normalize_path(path))
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "file not found"))
    }
}

impl DatFile {
    fn is_compressed(&self) -> bool {
        self.compressed_size != 0
    }
}

impl Provider for Dat {
    fn reader(&self, path: &str) -> Result<Box<dyn BufRead + Send>> {
        let dat_file = self.file(path)?;
        let read_size = if dat_file.is_compressed() {
            dat_file.compressed_size
        } else {
            dat_file.size
        };
        let reader = BufReader::new({
            let mut f = File::open(&self.path)?;
            f.seek(SeekFrom::Start(dat_file.offset as u64))?;
            f.take(read_size as u64)
        });
        Ok(if dat_file.is_compressed() {
            use flate2::bufread::ZlibDecoder;
            Box::new(BufReader::new(ZlibDecoder::new(reader)))
        } else {
            Box::new(reader)
        })
    }

    fn metadata(&self, path: &str) -> Result<Metadata> {
        self.file(path).map(|f| Metadata { len: f.size as u64 })
    }
}

fn read_path<R: Read>(r: &mut R) -> Result<String> {
    let l = r.read_u32::<LittleEndian>()? as usize;
    let mut s = String::with_capacity(l);
    for _ in 0..l {
        let c = r.read_u8()?;
        assert!(c.is_ascii());
        let c = c as char;
        build_normalized_path(&mut s, Some(c));
    }
    build_normalized_path(&mut s, None);

    Ok(s)
}

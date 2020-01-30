use byteorder::{ReadBytesExt, BigEndian};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Result, SeekFrom};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use super::lzss;
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

        let dir_count = reader.read_u32::<BigEndian>()?;

        reader.seek(SeekFrom::Current(4 * 3))?;

        let mut dirs = Vec::with_capacity(dir_count as usize);
        for _ in 0..dir_count {
            dirs.push(read_path(&mut reader)?);
        }

        let mut files = HashMap::new();

        for dir in &dirs {
            let file_count = reader.read_u32::<BigEndian>()?;

            reader.seek(SeekFrom::Current(4 * 3))?;

            for _ in 0..file_count {
                let mut path = String::with_capacity(dir.len() + 257);
                if !dir.is_empty() {
                    path.push_str(dir);
                    if !path.ends_with('\\') {
                        path.push('\\');
                    }
                }
                read_path_into(&mut reader, &mut path)?;

                let _flags = reader.read_u32::<BigEndian>()?;
                let offset = reader.read_u32::<BigEndian>()?;
                let size = reader.read_u32::<BigEndian>()?;
                let compressed_size = reader.read_u32::<BigEndian>()?;

                files.insert(path,
                    DatFile {
                        offset,
                        size,
                        compressed_size,
                    });
            }
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
            // TODO make LzssDecoder implement BufRead
            Box::new(BufReader::new(lzss::LzssDecoder::new(reader, dat_file.size as u64)))
        } else {
            Box::new(reader)
        })
    }

    fn metadata(&self, path: &str) -> Result<Metadata> {
        self.file(path).map(|f| Metadata { len: f.size as u64 })
    }
}

fn read_path<R: Read>(reader: &mut R) -> Result<String> {
    let mut r = String::new();
    read_path_into(reader, &mut r)?;
    Ok(r)
}

fn read_path_into<R: Read>(reader: &mut R, result: &mut String) -> Result<()> {
    let l = reader.read_u8()? as usize;

    if result.capacity() < l {
        let rl = result.len();
        result.reserve_exact(l - rl);
    }

    for _ in 0..l {
        let c = reader.read_u8()?;
        assert!(c.is_ascii());
        let c = c as char;
        build_normalized_path(result, Some(c));
    }
    build_normalized_path(result, None);

    Ok(())
}

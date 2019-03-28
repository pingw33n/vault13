use atoi::atoi;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::fs::FileSystem;

#[derive(Debug, Eq, PartialEq)]
pub struct ScriptInfo {
    pub name: String,
    pub local_var_count: usize,
}

#[derive(Debug)]
pub struct ScriptDb {
    infos: Vec<ScriptInfo>,
}

impl ScriptDb {
    pub fn new(fs: &FileSystem) -> io::Result<Self> {
        let infos = read_lst(&mut fs.reader("scripts/scripts.lst")?)?;
        Ok(Self {
            infos,
        })
    }

    pub fn info(&self, program_id: u32) -> Option<&ScriptInfo> {
        self.infos.get(program_id as usize)
    }
}

fn read_lst(rd: &mut impl BufRead) -> io::Result<Vec<ScriptInfo>> {
    let mut r = Vec::new();
    for l in rd.lines() {
        let mut l = l?;
        l.make_ascii_lowercase();

        const DOT_INT: &'static str = ".int";
        const LOCAL_VARS: &'static str = "local_vars=";

        if let Some(i) = l.find(DOT_INT) {
            let name = l[..i].to_owned();

            let l = &l[i + DOT_INT.len()..];
            let i = l.find('#')
                .and_then(|i| l[i + 1..].find(LOCAL_VARS).map(|j| i + 1 + j + LOCAL_VARS.len()));
            let local_var_count = if let Some(i) = i {
                let l = &l[i..];
                atoi::<u32>(l.trim_end().as_bytes())
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                        "error parsing local_vars in scripts lst file"))? as usize
            } else {
                0
            };

            r.push(ScriptInfo {
                name,
                local_var_count,
            });
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_lst_() {
        fn si(name: &str, local_var_count: usize) -> ScriptInfo {
            ScriptInfo {
                name: name.into(),
                local_var_count,
            }
        }
        let a = read_lst(&mut ::std::io::Cursor::new(
            "\n\
             \t  \t\t\n\
            lines without dot int are ignored\n\
            script1.int\n\
            SCripT2.InT ; \tdsds\n\
            scr ipt 3  .int         #\n\
            Test0.int       ; Used to Test Scripts                          # local_vars=8\n\
            FSBroDor.int    ; Brother Hood Door                             # local_vars=3#\
            ".as_bytes())).unwrap();
        assert_eq!(a, vec![
            si("script1", 0),
            si("script2", 0),
            si("scr ipt 3  ", 0),
            si("test0", 8),
            si("fsbrodor", 3),
        ]);
    }
}
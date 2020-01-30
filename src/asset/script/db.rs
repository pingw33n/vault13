use bstring::bstr;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;

use super::ProgramId;
use crate::asset::message::Messages;
use crate::fs::FileSystem;

#[derive(Debug, Eq, PartialEq)]
pub struct ScriptInfo {
    pub name: String,
    pub local_var_count: usize,
}

pub struct ScriptDb {
    fs: Rc<FileSystem>,
    infos: Vec<ScriptInfo>,
    messages: HashMap<ProgramId, Messages>,
    language: String,
}

impl ScriptDb {
    pub fn new(fs: Rc<FileSystem>, language: &str) -> io::Result<Self> {
        let infos = read_lst(&mut fs.reader("scripts/scripts.lst")?)?;
        Ok(Self {
            fs,
            infos,
            messages: HashMap::new(),
            language: language.into(),
        })
    }

    pub fn info(&self, program_id: ProgramId) -> Option<&ScriptInfo> {
        self.infos.get(program_id.index())
    }

    pub fn load(&self, program_id: ProgramId) -> io::Result<(Box<[u8]>, &ScriptInfo)> {
        let info = self.info_ok(program_id)?;
        let path = format!("scripts/{}.int", info.name);
        let mut code = Vec::new();
        self.fs.reader(&path)?.read_to_end(&mut code)?;
        Ok((code.into(), info))
    }

    pub fn messages(&mut self, program_id: ProgramId) -> io::Result<&Messages> {
        if !self.messages.contains_key(&program_id) {
            let msgs = self.load_messages(program_id)?;
            self.messages.insert(program_id, msgs);
        }

        Ok(&self.messages[&program_id])
    }

    fn load_messages(&self, program_id: ProgramId) -> io::Result<Messages> {
        let info = self.info_ok(program_id)?;
        Messages::read_file(&self.fs, &self.language, &format!("dialog/{}.msg", info.name))
    }

    fn info_ok(&self, program_id: ProgramId) -> io::Result<&ScriptInfo> {
        self.info(program_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput,
                format!("program id {} doesn't exist", program_id.val())))
    }
}

fn read_lst(rd: &mut impl BufRead) -> io::Result<Vec<ScriptInfo>> {
    let mut r = Vec::new();
    for l in rd.lines() {
        let mut l = l?;
        l.make_ascii_lowercase();

        const DOT_INT: &str = ".int";
        const LOCAL_VARS: &str = "local_vars=";

        if let Some(i) = l.find(DOT_INT) {
            let name = l[..i].to_owned();

            let l = &l[i + DOT_INT.len()..];
            let i = l.find('#')
                .and_then(|i| l[i + 1..].find(LOCAL_VARS).map(|j| i + 1 + j + LOCAL_VARS.len()));
            let local_var_count = if let Some(i) = i {
                let mut l = l[i..].as_bytes();
                while let Some(c) = l.last() {
                    if c.is_ascii_digit() {
                        break;
                    } else {
                        l = &l[..l.len() - 1];
                    }
                }
                let l: &bstr = l.into();
                btoi::btoi::<u32>(l.as_bytes())
                    .map_err(|_| Error::new(ErrorKind::InvalidData,
                        format!("error parsing local_vars in scripts lst file: {}",
                            l.display())))? as usize
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
            FSBroDor.int    ; Brother Hood Door                             # local_vars=3# non_digit\
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
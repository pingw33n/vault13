use enum_map::EnumMap;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;

use asset::{EntityKind, LstEntry, read_lst, WeaponKind};
use fs::FileSystem;
use graphics::frm::FrameSet;
use graphics::render::TextureHandle;
use super::*;
use util::EnumExt;

pub struct FrmDb {
    fs: Rc<FileSystem>,
    language: Option<String>,
    lst: EnumMap<EntityKind, Vec<LstEntry>>,
    frms: RefCell<HashMap<Fid, FrameSet>>,
}

impl FrmDb {
    pub fn new(fs: Rc<FileSystem>, language: impl Into<String>) -> io::Result<Self> {
        let language = Some(language.into()).filter(|s| !s.eq_ignore_ascii_case("english"));
        let lst = Self::read_lst_files(&fs)?;
        Ok(Self {
            fs,
            language,
            lst,
            frms: RefCell::new(HashMap::new()),
        })
    }

    // art_get_name()
    /// Returns .frm or .frN file name without path.
    pub fn name(&self, fid: Fid) -> Option<String> {
        let fid = self.normalize_fid(fid);
        let base_name = &self.lst[fid.kind()].get(fid.id0() as usize)?.fields[0];

        Some(match fid.kind() {
            EntityKind::Critter => {
                let wk = WeaponKind::from_u8(fid.id1())?;
                let anim = CritterAnim::from_u8(fid.id2())?;
                let (c1, c2) = critter_anim_codes(wk, anim)?;
                if fid.id3() > 0 {
                    format!("{}{}{}.fr{}", base_name, c1, c2, (b'0' + fid.id3() - 1) as char)
                } else {
                    format!("{}{}{}.frm", base_name, c1, c2)
                }
            }
            EntityKind::Head => {
                static ID2_TO_CODE1: &'static [u8] = b"gggnnnbbbgnb";
                static ID2_TO_CODE2: &'static [u8] = b"vfngfbnfvppp";

                let id2 = fid.id2() as usize;
                if id2 >= ID2_TO_CODE1.len() {
                    return None;
                }

                let c1 = ID2_TO_CODE1[id2] as char;
                let c2 = ID2_TO_CODE2[id2] as char;
                if c2 == 'f' {
                    format!("{}{}{}.frm", c1, c2, fid.id1())
                } else {
                    format!("{}{}.frm", c1, c2)
                }
            }
            _ => base_name.to_string(),
        })
    }

    //  art_exists()
    pub fn exists(&self, fid: Fid) -> bool {
        self.read(fid).is_ok()
    }

    pub fn get_or_load(&self, fid: Fid, render: &mut Render) -> io::Result<Ref<FrameSet>> {
        {
            let mut frms = self.frms.borrow_mut();
            if !frms.contains_key(&fid) {
                let frm = read_frm(&mut self.read(fid)?, render)?;
                frms.insert(fid, frm);
            }
        }
        Ok(self.get(fid))
    }

    pub fn get(&self, fid: Fid) -> Ref<FrameSet> {
        Ref::map(self.frms.borrow(), |v| &v[&fid])
    }

    // art_alias_fid()
    fn normalize_fid(&self, fid: Fid) -> Fid {
        if fid.kind() != EntityKind::Critter {
            return fid;
        }

        // TODO replace unwraps with logging

        use self::CritterAnim::*;
        match CritterAnim::from_u8(fid.id2()).unwrap() {
            | Electrify
            | BurnedToNothing
            | ElectrifiedToNothing
            | ElectrifySf
            | BurnedToNothingSf
            | ElectrifiedToNothingSf
            | FireDance
            | CalledShotPic
            => {
                let alias = self.lst[fid.kind()].get(fid.id0() as usize).unwrap()
                    .fields.get(1).unwrap();
                let alias = alias.parse().unwrap();
                fid.with_id0(alias).unwrap()
            }
            _ => fid,
        }
    }

    fn read(&self, fid: Fid) -> io::Result<Box<BufRead + Send>> {
        let name = self.name(fid)
            .ok_or_else(|| Error::new(ErrorKind::NotFound,
                format!("no name exists for FID: {:?}", fid)))?;
        let path = Self::full_path(fid.kind(), &name, self.language.as_ref());
        let path = if self.fs.exists(&path) ||
                // Let the fs.reader() fail with NotFound.
                self.language.is_none() {
            path
        } else {
            Self::full_path(fid.kind(), &name, None)
        };
        self.fs.reader(&path)
    }

    fn read_lst_files(fs: &FileSystem) -> io::Result<EnumMap<EntityKind, Vec<LstEntry>>> {
        let mut lst = EnumMap::new();
        for kind in EntityKind::iter() {
            let path = Self::full_path(kind, &format!("{}.lst", kind.dir()), None);
            lst[kind] = read_lst(&mut fs.reader(&path)?)?;
        }
        Ok(lst)
    }

    fn full_path(kind: EntityKind, path: &str, language: Option<&String>) -> String {
        if let Some(language) = language {
            format!("art/{}/{}/{}", kind.dir(), language, path)
        } else {
            format!("art/{}/{}", kind.dir(), path)
        }
    }
}

fn critter_anim_codes(weapon_kind: WeaponKind, anim: CritterAnim) -> Option<(char, char)> {
    use self::WeaponKind::*;
    use self::CritterAnim::*;
    Some(match anim {
        ProneToStanding => ('c', 'h'),
        BackToStanding => ('c', 'j'),
        _ if anim >= TakeOut && anim <= FireContinuous => {
            if weapon_kind == Unarmed {
                return None;
            }
            (weapon_kind.anim_code(), anim.code(TakeOut, b'c'))
        }
        CalledShotPic => ('n', 'a'),
        _ if anim >= FallBackSf => ('r', anim.code(FallBackSf, b'a')),
        _ if anim >= FallBack => ('b', anim.code(FallBack, b'a')),
        _ if anim >= ThrowAnim => match weapon_kind {
            Knife | Spear => (weapon_kind.anim_code(), 'm'),
            _ => (Unarmed.anim_code(), 's')
        }
        _ if anim != DodgeAnim => {
            let c1 = match anim {
                Stand | Walk => weapon_kind.anim_code(),
                _ => Unarmed.anim_code(),
            };
            let c2 = anim.code(Stand, b'a');
            (c1, c2)
        }
        _ if weapon_kind == Unarmed => {
            (Unarmed.anim_code(), 'n')
        }
        _ => (weapon_kind.anim_code(), 'e')
    }).map(|(c1, c2)| (c1, c2))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn critter_anim_codes_() {
        for ((wk, anim), exp) in vec![
                ((0, 0), Some("aa")),
                ((7, 0), Some("ja")),
                ((7, 38), Some("jc")),
            ] {
            let act = critter_anim_codes(WeaponKind::from_usize(wk).unwrap(),
                    CritterAnim::from_usize(anim).unwrap())
                .map(|(c1, c2)| format!("{}{}", c1, c2));
            assert_eq!(act, exp.map(|s| s.to_owned()));
        }
    }
}
use enum_map::EnumMap;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;

use asset::{EntityKind, LstEntry, read_lst, WeaponKind};
use fs::FileSystem;
use graphics::frm::FrameSet;
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
        let base_name = &self.lst[fid.kind()].get(fid.id() as usize)?.fields[0];

        Some(match fid {
            Fid::Critter(fid) => {
                let wk = fid.weapon();
                let anim = fid.anim();
                let (c1, c2) = critter_anim_codes(wk, anim)?;
                if let Some(direction) = fid.direction() {
                    format!("{}{}{}.fr{}", base_name, c1, c2, (b'0' + direction as u8) as char)
                } else {
                    format!("{}{}{}.frm", base_name, c1, c2)
                }
            }
            Fid::Head(fid) => {
                static ANIM_TO_CODE1: &'static [u8] = b"gggnnnbbbgnb";
                static ANIM_TO_CODE2: &'static [u8] = b"vfngfbnfvppp";

                let anim = fid.anim() as usize;
                if anim >= ANIM_TO_CODE1.len() {
                    return None;
                }

                let c1 = ANIM_TO_CODE1[anim] as char;
                let c2 = ANIM_TO_CODE2[anim] as char;
                if c2 == 'f' {
                    format!("{}{}{}.frm", c1, c2, (b'0' + fid.sub_anim()) as char)
                } else {
                    format!("{}{}.frm", c1, c2)
                }
            }
            _ => base_name.to_string(),
        })
    }

    //  art_exists()
    pub fn exists(&self, fid: Fid) -> bool {
        let fid = self.normalize_fid(fid);
        self.read(fid).is_ok()
    }

    pub fn get_or_load(&self, fid: Fid, render: &mut Render) -> io::Result<Ref<FrameSet>> {
        let fid = self.normalize_fid(fid);
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
        let fid = self.normalize_fid(fid);
        Ref::map(self.frms.borrow(), |v| &v[&fid])
    }

    // art_alias_fid()
    // art_id()
    fn normalize_fid(&self, fid: Fid) -> Fid {
        if let Fid::Critter(critter_fid) = fid {
            use self::CritterAnim::*;

            let anim = critter_fid.anim();

            let fid = match anim {
                | Electrify
                | BurnedToNothing
                | ElectrifiedToNothing
                | ElectrifySf
                | BurnedToNothingSf
                | ElectrifiedToNothingSf
                | FireDance
                | CalledShotPic
                => {
                    // TODO replace unwraps with logging
                    let alias = self.lst[EntityKind::Critter].get(critter_fid.id() as usize).unwrap()
                        .fields.get(1).unwrap();
                    // TODO parse this once during Self::new().
                    let alias = alias.parse().unwrap();
                    critter_fid.with_id(alias).unwrap().into()
                }
                _ => fid,
            };
            if anim < FallBack
                    || anim > FallFrontBlood
                    || anim == FireDance
                    || !self.exists_no_normalize(fid) {
                critter_fid.with_direction(None).into()
            } else {
                fid
            }
        } else {
            fid
        }
    }

    fn read(&self, fid: Fid) -> io::Result<Box<BufRead + Send>> {
        let name = self.name_no_normalize(fid)
            .ok_or_else(|| Error::new(ErrorKind::NotFound,
                format!("no name exists for FID: {:?}", fid)))?;
        self.read_by_name(fid.kind(), &name)
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

    fn name_no_normalize(&self, fid: Fid) -> Option<String> {
        let base_name = &self.lst[fid.kind()].get(fid.id() as usize)?.fields[0];

        Some(match fid {
            Fid::Critter(fid) => {
                let wk = fid.weapon();
                let anim = fid.anim();
                let (c1, c2) = critter_anim_codes(wk, anim)?;
                if let Some(direction) = fid.direction() {
                    format!("{}{}{}.fr{}", base_name, c1, c2, (b'0' + direction as u8) as char)
                } else {
                    format!("{}{}{}.frm", base_name, c1, c2)
                }
            }
            Fid::Head(fid) => {
                static ANIM_TO_CODE1: &'static [u8] = b"gggnnnbbbgnb";
                static ANIM_TO_CODE2: &'static [u8] = b"vfngfbnfvppp";

                let anim = fid.anim() as usize;
                if anim >= ANIM_TO_CODE1.len() {
                    return None;
                }

                let c1 = ANIM_TO_CODE1[anim] as char;
                let c2 = ANIM_TO_CODE2[anim] as char;
                if c2 == 'f' {
                    format!("{}{}{}.frm", c1, c2, (b'0' + fid.sub_anim()) as char)
                } else {
                    format!("{}{}.frm", c1, c2)
                }
            }
            _ => base_name.to_string(),
        })
    }

    fn read_by_name(&self, kind: EntityKind, name: &str) -> io::Result<Box<BufRead + Send>> {
        let path = Self::full_path(kind, &name, self.language.as_ref());
        let path = if self.fs.exists(&path) ||
                // Let the fs.reader() fail with NotFound.
                self.language.is_none() {
            path
        } else {
            Self::full_path(kind, &name, None)
        };
        self.fs.reader(&path)
    }

    fn exists_no_normalize(&self, fid: Fid) -> bool {
        if let Some(name) = self.name_no_normalize(fid) {
            self.read_by_name(fid.kind(), &name).is_ok()
        } else {
            false
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
        ThrowAnim => match weapon_kind {
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
                ((6, 19), Some("at")),
            ] {
            let wke = WeaponKind::from_usize(wk).unwrap();
            let anime = CritterAnim::from_usize(anim).unwrap();
            let act = critter_anim_codes(wke, anime)
                .map(|(c1, c2)| format!("{}{}", c1, c2));
            assert_eq!(act, exp.map(|s| s.to_owned()),
                "WeaponKind::{:?} ({}), CritterAnim::{:?} ({})", wke, wk, anime, anim);
        }
    }
}
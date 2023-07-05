use enum_map::EnumMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;

use super::*;
use crate::asset::{CritterAnim, EntityKind, LstEntry, read_lst, WeaponKind};
use crate::fs::FileSystem;
use crate::graphics::sprite::FrameSet;
use crate::util::EnumExt;

pub struct FrameDb {
    fs: Rc<FileSystem>,
    language: Option<String>,
    lst: EnumMap<EntityKind, Vec<LstEntry>>,
    frms: RefCell<HashMap<FrameId, Rc<FrameSet>>>,
    texture_factory: TextureFactory,
}

impl FrameDb {
    pub fn new(fs: Rc<FileSystem>, language: &str, texture_factory: TextureFactory)
        -> io::Result<Self>
    {
        let language = Some(language)
            .filter(|s| !s.eq_ignore_ascii_case("english"))
            .map(|s| s.to_owned());
        let lst = Self::read_lst_files(&fs)?;
        Ok(Self {
            fs,
            language,
            lst,
            frms: RefCell::new(HashMap::new()),
            texture_factory,
        })
    }

    // art_get_name()
    /// Returns .frm or .frN file name without path.
    pub fn name(&self, fid: FrameId) -> Option<String> {
        let fid = self.normalize_fid(fid);
        self.name_no_normalize(fid)
    }

    //  art_exists()
    pub fn exists(&self, fid: FrameId) -> bool {
        let fid = self.normalize_fid(fid);
        self.read(fid).is_ok()
    }

    pub fn get(&self, fid: FrameId) -> io::Result<Rc<FrameSet>> {
        let fid = self.normalize_fid(fid);
        let mut frms = self.frms.borrow_mut();
        Ok(if frms.contains_key(&fid) {
            frms[&fid].clone()
        } else {
            let frm = Rc::new(read_frm(&mut self.read(fid)?, &self.texture_factory)?);
            frms.insert(fid, frm.clone());
            frm
        })
    }

    /// Looks for `base_name` and returns its ID if found.
    /// Note the `base_name` format depends on the `kind`. For example for `Critter` it's
    /// just a part of the `.fr_` filename like `hapowr`, and for `Interface` it's a full
    /// filename like `combat.frm`.
    pub fn find_id(&self, kind: EntityKind, base_name: &str) -> Option<u16> {
        for (i, e) in self.lst[kind].iter().enumerate() {
            if e.fields[0].eq_ignore_ascii_case(base_name) {
                return Some(i as u16);
            }
        }
        None
    }

    // art_alias_fid()
    // art_id()
    fn normalize_fid(&self, fid: FrameId) -> FrameId {
        if let FrameId::Critter(critter_fid) = fid {
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
                    let alias = self.lst[EntityKind::Critter].get(critter_fid.idx() as usize).unwrap()
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

    fn read(&self, fid: FrameId) -> io::Result<Box<dyn BufRead + Send>> {
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

    fn name_no_normalize(&self, fid: FrameId) -> Option<String> {
        let base_name = &self.lst[fid.kind()].get(fid.idx() as usize)?.fields[0];

        Some(match fid {
            FrameId::Critter(fid) => {
                let wk = fid.weapon();
                let anim = fid.anim();
                let (c1, c2) = critter_anim_codes(wk, anim)?;
                if let Some(direction) = fid.direction() {
                    format!("{}{}{}.fr{}", base_name, c1, c2, (b'0' + direction as u8) as char)
                } else {
                    format!("{}{}{}.frm", base_name, c1, c2)
                }
            }
            FrameId::Head(fid) => {
                static ANIM_TO_CODE1: &[u8] = b"gggnnnbbbgnb";
                static ANIM_TO_CODE2: &[u8] = b"vfngfbnfvppp";

                let anim = fid.anim() as usize;
                if anim >= ANIM_TO_CODE1.len() {
                    return None;
                }

                let c1 = ANIM_TO_CODE1[anim] as char;
                let c2 = ANIM_TO_CODE2[anim] as char;
                if c2 == 'f' {
                    format!("{}{}{}{}.frm", base_name, c1, c2, (b'0' + fid.sub_anim()) as char)
                } else {
                    format!("{}{}{}.frm", base_name, c1, c2)
                }
            }
            _ => base_name.to_string(),
        })
    }

    fn read_by_name(&self, kind: EntityKind, name: &str) -> io::Result<Box<dyn BufRead + Send>> {
        let path = Self::full_path(kind, name, self.language.as_ref());
        let path = if self.fs.exists(&path) ||
                // Let the fs.reader() fail with NotFound.
                self.language.is_none() {
            path
        } else {
            Self::full_path(kind, name, None)
        };
        self.fs.reader(&path)
    }

    fn exists_no_normalize(&self, fid: FrameId) -> bool {
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
    use num_traits::cast::FromPrimitive;

    #[test]
    fn critter_anim_codes_() {
        for &((wk, anim), exp) in &[
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

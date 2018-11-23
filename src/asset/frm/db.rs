use enum_map::EnumMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;

use asset::{EntityKind, read_lst, WeaponKind};
use fs::FileSystem;
use super::*;
use util::EnumExt;

pub struct FrmDb {
    fs: Rc<FileSystem>,
    language: Option<String>,
    lst: EnumMap<EntityKind, Vec<String>>,
}

fn anim_codes(weapon_kind: WeaponKind, anim: u8) -> Option<(char, char)> {
    use WeaponKind::*;
    Some(match anim {
        36 => ('c', b'h'),
        37 => ('c', b'j'),
        38..=47 => {
            if weapon_kind == Unarmed {
                return None;
            }
            (weapon_kind.anim_code(), anim + b'=')
        }
        64 => ('n', b'a'),
        48..=255 => ('r', anim + b'1'),
        _ if anim >= 18 => match weapon_kind {
            Knife | Spear => (weapon_kind.anim_code(), b'm'),
            _ => (Unarmed.anim_code(), b's')
        }
        _ if anim >= 20 => ('b', anim + b'M'),
        _ if anim != 13 => {
            let c1 = if anim <= 1 && weapon_kind != Unarmed {
                weapon_kind.anim_code()
            } else {
                Unarmed.anim_code()
            };
            let c2 = anim + b'a';
            (c1, c2)
        }
        _ if weapon_kind == Unarmed => {
            (Unarmed.anim_code(), b'n')
        }
        _ => (weapon_kind.anim_code(), b'e')
    }).map(|(c1, c2)| (c1, c2 as char))
}

impl FrmDb {
    pub fn new(fs: Rc<FileSystem>, language: impl Into<String>) -> io::Result<Self> {
        let language = Some(language.into()).filter(|s| !s.eq_ignore_ascii_case("english"));
        let lst = Self::read_lst_files(&fs)?;
        Ok(Self {
            fs,
            language,
            lst,
        })
    }

    // art_get_name()
    pub fn path(&self, fid: Fid) -> Option<String> {
        let id3 = fid.id3();
//        fid_alias = art_alias_fid_(fid);
//          if ( fid_alias != -1 )
//            fid = fid_alias;
        let id0 = fid.id0();
        let id2 = fid.id2();
        let id1 = fid.id1();
        let kind = fid.kind();

        Some(Self::make_path(kind, &match kind {
            EntityKind::Critter => {
                let wk = WeaponKind::from_u8(id1)?;
                let base_name = self.lst[kind].get(id0 as usize)?;
                let (c1, c2) = anim_codes(wk, id2)?;
                format!("{}{}{}.frm", base_name, c1, c2)
            }
            _ => unimplemented!(),
        }, self.language.as_ref().map(|s| s.as_ref())))
//  if ( obj_type == OBJ_TYPE_CRITTER )
//  {
//    if ( art_get_code_(id1, id2, &code1, &code2) == -1 )
//      return 0;
//    if ( id3 )
//    {
//      sprintf_(
//        g_art_name,
//        aSSSSCC_frC,
//        g_resource_root_path,
//        g_art_root_path,
//        g_arts[1].path,
//        g_arts[1].names[num_],
//        (unsigned __int8)code2,
//        (unsigned __int8)code1,
//        id3 + 47);
//      return g_art_name;
//    }
//    sprintf_(
//      g_art_name,
//      aSSSSCC_frm,
//      g_resource_root_path,
//      g_art_root_path,
//      g_arts[1].path,
//      g_arts[1].names[num_],
//      (unsigned __int8)code2,
//      (unsigned __int8)code1);
//    return g_art_name;
//  }
//  if ( obj_type != OBJ_TYPE_HEAD )
//  {
//    sprintf_(
//      g_art_name,
//      aSSSS,
//      g_resource_root_path,
//      g_art_root_path,
//      g_arts[obj_type].path,
//      g_arts[obj_type].names[num_]);
//    return g_art_name;
//  }
//  code2 = g_art_head_id2_to_code2[id2];
//  if ( code2 == 'f' )
//  {
//    sprintf_(
//      g_art_name,
//      aSSSSCCD_frm,
//      g_resource_root_path,
//      g_art_root_path,
//      g_arts[8].path,
//      g_arts[8].names[num_],
//      (unsigned __int8)g_art_head_id2_to_code1[id2],
//      'f',
//      id1);
//    result = g_art_name;
//  }
//  else
//  {
//    sprintf_(
//      g_art_name,
//      aSSSSCC_frm,
//      g_resource_root_path,
//      g_art_root_path,
//      g_arts[8].path,
//      g_arts[8].names[num_],
//      (unsigned __int8)g_art_head_id2_to_code1[id2],
//      (unsigned __int8)code2);
//    result = g_art_name;
//  }
//  return result;
//}
    }

    fn read_lst_files(fs: &FileSystem) -> io::Result<EnumMap<EntityKind, Vec<String>>> {
        let mut lst = EnumMap::new();
        for kind in EntityKind::iter() {
            let path = Self::make_path(kind, &format!("{}.lst", kind.dir()), None);
            lst[kind] = read_lst(&mut fs.reader(&path)?)?;
        }
        Ok(lst)
    }

    fn make_path(kind: EntityKind, path: &str, language: Option<&str>) -> String {
        if let Some(language) = language {
            format!("art/{}/{}/{}", kind.dir(), language, path)
        } else {
            format!("art/{}/{}", kind.dir(), path)
        }
    }
}
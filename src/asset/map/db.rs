use std::collections::HashMap;
use std::io::{self, BufRead};

use crate::fs::FileSystem;
use crate::graphics::EPoint;
use crate::graphics::geometry::hex::TileGrid;

#[derive(Debug, Eq, PartialEq)]
pub struct MapDef {
    pub lookup_name: String,
    pub name: String,
    pub music: Option<String>,
    pub ambient_sfx: Vec<(String, u32)>,
    pub saved: bool,
    pub dead_bodies_age: bool,
    /// Per each elevation.
    pub can_rest_here: Vec<bool>,
    pub pipboy_active: bool,
    pub random_start_points: Vec<EPoint>,
}

pub struct MapDb {
    maps: Vec<MapDef>,
}

impl MapDb {
    pub fn new(fs: &FileSystem) -> io::Result<Self> {
        Self::read(&mut fs.reader("data/maps.txt")?)
    }

    fn read(rd: &mut impl BufRead) -> io::Result<Self> {
        let ini = crate::asset::read_ini(rd)?;
        let mut maps = Vec::new();
        for i in 0..1000 {
            let n = format!("Map {:03}", i);
            let section = if let Some(v) = ini.get(&n) {
                v
            } else {
                break;
            };
            let lookup_name = if let Some(v) = section.get("lookup_name") {
                v.clone()
            } else {
                break;
            };
            let name = section.get("map_name").map(|v| v.to_owned()).expect("missing map_name");
            let music = section.get("music").map(|v| v.to_owned()).to_owned();

            let ambient_sfx = if let Some(ambient_sfx) = section.get("ambient_sfx") {
                ambient_sfx.split(',')
                    .map(|s| {
                        let mut parts = s.splitn(2, ':');
                        let sfx = parts.next().unwrap().trim();
                        let val = parts.next().unwrap().trim();

                        // Handle bad input:
                        // ambient_sfx=water:40, water1:25, animal:15 animal:10, pebble:5, pebble1:5
                        //                                           ^
                        let val = val.split(' ').next().unwrap();

                        let val = val.parse().unwrap();
                        (sfx.into(), val)
                    })
                    .collect()
            } else {
                vec![]
            };

            fn parse_bool(s: &str) -> bool {
                match s.to_ascii_lowercase().as_str() {
                    "no" => false,
                    "yes" => true,
                    _ => panic!("expected yes/no but found: {}", s),
                }
            }

            fn get_bool(m: &HashMap<String, String>, key: &str) -> Option<bool> {
                let s = m.get(key)?;
                let s = s.trim();
                Some(parse_bool(s))
            }

            let saved = get_bool(section, "saved").unwrap_or(true);
            let dead_bodies_age = get_bool(section, "dead_bodies_age").unwrap_or(true);
            let pipboy_active = get_bool(section, "pipboy_active").unwrap_or(true);

            let can_rest_here = if let Some(s) = section.get("can_rest_here") {
                s.split(',')
                    .map(parse_bool)
                    .collect()
            } else {
                vec![true, true, true]
            };
            assert_eq!(can_rest_here.len(), 3);

            let mut random_start_points = Vec::new();
            for i in 0..15 {
                if let Some(s) = section.get(&format!("random_start_point_{}", i)) {
                    let mut elev: Option<u32> = None;
                    let mut tile_num: Option<u32> = None;
                    for s in s.split(',') {
                        let mut parts = s.splitn(2, ':');
                        let k = parts.next().unwrap().trim();
                        let v = parts.next().unwrap().trim();
                        match k {
                            "elev" if elev.is_none() => elev = Some(v.parse().unwrap()),
                            "tile_num" if tile_num.is_none() => tile_num = Some(v.parse().unwrap()),
                            _ => panic!("unknown or duplicated key '{}'", k),
                        }
                    }
                    let elev = elev.unwrap();
                    let tile_num = tile_num.unwrap();
                    let pos = EPoint::new(elev, TileGrid::default().linear_to_rect_inv(tile_num));

                    random_start_points.push(pos);
                } else {
                    break;
                }
            }

            maps.push(MapDef {
                lookup_name,
                name,
                music,
                ambient_sfx,
                saved,
                dead_bodies_age,
                can_rest_here,
                pipboy_active,
                random_start_points,
            })
        }
        Ok(Self {
            maps,
        })
    }

    pub fn get(&self, id: u32) -> Option<&MapDef> {
        self.maps.get(id as usize)
    }
}

#[cfg(test)]
mod test {
    use std::io::*;
    use super::*;
    use crate::graphics::Point;

    #[test]
    fn read() {
        let inp = "
 ; comment
[Map 000]
lookup_name=Desert Encounter 1
map_name=desert1
music=07desert
ambient_sfx=gustwind:20, gustwin1:5 ignored:100, foo:42
saved=No  ; Random encounter maps aren't saved normally (only in savegames)
dead_bodies_age=No
can_rest_here=No,Yes,No  ; All 3 elevations
pipboy_active=no
random_start_point_0=elev:0, tile_num:19086
random_start_point_1=elev:1, tile_num:17302
random_start_point_2=elev:2, tile_num:21315


[Map 001]
lookup_name=Desert Encounter 2
map_name=desert2";

        let exp = &[
            MapDef {
                lookup_name: "Desert Encounter 1".into(),
                name: "desert1".into(),
                music: Some("07desert".into()),
                ambient_sfx: vec![
                    ("gustwind".into(), 20),
                    ("gustwin1".into(), 5),
                    ("foo".into(), 42),
                ],
                saved: false,
                dead_bodies_age: false,
                can_rest_here: vec![false, true, false],
                pipboy_active: false,
                random_start_points: vec![
                    (EPoint::new(0, Point::new(113, 95))),
                    (EPoint::new(1, Point::new(97, 86))),
                    (EPoint::new(2, Point::new(84, 106))),
                ],
            },
            MapDef {
                lookup_name: "Desert Encounter 2".to_string(),
                name: "desert2".to_string(),
                music: None,
                ambient_sfx: vec![],
                saved: true,
                dead_bodies_age: true,
                can_rest_here: vec![true, true, true],
                pipboy_active: true,
                random_start_points: vec![],
            },
        ];

        let act = MapDb::read(&mut BufReader::new(Cursor::new(inp))).unwrap().maps;
        assert_eq!(act, exp);
    }
}

use bstring::bstr;
use byteorder::{BigEndian, ReadBytesExt};
use linearize::{static_map, StaticMap};
use std::cell::RefCell;
use std::collections::hash_map::{self, HashMap};
use std::convert::{TryFrom, TryInto};
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;
use std::str;

use super::*;
use crate::asset::frame::*;
use crate::asset::message::{MessageId, Messages};
use crate::game::script::ScriptPid;
use crate::fs::FileSystem;
use crate::util::RangeInclusive;

pub struct ProtoDb {
    fs: Rc<FileSystem>,
    lst: Lst,
    messages: Messages,
    entity_messages: StaticMap<EntityKind, Messages>,
    protos: RefCell<HashMap<ProtoId, ProtoRef>>,
}

impl ProtoDb {
    pub fn new(fs: Rc<FileSystem>, language: &str) -> io::Result<Self> {
        let lst = Lst::read(&fs)?;
        let messages = Messages::read_file(&fs, language, "game/proto.msg")?;
        let entity_messages = Self::read_entity_messages(&fs, language)?;

        let mut protos = HashMap::new();
        protos.insert(ProtoId::DUDE, Rc::new(RefCell::new(Proto {
            id: ProtoId::DUDE,
            name: None,
            description: None,
            fid: FrameId::new(EntityKind::Critter, None, 0, 0, 0).unwrap(),
            light_radius: 0,
            light_intensity: 0,
            flags: Flag::LightThru.into(),
            flags_ext: BitFlags::empty(),
            script: None,
            sub: SubProto::Critter(Critter {
                flags: BitFlags::empty(),
                base_stats: StaticMap::default(),
                bonus_stats: StaticMap::default(),
                skills: StaticMap::default(),
                body_kind: BodyKind::Biped,
                experience: 0,
                kill_kind: CritterKillKind::Man,
                damage_kind: DamageKind::Melee,
                head_fid: None,
                ai_packet: 0,
                team_id: 0
            }),
        })));

        Ok(Self {
            fs,
            lst,
            messages,
            entity_messages,
            protos: RefCell::new(protos),
        })
    }

    pub fn len(&self, kind: EntityKind) -> usize {
        self.lst.len(kind)
    }

    pub fn messages(&self) -> &Messages {
        &self.messages
    }

    pub fn proto(&self, pid: ProtoId) -> io::Result<ProtoRef> {
        let mut protos = self.protos.borrow_mut();
        match protos.entry(pid) {
            hash_map::Entry::Occupied(e) => Ok(e.get().clone()),
            hash_map::Entry::Vacant(e) => {
                let file_name = self.lst.get(pid)
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                        format!("can't find proto file name for {:?}", pid)))?;
                let path = format!("proto/{}/{}", pid.kind().dir(), file_name);

                let proto = Rc::new(RefCell::new(self.read_proto_file(&path)?));
                e.insert(proto.clone());
                Ok(proto)
            }
        }
    }

    pub fn dude(&self) -> ProtoRef {
        self.protos.borrow().get(&ProtoId::DUDE).unwrap().clone()
    }

    fn read_entity_messages(fs: &FileSystem, language: &str)
        -> io::Result<StaticMap<EntityKind, Messages>>
    {
        let mut map = StaticMap::default();
        for k in proto_entity_kinds() {
            let path = format!("game/pro_{}.msg", &k.dir()[..4]);
            map[k] = Messages::read_file(fs, language, &path)?;
        }
        Ok(map)
    }

    fn read_proto_file(&self, path: &str) -> io::Result<Proto> {
        let rd = &mut self.fs.reader(path)?;

        let pid = ProtoId::read(rd)?;
        let message_id = rd.read_i32::<BigEndian>()?;
        let fid = FrameId::read(rd)?;

        let light_radius = rd.read_i32::<BigEndian>()?;
        let light_intensity = rd.read_i32::<BigEndian>()?;
        let v = rd.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(v)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid proto flags: {:x}", v)))?;
        let v = rd.read_u32::<BigEndian>()?;
        let mut flags_ext = BitFlags::from_bits(v)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid proto flags ext: {:x}", v)))?;

        let kind = pid.kind();
        let script = match kind {
            | EntityKind::Item
            | EntityKind::Critter
            | EntityKind::Scenery
            | EntityKind::Wall
            => {
                ScriptPid::read_opt(rd)?
            }
            | EntityKind::SqrTile
            | EntityKind::Misc
            | EntityKind::Interface
            | EntityKind::Inventory
            | EntityKind::Head
            | EntityKind::Background
            | EntityKind::Skilldex
            => {
                None
            }
        };

        let sub = match kind {
            EntityKind::Item => SubProto::Item(Self::read_item(rd, &mut flags_ext)?),
            EntityKind::Critter => SubProto::Critter(Self::read_critter(rd)?),
            EntityKind::Scenery => SubProto::Scenery(Self::read_scenery(rd)?),
            EntityKind::Wall => SubProto::Wall(Self::read_wall(rd)?),
            EntityKind::SqrTile => SubProto::SqrTile(Self::read_sqr_tile(&mut flags_ext)?),
            EntityKind::Misc => SubProto::Misc,
            | EntityKind::Interface
            | EntityKind::Inventory
            | EntityKind::Head
            | EntityKind::Background
            | EntityKind::Skilldex
            => return Err(Error::new(ErrorKind::InvalidData, "unsupported proto kind"))
        };

        // proto_name()
        let name = self.msg(pid.kind(), message_id, 0)?
            .map(|s| s.to_owned());
        // proto_description()
        let description = self.msg(pid.kind(), message_id, 1)?
            .map(|s| s.to_owned());

        Ok(Proto {
            id: pid,
            name,
            description,
            fid,
            light_radius,
            light_intensity,
            flags,
            flags_ext,
            script,
            sub,
        })
    }

    fn read_item(rd: &mut impl Read, flags_ext: &mut BitFlags<FlagExt>) -> io::Result<Item> {
        let item_kind = read_enum(rd, "invalid item kind")?;
        let material = read_enum(rd, "invalid item material")?;
        let size = rd.read_i32::<BigEndian>()?;
        let weight = rd.read_i32::<BigEndian>()?.try_into().unwrap();
        let price = rd.read_i32::<BigEndian>()?;
        let inventory_fid = FrameId::read_opt(rd)?;
        let sound_id = rd.read_u8()?;
        let sub = match item_kind {
            ItemKind::Armor => SubItem::Armor(Self::read_armor(rd)?),
            ItemKind::Container => SubItem::Container(Self::read_container(rd)?),
            ItemKind::Drug => SubItem::Drug(Self::read_drug(rd)?),
            ItemKind::Weapon => SubItem::Weapon(Self::read_weapon(rd, flags_ext)?),
            ItemKind::Ammo => SubItem::Ammo(Self::read_ammo(rd)?),
            ItemKind::Misc => SubItem::Misc(Self::read_misc_item(rd)?),
            ItemKind::Key => SubItem::Key(Self::read_key(rd)?),
        };
        Ok(Item {
            material,
            size,
            weight,
            price,
            inventory_fid,
            sound_id,
            sub,
        })
    }

    fn read_container(rd: &mut impl Read) -> io::Result<Container> {
        let capacity = rd.read_i32::<BigEndian>()?;
        let flags = BitFlags::from_bits(rd.read_u32::<BigEndian>()?)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid container flags"))?;
        Ok(Container {
            capacity,
            flags,
        })
    }

    fn read_armor(rd: &mut impl Read) -> io::Result<Armor> {
        let armor_class = rd.read_i32::<BigEndian>()?;
        let mut damage_resistance = StaticMap::default();
        for &dmg in DamageKind::basic() {
            damage_resistance[dmg] = rd.read_i32::<BigEndian>()?;
        }
        let mut damage_threshold = StaticMap::default();
        for &dmg in DamageKind::basic() {
            damage_threshold[dmg] = rd.read_i32::<BigEndian>()?;
        }
        let perk = read_opt_enum(rd, "invalid armor perk")?;
        let male_fidx = FrameId::read(rd)?.idx();
        let female_fidx = FrameId::read(rd)?.idx();
        Ok(Armor {
            armor_class,
            damage_resistance,
            damage_threshold,
            perk,
            male_fidx,
            female_fidx,
        })
    }

    fn read_drug(rd: &mut impl Read) -> io::Result<Drug> {
        let stats = {
            let s1 = rd.read_i32::<BigEndian>()?;
            let s2 = rd.read_i32::<BigEndian>()?;
            let s3 = rd.read_i32::<BigEndian>()?;
            [s1, s2, s3]
        };
        let mut mods = [(0, [0, 0, 0]); 3];
        for (i, m) in mods.iter_mut().enumerate() {
            let d = if i != 0 {
                rd.read_u32::<BigEndian>()?
            } else {
                0
            };
            let m1 = rd.read_i32::<BigEndian>()?;
            let m2 = rd.read_i32::<BigEndian>()?;
            let m3 = rd.read_i32::<BigEndian>()?;
            *m = (d, [m1, m2, m3]);
        }
        let addiction_chance = rd.read_u32::<BigEndian>()?;
        let addiction_perk = read_opt_enum(rd, "invalid drug addiction perk")?;
        let addiction_delay = rd.read_u32::<BigEndian>()?;

        let mut effects = Vec::with_capacity(3);
        let stat_i_start = if stats[0] == -2 {
            assert!(stats[1] != -2);
            let stat = get_opt_enum(stats[1], "invalid drug stat")?;
            if let Some(stat) = stat {
                for mods_i in 0..3 {
                    let mods = mods[mods_i];
                    let from = mods.1[0];
                    let to = mods.1[1];
                    assert!(from <= to);
                    if from != 0 || to != 0 {
                        effects.push(DrugEffect {
                            delay: mods.0,
                            stat,
                            modifier: DrugEffectModifier::Random(from, to),
                        });
                    }
                }
            }
            2
        } else {
            0
        };
        for (stat_i, &stat) in stats.iter().enumerate().skip(stat_i_start) {
            let stat = get_opt_enum(stat, "invalid drug stat")?;
            if let Some(stat) = stat {
                for mods_i in 0..3 {
                    let mods = mods[mods_i];
                    let mod_v = mods.1[stat_i];
                    if mod_v != 0 {
                        effects.push(DrugEffect {
                            delay: mods.0,
                            stat,
                            modifier: DrugEffectModifier::Fixed(mod_v),
                        });
                    }
                }
            }
        }
        Ok(Drug {
            effects,
            addiction: DrugAddiction {
                chance: addiction_chance,
                perk: addiction_perk,
                delay: addiction_delay,
            }
        })
    }

    fn read_weapon(rd: &mut impl Read, flags_ext: &mut BitFlags<FlagExt>) -> io::Result<Weapon> {
        let primary = get_enum(flags_ext.bits() & 0xf, "invalid weapon primary attack kind")?;
        let secondary = get_enum((flags_ext.bits() >> 4) & 0xf, "invalid weapon secondary attack kind")?;
        let attack_kinds = static_map! {
            AttackGroup::Primary => primary,
            AttackGroup::Secondary => secondary,
        };
        *flags_ext = BitFlags::from_bits(flags_ext.bits() & !0xff).unwrap();

        let animation_code = read_enum(rd, "invalid weapon animation code")?;
        let damage = RangeInclusive {
            start: rd.read_i32::<BigEndian>()?,
            end: rd.read_i32::<BigEndian>()?,
        };
        let damage_kind = read_enum(rd, "invalid weapon damage kind")?;
        let primary = rd.read_i32::<BigEndian>()?;
        let secondary = rd.read_i32::<BigEndian>()?;
        let max_ranges = static_map! {
            AttackGroup::Primary => primary,
            AttackGroup::Secondary => secondary,
        };
        let projectile_pid = ProtoId::read_opt(rd)?;
        let min_strength = rd.read_i32::<BigEndian>()?;
        let primary = rd.read_i32::<BigEndian>()?;
        let secondary = rd.read_i32::<BigEndian>()?;
        let ap_costs = static_map! {
            AttackGroup::Primary => primary,
            AttackGroup::Secondary => secondary,
        };
        let crit_failure_table = rd.read_i32::<BigEndian>()?;
        let perk = read_opt_enum(rd, "invalid weapon perk")?;
        let burst_bullet_count = rd.read_i32::<BigEndian>()?;
        let caliber = rd.read_u32::<BigEndian>()?;
        let ammo_proto_id = ProtoId::read_opt(rd)?;
        let max_ammo_count = rd.read_i32::<BigEndian>()?.try_into().unwrap();
        let sound_id = rd.read_u8()?;

        Ok(Weapon {
            attack_kinds,
            kind: animation_code,
            damage,
            damage_kind,
            max_ranges,
            projectile_pid,
            min_strength,
            ap_costs,
            crit_failure_table,
            perk,
            burst_bullet_count,
            caliber,
            ammo_proto_id,
            max_ammo_count,
            sound_id,
        })
    }

    fn read_ammo(rd: &mut impl Read) -> io::Result<Ammo> {
        let caliber = rd.read_u32::<BigEndian>()?;
        let max_ammo_count = rd.read_i32::<BigEndian>()?.try_into().unwrap();
        let ac_modifier = rd.read_i32::<BigEndian>()?;
        let dr_modifier = rd.read_i32::<BigEndian>()?;
        let damage_mult = rd.read_i32::<BigEndian>()?;
        let damage_div = rd.read_i32::<BigEndian>()?;
        Ok(Ammo {
            caliber,
            max_ammo_count,
            ac_modifier,
            dr_modifier,
            damage_mult,
            damage_div,
        })
    }

    fn read_misc_item(rd: &mut impl Read) -> io::Result<MiscItem> {
        let ammo_proto_id = ProtoId::read_opt(rd)?;
        let ammo_kind = rd.read_u32::<BigEndian>()?;
        let max_ammo_count = rd.read_i32::<BigEndian>()?.try_into().unwrap_or(0);
        Ok(MiscItem {
            ammo_proto_id,
            ammo_kind,
            max_ammo_count,
        })
    }

    fn read_key(rd: &mut impl Read) -> io::Result<Key> {
        let id = rd.read_i32::<BigEndian>()?;
        Ok(Key {
            id,
        })
    }

    fn read_critter(rd: &mut impl Read) -> io::Result<Critter> {
        let head_fid = FrameId::read_opt(rd)?;
        let ai_packet = rd.read_i32::<BigEndian>()?;
        let team_id = rd.read_i32::<BigEndian>()?;

        let v = rd.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(v)
            .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid critter proto flags: {:x}", v))).unwrap();

        let mut base_stats = StaticMap::default();
        for stat in 0..35 {
            base_stats[Stat::from_usize(stat).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let mut bonus_stats = StaticMap::default();
        for stat in 0..35 {
            bonus_stats[Stat::from_usize(stat).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let mut skills = StaticMap::default();
        for skill in 0..18 {
            skills[Skill::from_usize(skill).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let body_kind = read_enum(rd, "invalid body kind in critter proto")?;
        let experience = rd.read_i32::<BigEndian>()?;
        let kill_kind = read_enum(rd, "invalid kill kind in critter proto")?;
        let damage_kind = read_enum(rd, "invalid damage kind in critter proto")?;

        Ok(Critter {
            flags,
            base_stats,
            bonus_stats,
            skills,
            body_kind,
            experience,
            kill_kind,
            damage_kind,
            head_fid,
            ai_packet,
            team_id,
        })
    }

    fn read_scenery(rd: &mut impl Read) -> io::Result<Scenery> {
        let kind = read_enum(rd, "invalid scenery kind")?;
        let material = read_enum(rd, "invalid material")?;
        let sound_id = rd.read_u8()?;
        let sub = match kind {
            SceneryKind::Door => {
                let flags = rd.read_u32::<BigEndian>()?;
                let flags = BitFlags::from_bits(flags)
                    .ok().ok_or_else(|| Error::new(ErrorKind::InvalidData,
                        format!("invalid door flags: {:x}", flags)))?;
                let key_id = rd.read_i32::<BigEndian>()?;
                SubScenery::Door(Door {
                    flags,
                    key_id,
                })
            }
            SceneryKind::Stairs => {
                let location = rd.read_i32::<BigEndian>()?;
                let map = rd.read_i32::<BigEndian>()?;

                let exit = if let Ok(location) = u32::try_from(location) {
                    Some(MapExit::decode(map, location)
                        .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                            format!("invalid map exit: map={} location={}", map, location)))?)
                } else {
                    None
                };
                SubScenery::Stairs(Stairs {
                    exit,
                })
            }
            SceneryKind::Elevator => {
                let kind = rd.read_u32::<BigEndian>()?;
                let level = rd.read_u32::<BigEndian>()?;
                SubScenery::Elevator(Elevator {
                    kind,
                    level,
                })
            }
            SceneryKind::LadderUp | SceneryKind::LadderDown => {
                let ladder_kind = match kind {
                    SceneryKind::LadderUp => LadderKind::Up,
                    SceneryKind::LadderDown => LadderKind::Down,
                    _ => unreachable!(),
                };
                let location = rd.read_i32::<BigEndian>()?;
                let exit = if let Ok(location) = u32::try_from(location) {
                    Some(MapExit::decode(0, location)
                        .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                            format!("invalid map exit location: {}", location)))?)
                } else {
                    None
                };
                SubScenery::Ladder(Ladder {
                    kind: ladder_kind,
                    exit,
                })
            }
            SceneryKind::Misc => {
                let _ = rd.read_u32::<BigEndian>()?;
                SubScenery::Misc
            }
        };
        Ok(Scenery {
            material,
            sound_id,
            sub,
        })
    }

    fn read_wall(rd: &mut impl Read) -> io::Result<Wall> {
        let material = read_enum(rd, "invalid wall material")?;
        Ok(Wall {
            material,
        })
    }

    fn read_sqr_tile(flags_ext: &mut BitFlags<FlagExt>) -> io::Result<SqrTile> {
        let material = get_enum(flags_ext.bits(), "invalid sqr tile material")?;
        *flags_ext = BitFlags::empty();
        Ok(SqrTile {
            material,
        })
    }

    fn msg(&self, kind: EntityKind, msg_id: MessageId, base: MessageId)
        -> io::Result<Option<&bstr>>
    {
        Ok(self.entity_messages[kind].get(base + msg_id)
            .map(|m| m.text.as_ref()))
    }
}

struct Lst {
    lst: StaticMap<EntityKind, Vec<LstEntry>>,
}

impl Lst {
    pub fn read(fs: &FileSystem) -> io::Result<Self> {
        let mut lst = StaticMap::default();
        for k in proto_entity_kinds() {
            lst[k] = Self::read_lst_file(fs, k)?;
        }
        Ok(Self {
            lst,
        })
    }

    pub fn len(&self, kind: EntityKind) -> usize {
        self.lst[kind].len()
    }

    pub fn get(&self, pid: ProtoId) -> Option<&str> {
        let id = pid.id() as usize;
        if id > 0 {
            self.lst[pid.kind()].get(id - 1).map(|e| e.fields[0].as_ref())
        } else {
            None
        }
    }

    fn read_lst_file(fs: &FileSystem, kind: EntityKind) -> io::Result<Vec<LstEntry>> {
        let path = format!("proto/{0}/{0}.lst", kind.dir());
        read_lst(&mut fs.reader(&path)?)
    }
}

fn get_enum<T: FromPrimitive>(v: u32, err: &str) -> io::Result<T> {
    T::from_u32(v)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, err))
}

fn get_opt_enum<T: FromPrimitive>(v: i32, err: &str) -> io::Result<Option<T>> {
    if v >= 0 {
        Ok(Some(get_enum(v as u32, err)?))
    } else {
        Ok(None)
    }
}

fn read_enum<T: FromPrimitive>(rd: &mut impl Read, err: &str) -> io::Result<T> {
    get_enum(rd.read_u32::<BigEndian>()?, err)
}

fn read_opt_enum<T: FromPrimitive>(rd: &mut impl Read, err: &str) -> io::Result<Option<T>> {
    get_opt_enum(rd.read_i32::<BigEndian>()?, err)
}

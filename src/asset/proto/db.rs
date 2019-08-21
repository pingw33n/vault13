use bstring::bstr;
use byteorder::{BigEndian, ReadBytesExt};
use enum_map::EnumMap;
use num_traits::FromPrimitive;
use std::cell::{Ref, RefCell};
use std::cmp;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, prelude::*};
use std::rc::Rc;
use std::str;

use super::*;
use crate::asset::frame::*;
use crate::asset::message::Messages;
use crate::fs::FileSystem;

pub struct ProtoDb {
    fs: Rc<FileSystem>,
    lst: Lst,
    messages: EnumMap<EntityKind, Messages>,
    protos: RefCell<HashMap<ProtoId, Proto>>,
}

impl ProtoDb {
    pub fn new(fs: Rc<FileSystem>, language: &str) -> io::Result<Self> {
        let lst = Lst::read(&fs)?;
        let messages = Self::read_messages(&fs, language)?;
        Ok(Self {
            fs,
            lst,
            messages,
            protos: RefCell::new(HashMap::new()),
        })
    }

    pub fn len(&self, kind: EntityKind) -> usize {
        self.lst.len(kind)
    }

    // proto_name()
    pub fn name(&self, pid: ProtoId) -> io::Result<Option<&bstr>> {
        self.msg(pid, 0)
    }

    // proto_description()
    pub fn description(&self, pid: ProtoId) -> io::Result<Option<&bstr>> {
        self.msg(pid, 1)
    }

    pub fn proto(&self, pid: ProtoId) -> io::Result<Ref<Proto>> {
        {
            let mut protos = self.protos.borrow_mut();
            if !protos.contains_key(&pid) {
                let name = self.lst.get(pid)
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                        format!("can't find proto file name for {:?}", pid)))?;
                let path = format!("proto/{}/{}", pid.kind().dir(), name);
                let proto = self.read_proto_file(&path)?;
                protos.insert(pid, proto);
            }
        }
        Ok(Ref::map(self.protos.borrow(), |p| p.get(&pid).unwrap()))
    }

    pub fn kind(&self, pid: ProtoId) -> ExactEntityKind {
        // As in original item_get_type().
        // TODO maybe update the proto instead?
        if pid == ProtoId::SHIV {
            return ExactEntityKind::Item(ItemKind::Misc);
        }
        self.proto(pid).unwrap().kind()
    }

    // proto_action_can_use()
    pub fn can_use(&self, pid: ProtoId) -> bool {
        if let Ok(proto) = self.proto(pid) {
            proto.flags_ext.contains(FlagExt::CanUse) ||
                proto.kind() == ExactEntityKind::Item(ItemKind::Container)
        } else {
            false
        }
    }

    // proto_action_can_use_on()
    pub fn can_use_on(&self, pid: ProtoId) -> bool {
        if let Ok(proto) = self.proto(pid) {
            proto.flags_ext.contains(FlagExt::CanUseOn) ||
                proto.kind() == ExactEntityKind::Item(ItemKind::Drug)
        } else {
            false
        }
    }

    // proto_action_can_talk_to()
    pub fn can_talk_to(&self, pid: ProtoId) -> bool {
        if let Ok(proto) = self.proto(pid) {
            proto.flags_ext.contains(FlagExt::CanTalk) ||
                proto.kind() == ExactEntityKind::Critter
        } else {
            false
        }
    }

    // proto_action_can_pick_up()
    pub fn can_pick_up(&self, pid: ProtoId) -> bool {
        if let Ok(proto) = self.proto(pid) {
            proto.flags_ext.contains(FlagExt::CanPickup) ||
                proto.kind() == ExactEntityKind::Item(ItemKind::Container)
        } else {
            false
        }
    }

    fn read_messages(fs: &FileSystem, language: &str) -> io::Result<EnumMap<EntityKind, Messages>> {
        let mut map = EnumMap::new();
        for k in proto_entity_kinds() {
            let path = format!("game/pro_{}.msg", &k.dir()[..4]);
            map[k] = Messages::read_file(fs, language, &path)?;
        }
        Ok(map)
    }

    fn read_proto_file(&self, path: &str) -> io::Result<Proto> {
        Self::read_proto(&mut self.fs.reader(&path)?)
    }

    fn read_proto(rd: &mut impl Read) -> io::Result<Proto> {
        let pid = ProtoId::read(rd)?;
        let message_id = rd.read_i32::<BigEndian>()?;
        let fid = FrameId::read(rd)?;

        let light_radius = rd.read_i32::<BigEndian>()?;
        let light_intensity = rd.read_i32::<BigEndian>()?;
        let v = rd.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid proto flags: {:x}", v)))?;
        let v = rd.read_u32::<BigEndian>()?;
        let mut flags_ext = BitFlags::from_bits(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid proto flags ext: {:x}", v)))?;

        let kind = pid.kind();
        let script_id = match kind {
            | EntityKind::Item
            | EntityKind::Critter
            | EntityKind::Scenery
            | EntityKind::Wall
            => {
                Some(rd.read_u32::<BigEndian>()?)
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

        let proto = match kind {
            EntityKind::Item => Variant::Item(Self::read_item(rd, &mut flags_ext)?),
            EntityKind::Critter => Variant::Critter(Self::read_critter(rd)?),
            EntityKind::Scenery => Variant::Scenery(Self::read_scenery(rd)?),
            EntityKind::Wall => Variant::Wall(Self::read_wall(rd)?),
            EntityKind::SqrTile => Variant::SqrTile(Self::read_sqr_tile(&mut flags_ext)?),
            EntityKind::Misc => Variant::Misc,
            | EntityKind::Interface
            | EntityKind::Inventory
            | EntityKind::Head
            | EntityKind::Background
            | EntityKind::Skilldex
            => return Err(Error::new(ErrorKind::InvalidData, "unsupported proto kind"))
        };

        Ok(Proto {
            pid,
            message_id,
            fid,
            light_radius,
            light_intensity,
            flags,
            flags_ext,
            script_id,
            proto,
        })
    }

    fn read_item(rd: &mut impl Read, flags_ext: &mut BitFlags<FlagExt>) -> io::Result<Item> {
        let item_kind = read_enum(rd, "invalid item kind")?;
        let material = read_enum(rd, "invalid item material")?;
        let size = rd.read_i32::<BigEndian>()?;
        let weight = rd.read_i32::<BigEndian>()?;
        let price = rd.read_i32::<BigEndian>()?;
        let inventory_fid = FrameId::read_opt(rd)?;
        let sound_id = rd.read_u8()?;
        let item = match item_kind {
            ItemKind::Armor => ItemVariant::Armor(Self::read_armor(rd)?),
            ItemKind::Container => ItemVariant::Container(Self::read_container(rd)?),
            ItemKind::Drug => ItemVariant::Drug(Self::read_drug(rd)?),
            ItemKind::Weapon => ItemVariant::Weapon(Self::read_weapon(rd, flags_ext)?),
            ItemKind::Ammo => ItemVariant::Ammo(Self::read_ammo(rd)?),
            ItemKind::Misc => ItemVariant::Misc(Self::read_misc_item(rd)?),
            ItemKind::Key => ItemVariant::Key(Self::read_key(rd)?),
        };
        Ok(Item {
            material,
            size,
            weight,
            price,
            inventory_fid,
            sound_id,
            item,
        })
    }

    fn read_container(rd: &mut impl Read) -> io::Result<Container> {
        let capacity = rd.read_i32::<BigEndian>()?;
        let flags = BitFlags::from_bits(rd.read_u32::<BigEndian>()?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid container flags"))?;
        Ok(Container {
            capacity,
            flags,
        })
    }

    fn read_armor(rd: &mut impl Read) -> io::Result<Armor> {
        let armor_class = rd.read_i32::<BigEndian>()?;
        let mut damage_resistance = EnumMap::new();
        for d in 0..7 {
            let dmg = DamageKind::from_usize(d).unwrap();
            damage_resistance[dmg] = rd.read_i32::<BigEndian>()?;
        }
        let mut damage_threshold = EnumMap::new();
        for d in 0..7 {
            let dmg = DamageKind::from_usize(d).unwrap();
            damage_threshold[dmg] = rd.read_i32::<BigEndian>()?;
        }
        let perk = read_opt_enum(rd, "invalid armor perk")?;
        let male_fid = FrameId::read(rd)?;
        let female_fid = FrameId::read(rd)?;
        Ok(Armor {
            armor_class,
            damage_resistance,
            damage_threshold,
            perk,
            male_fid,
            female_fid,
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
        for i in 0..3 {
            let d = if i != 0 {
                rd.read_u32::<BigEndian>()?
            } else {
                0
            };
            let m1 = rd.read_i32::<BigEndian>()?;
            let m2 = rd.read_i32::<BigEndian>()?;
            let m3 = rd.read_i32::<BigEndian>()?;
            mods[i] = (d, [m1, m2, m3]);
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
        for stat_i in stat_i_start..3 {
            let stat = get_opt_enum(stats[stat_i], "invalid drug stat")?;
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
        let attack_kind = Dual {
            primary: get_enum(flags_ext.bits() & 0xf, "invalid weapon primary attack kind")?,
            secondary: get_enum((flags_ext.bits() >> 4) & 0xf, "invalid weapon secondary attack kind")?,
        };
        *flags_ext = BitFlags::from_bits(flags_ext.bits() & !0xff).unwrap();

        let animation_code = read_enum(rd, "invalid weapon animation code")?;
        let damage = rd.read_i32::<BigEndian>()?..=rd.read_i32::<BigEndian>()?;
        let damage_kind = read_enum(rd, "invalid weapon damage kind")?;
        let max_range = Dual {
            primary: rd.read_i32::<BigEndian>()?,
            secondary: rd.read_i32::<BigEndian>()?,
        };
        let projectile_pid = ProtoId::read_opt(rd)?;
        let min_strength = rd.read_i32::<BigEndian>()?;
        let ap_cost =  Dual {
            primary: rd.read_i32::<BigEndian>()?,
            secondary: rd.read_i32::<BigEndian>()?,
        };
        let crit_failure_table = rd.read_i32::<BigEndian>()?;
        let perk = read_opt_enum(rd, "invalid weapon perk")?;
        let burst_bullet_count = rd.read_i32::<BigEndian>()?;
        let caliber = rd.read_i32::<BigEndian>()?;
        let ammo_pid = ProtoId::read_opt(rd)?;
        let max_ammo = rd.read_i32::<BigEndian>()?;
        let sound_id = rd.read_u8()?;

        Ok(Weapon {
            attack_kind,
            animation_code,
            damage,
            damage_kind,
            max_range,
            projectile_pid,
            min_strength,
            ap_cost,
            crit_failure_table,
            perk,
            burst_bullet_count,
            caliber,
            ammo_pid,
            max_ammo,
            sound_id,
        })
    }

    fn read_ammo(rd: &mut impl Read) -> io::Result<Ammo> {
        let caliber = rd.read_i32::<BigEndian>()?;
        let magazine_size = rd.read_i32::<BigEndian>()?;
        let ac_modifier = rd.read_i32::<BigEndian>()?;
        let dr_modifier = rd.read_i32::<BigEndian>()?;
        let damage_mult = rd.read_i32::<BigEndian>()?;
        let damage_div = rd.read_i32::<BigEndian>()?;
        Ok(Ammo {
            caliber,
            magazine_size,
            ac_modifier,
            dr_modifier,
            damage_mult,
            damage_div,
        })
    }

    fn read_misc_item(rd: &mut impl Read) -> io::Result<MiscItem> {
        let charge_pid = ProtoId::read_opt(rd)?;
        let charge_kind = rd.read_u32::<BigEndian>()?;
        let max_charges = cmp::max(rd.read_i32::<BigEndian>()?, 0);
        Ok(MiscItem {
            charge_pid,
            charge_kind,
            max_charges,
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
        let ai_packet = rd.read_u32::<BigEndian>()?;
        let team_id = rd.read_u32::<BigEndian>()?;

        let v = rd.read_u32::<BigEndian>()?;
        let flags = BitFlags::from_bits(v)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData,
                format!("invalid critter proto flags: {:x}", v))).unwrap();

        let mut base_stats = EnumMap::new();
        for stat in 0..35 {
            base_stats[Stat::from_usize(stat).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let mut bonus_stats = EnumMap::new();
        for stat in 0..35 {
            bonus_stats[Stat::from_usize(stat).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let mut skills = EnumMap::new();
        for skill in 0..18 {
            skills[Skill::from_usize(skill).unwrap()] = rd.read_i32::<BigEndian>()?;
        }
        let body_kind = rd.read_u32::<BigEndian>()?;
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
        let scenery = match kind {
            SceneryKind::Door => {
                let flags = rd.read_u32::<BigEndian>()?;
                let key_id = rd.read_u32::<BigEndian>()?;
                SceneryVariant::Door(Door {
                    flags,
                    key_id,
                })
            }
            SceneryKind::Stairs => {
                let elevation_and_tile = rd.read_u32::<BigEndian>()?;
                let map_id = rd.read_u32::<BigEndian>()?;
                SceneryVariant::Stairs(Stairs {
                    elevation_and_tile,
                    map_id,
                })
            }
            SceneryKind::Elevator => {
                let kind = rd.read_u32::<BigEndian>()?;
                let level = rd.read_u32::<BigEndian>()?;
                SceneryVariant::Elevator(Elevator {
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
                let elevation_and_tile = rd.read_u32::<BigEndian>()?;
                SceneryVariant::Ladder(Ladder {
                    kind: ladder_kind,
                    elevation_and_tile,
                })
            }
            SceneryKind::Misc => {
                let _ = rd.read_u32::<BigEndian>()?;
                SceneryVariant::Misc
            }
        };
        Ok(Scenery {
            material,
            sound_id,
            scenery,
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

    fn msg(&self, pid: ProtoId, base: i32) -> io::Result<Option<&bstr>> {
        let proto = self.proto(pid)?;
        Ok(self.messages[pid.kind()].get(base + proto.message_id)
            .map(|m| m.text.as_ref()))
    }
}

struct Lst {
    lst: EnumMap<EntityKind, Vec<LstEntry>>,
}

impl Lst {
    pub fn read(fs: &FileSystem) -> io::Result<Self> {
        let mut lst = EnumMap::new();
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
        if let Some(id) = pid.id() {
            self.lst[pid.kind()].get(id as usize).map(|e| e.fields[0].as_ref())
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
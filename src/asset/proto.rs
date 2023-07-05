mod db;
mod id;

use bstring::{bstr, BString};
use enumflags2::{bitflags, BitFlags};
use enum_map::EnumMap;
use num_traits::cast::FromPrimitive;

pub use id::ProtoId;
pub use db::ProtoDb;

use super::*;
use crate::asset::EntityKind;
use crate::asset::frame::{FrameId, Idx};
use crate::asset::message::MessageId;
use crate::game::script::ScriptPid;
use crate::graphics::geometry::hex::TileGrid;
use crate::util::{enum_iter, EnumIter, RangeInclusive};

/// "The doorway seems to be blocked."
pub const MSG_DOORWAY_SEEMS_TO_BE_BLOCKED: MessageId = 597;

/// "You see: %s."
pub const MSG_YOU_SEE_X: MessageId = 480;

pub type ProtoRef = std::rc::Rc<std::cell::RefCell<Proto>>;

#[derive(Debug)]
pub struct Proto {
    id: ProtoId,
    name: Option<BString>,
    description: Option<BString>,
    pub fid: FrameId,
    pub light_radius: i32,
    pub light_intensity: i32,
    pub flags: BitFlags<Flag>,
    pub flags_ext: BitFlags<FlagExt>,
    pub script: Option<ScriptPid>,
    pub sub: SubProto,
}

impl Proto {
    pub fn kind(&self) -> ExactEntityKind {
        self.sub.kind()
    }

    pub fn id(&self) -> ProtoId {
        self.id
    }

    // critter_name
    // item_name
    pub fn name(&self) -> Option<&bstr> {
        self.name.as_ref().map(|s| s.as_ref())
    }

    pub fn set_name(&mut self, name: BString) {
        self.name = Some(name);
    }

    pub fn description(&self) -> Option<&bstr> {
        self.description.as_ref().map(|s| s.as_ref())
    }

    // proto_action_can_use()
    pub fn can_use(&self) -> bool {
        self.flags_ext.contains(FlagExt::CanUse) ||
            self.kind() == ExactEntityKind::Item(ItemKind::Container)
    }

    // proto_action_can_use_on()
    pub fn can_use_on(&self) -> bool {
        self.flags_ext.contains(FlagExt::CanUseOn) ||
            self.kind() == ExactEntityKind::Item(ItemKind::Drug)
    }

    // proto_action_can_talk_to()
    pub fn can_talk_to(&self) -> bool {
        self.flags_ext.contains(FlagExt::CanTalk) ||
            self.kind() == ExactEntityKind::Critter
    }

    // proto_action_can_pick_up()
    pub fn can_pick_up(&self) -> bool {
        self.flags_ext.contains(FlagExt::CanPickup) ||
            self.kind() == ExactEntityKind::Item(ItemKind::Container)
    }

    // item_w_max_ammo
    #[must_use]
    pub fn max_ammo_count(&self) -> Option<u32> {
        Some(if let SubProto::Item(item) = &self.sub {
            match &item.sub {
                SubItem::Ammo(a) => a.max_ammo_count,
                SubItem::Weapon(w) => w.max_ammo_count,
                _ => return None,
            }
        } else {
            return None;
        })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, enum_as_inner::EnumAsInner)]
pub enum SubProto {
    Item(Item),
    Critter(Critter),
    Scenery(Scenery),
    Wall(Wall),
    SqrTile(SqrTile),
    Misc,
}

impl SubProto {
    pub fn kind(&self) -> ExactEntityKind {
        use self::SubProto::*;
        match self {
            Item(ref v) => ExactEntityKind::Item(v.sub.kind()),
            Critter(_) => ExactEntityKind::Critter,
            Scenery(ref v) => ExactEntityKind::Scenery(v.sub.kind()),
            Wall(_) => ExactEntityKind::Wall,
            SqrTile(_) => ExactEntityKind::SqrTile,
            Misc => ExactEntityKind::Misc,
        }
    }

    pub fn as_armor(&self) -> Option<&Armor> {
        self.as_item()?.sub.as_armor()
    }

    pub fn as_armor_mut(&mut self) -> Option<&mut Armor> {
        self.as_item_mut()?.sub.as_armor_mut()
    }

    pub fn as_weapon(&self) -> Option<&Weapon> {
        self.as_item()?.sub.as_weapon()
    }

    pub fn as_weapon_mut(&mut self) -> Option<&mut Weapon> {
        self.as_item_mut()?.sub.as_weapon_mut()
    }

    pub fn as_ammo(&self) -> Option<&Ammo> {
        self.as_item()?.sub.as_ammo()
    }

    pub fn as_ammo_mut(&mut self) -> Option<&mut Ammo> {
        self.as_item_mut()?.sub.as_ammo_mut()
    }
}

#[derive(Debug)]
pub struct Item {
    pub material: Material,
    pub size: i32,
    pub weight: u32,
    pub price: i32,
    pub inventory_fid: Option<FrameId>,
    pub sound_id: u8,
    pub sub: SubItem,
}

#[derive(Debug, enum_as_inner::EnumAsInner)]
pub enum SubItem {
    Armor(Armor),
    Container(Container),
    Drug(Drug),
    Weapon(Weapon),
    Ammo(Ammo),
    Misc(MiscItem),
    Key(Key),
}

impl SubItem {
    pub fn kind(&self) -> ItemKind {
        use self::SubItem::*;
        match self {
            Armor(_)        => ItemKind::Armor,
            Container(_)    => ItemKind::Container,
            Drug(_)         => ItemKind::Drug,
            Weapon(_)       => ItemKind::Weapon,
            Ammo(_)         => ItemKind::Ammo,
            Misc(_)         => ItemKind::Misc,
            Key(_)          => ItemKind::Key,
        }
    }
}

#[derive(Debug)]
pub struct Armor {
  pub armor_class: i32,
  pub damage_resistance: EnumMap<DamageKind, i32>,
  pub damage_threshold: EnumMap<DamageKind, i32>,
  pub perk: Option<Perk>,
  pub male_fidx: Idx,
  pub female_fidx: Idx,
}

impl Armor {
    pub fn stat(&self, stat: Stat) -> Option<i32> {
        if stat == Stat::ArmorClass {
            Some(self.armor_class)
        } else if let Some(d) = stat.resist_damage_kind() {
            Some(self.damage_resistance[d])
        } else {
            stat.thresh_damage_kind().map(|d| self.damage_threshold[d])
        }
    }
}

#[derive(Debug)]
pub struct Container {
    pub capacity: i32,
    pub flags: BitFlags<ContainerFlag>,
}

#[bitflags]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContainerFlag {
    CannotPickUp = 1,
    MagicHandsGround = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrugEffectModifier {
    Fixed(i32),
    Random(i32, i32),
}

#[derive(Debug)]
pub struct DrugEffect {
    pub delay: u32,
    pub stat: Stat,
    pub modifier: DrugEffectModifier,
}

#[derive(Debug)]
pub struct DrugAddiction {
    pub chance: u32,
    pub perk: Option<Perk>,
    pub delay: u32,
}

#[derive(Debug)]
pub struct Drug {
    pub effects: Vec<DrugEffect>,
    pub addiction: DrugAddiction,
}

#[derive(Debug)]
pub struct Weapon {
    pub attack_kinds: EnumMap<AttackGroup, AttackKind>,
    pub kind: WeaponKind,
    // item_w_damage_min_max
    pub damage: RangeInclusive<i32>,
    pub damage_kind: DamageKind,
    pub max_ranges: EnumMap<AttackGroup, i32>,
    pub projectile_pid: Option<ProtoId>,
    pub min_strength: i32,
    pub ap_costs: EnumMap<AttackGroup, i32>,
    pub crit_failure_table: i32,
    pub perk: Option<Perk>,
    // Number of bullets per burst shot.
    pub burst_bullet_count: i32,
    // proto.msg:300
    pub caliber: u32,
    pub ammo_proto_id: Option<ProtoId>,
    /// Magazine capacity.
    pub max_ammo_count: u32,
    pub sound_id: u8,
}

#[derive(Debug)]
pub struct Ammo {
    pub caliber: u32,
    pub max_ammo_count: u32,
    pub ac_modifier: i32,
    pub dr_modifier: i32,
    pub damage_mult: i32,
    pub damage_div: i32,
}

#[derive(Debug)]
pub struct MiscItem {
    pub ammo_proto_id: Option<ProtoId>,
    pub ammo_kind: u32,
    pub max_ammo_count: u32,
}

#[derive(Debug)]
pub struct Key {
    pub id: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum BodyKind {
    Biped = 0,
    Quadruped = 1,
    Robotic = 2,
}

#[derive(Debug)]
pub struct Critter {
    pub flags: BitFlags<CritterFlag>,
    pub base_stats: EnumMap<Stat, i32>,
    pub bonus_stats: EnumMap<Stat, i32>,
    pub skills: EnumMap<Skill, i32>,
    pub body_kind: BodyKind,
    pub experience: i32,
    //proto.msg:1450
    pub kill_kind: CritterKillKind,
    pub damage_kind: DamageKind,
    pub head_fid: Option<FrameId>,
    pub ai_packet: i32,
    pub team_id: i32,
}

#[bitflags]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CritterFlag {
    NoBarter        = 0x00000002, // Can barter with.
    NoSteal         = 0x00000020, // Can't steal from.
    NoDrop          = 0x00000040, // Doesn't drop items.
    NoLoseLimbs     = 0x00000080, // Can't shoot off limbs.
    Ages            = 0x00000100, // Dead body doesn't disappear.
    NoHeal          = 0x00000200, // HP doesn't restore over time.
    Invulnerable    = 0x00000400,
    NoDeadBody      = 0x00000800, // Dead body disappears immediately.
    SpecialDeath    = 0x00001000, // Has special death animation.
    RangedMelee     = 0x00002000, // Melee attack is possible at a distance.
    NoKnock         = 0x00004000, // Can't knock down.
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Primitive)]
pub enum CritterKillKind {
  Man = 0x0,
  Woman = 0x1,
  Children = 0x2,
  SuperMutant = 0x3,
  Ghoul = 0x4,
  Brahmin = 0x5,
  Radscorpion = 0x6,
  Rat = 0x7,
  Floater = 0x8,
  Centaur = 0x9,
  Robot = 0xA,
  Dog = 0xB,
  Manti = 0xC,
  DeathClaw = 0xD,
  Plant = 0xE,
  Gecko = 0xF,
  Alien = 0x10,
  GiantAnt = 0x11,
  BigBadBoss = 0x12,
}

#[derive(Debug)]
pub struct Scenery {
    pub material: Material,
    pub sound_id: u8,
    pub sub: SubScenery,
}

#[derive(Debug, enum_as_inner::EnumAsInner)]
pub enum SubScenery {
    Door(Door),
    Stairs(Stairs),
    Elevator(Elevator),
    Ladder(Ladder),
    Misc,
}

impl SubScenery {
    pub fn kind(&self) -> SceneryKind {
        use self::SubScenery::*;
        match self {
            Door(_) => SceneryKind::Door,
            Stairs(_) => SceneryKind::Stairs,
            Elevator(_) => SceneryKind::Elevator,
            Ladder(ref l) => match l.kind {
                LadderKind::Up => SceneryKind::LadderUp,
                LadderKind::Down => SceneryKind::LadderDown,
            }
            Misc => SceneryKind::Misc,
        }
    }
}

#[derive(Debug)]
pub struct Door {
    pub flags: BitFlags<DoorFlag>,
    pub key_id: i32,
}

#[derive(Debug)]
pub struct Stairs {
    pub exit: Option<MapExit>,
}

#[derive(Debug)]
pub struct Elevator {
    pub kind: u32,
    pub level: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LadderKind {
    Up,
    Down,
}

#[derive(Debug)]
pub struct Ladder {
    pub kind: LadderKind,
    pub exit: Option<MapExit>,
}

#[derive(Debug)]
pub struct Wall {
    pub material: Material,
}

#[derive(Debug)]
pub struct SqrTile {
    pub material: Material,
}

#[derive(Clone, Copy, Eq, Debug, PartialEq)]
pub enum WorldMapKind {
    Town,
    World,
}

#[derive(Debug, Clone)]
pub struct MapExit {
    pub map: TargetMap,
    pub pos: EPoint,
    pub direction: Direction,
}

impl MapExit {
    pub fn decode(map: i32, location: u32) -> Option<MapExit> {
        let map = if map != 0 {
            TargetMap::decode(map)?
        } else {
            TargetMap::CurrentMap
        };
        let elevation = location & 0x3ffffff;
        let pos = TileGrid::default().linear_to_rect_inv((location & 0xE0000000) >> 29)
            .elevated(elevation);
        let direction = Direction::from_u32((location & 0x1C000000) >> 26)?;
        Some(MapExit {
            map,
            pos,
            direction,
        })
    }
}

#[derive(Clone, Copy, Eq, Debug, PartialEq)]
pub enum TargetMap {
    Map {
        map_id: u32,
    },
    CurrentMap,
    WorldMap(WorldMapKind),
}

impl TargetMap {
    pub fn decode(map: i32) -> Option<TargetMap> {
        Some(match map {
            -1 => TargetMap::WorldMap(WorldMapKind::Town),
            -2 => TargetMap::WorldMap(WorldMapKind::World),
            map_id if map_id >= 0 => {
                TargetMap::Map { map_id: map_id as u32 }
            }
            _ => return None,
        })
    }
}

// Subset that has prototypes.
pub fn proto_entity_kinds() -> EnumIter<EntityKind> {
    enum_iter(..=EntityKind::Misc)
}

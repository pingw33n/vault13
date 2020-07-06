use enumflags2::BitFlags;
use enumflags2_derive::EnumFlags;
use enum_primitive_derive::Primitive;
use if_chain::if_chain;
use log::*;
use slotmap::{SecondaryMap, SlotMap};
use std::cell::{Ref, RefCell, RefMut};
use std::cmp;
use std::mem;
use std::rc::Rc;

use crate::asset::*;
use crate::asset::frame::*;
use crate::asset::proto::*;
use crate::asset::script::ProgramId;
use crate::game::rpg::Rpg;
use crate::game::script::{Scripts, ScriptIid};
use crate::graphics::{EPoint, Point, Rect};
use crate::graphics::geometry::TileGridView;
use crate::graphics::geometry::hex::{self, Direction, TileGrid};
use crate::graphics::geometry::hex::path_finder::*;
use crate::graphics::lighting::light_grid::*;
use crate::graphics::render::Canvas;
use crate::graphics::sprite::*;
use crate::util::{EnumExt, VecExt};
use crate::util::array2d::Array2d;
use crate::vm::PredefinedProc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetFrame {
    Index(usize),
    Last,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathTo {
    Object(Handle),
    Point {
        point: Point,
        /// If the `point` hex is blocked, allow path to end at a non-blocked neighbor hex.
        neighbor_if_blocked: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Outline {
    pub style: OutlineStyle,
    pub translucent: bool,
    pub disabled: bool,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    // item_total_weight
    pub fn weight(&self, objects: &Objects) -> u32 {
        let mut weight = 0;
        for item in &self.items {
            weight += objects.get(item.object).item_weight(objects).unwrap() * item.count;
        }
        weight

        // See https://trello.com/c/ksAC8gWn
    }
}

#[derive(Clone, Debug)]
pub struct InventoryItem {
    pub object: Handle,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquipmentSlot {
    Armor,
    Hand(Hand),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hand {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LightEmitter {
    pub intensity: u32,
    pub radius: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub pos: Point,
    pub fid: FrameId,
}

impl Egg {
    #[must_use]
    pub fn hit_test(&self, p: Point, tile_grid: &impl TileGridView, frm_db: &FrameDb) -> bool {
        let screen_pos = tile_grid.center_to_screen(self.pos);
        let frms = frm_db.get(self.fid).unwrap();
        let frml = &frms.frame_lists[Direction::NE];
        let frm = &frml.frames[0];

        let bounds = frm.bounds_centered(screen_pos, frml.center);
        if !bounds.contains(p) {
            return false;
        }
        let p = p - bounds.top_left();
        frm.mask.test(p).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Hit {
    /// Hit a translucent object.
    pub translucent: bool,

    /// Hit a `Wall` or `Scenery` object at point which is masked by the Egg.
    pub with_egg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CantTalkSpatial {
    Unreachable,
    TooFar,
}

new_handle_type! {
    pub struct Handle;
}

#[derive(Debug)]
pub struct Object {
    handle: Option<Handle>,
    pub flags: BitFlags<Flag>,
    pub updated_flags: BitFlags<UpdatedFlag>,
    pos: Option<EPoint>,
    pub screen_pos: Point,
    pub screen_shift: Point,
    pub fid: FrameId,
    pub frame_idx: usize,
    pub direction: Direction,
    light_emitter: LightEmitter,
    proto: Option<ProtoRef>,
    pub inventory: Inventory,
    pub outline: Option<Outline>,
    pub script: Option<(ScriptIid, ProgramId)>,
    pub sub: SubObject,
}

impl Object {
    pub fn new(
        fid: FrameId,
        proto: Option<ProtoRef>,
        pos: Option<EPoint>,
        sub: SubObject,
    ) -> Self {
        Self {
            handle: None,
            pos,
            screen_pos: Point::new(0, 0),
            screen_shift: Point::new(0, 0),
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            flags: BitFlags::empty(),
            updated_flags: BitFlags::empty(),
            proto,
            inventory: Inventory::new(),
            light_emitter: LightEmitter {
                intensity: 0,
                radius: 0,
            },
            outline: None,
            script: None,
            sub,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.unwrap()
    }

    pub fn kind(&self) -> EntityKind {
        self.fid.kind()
    }

    pub fn proto_ref(&self) -> Option<&RefCell<Proto>> {
        self.proto.as_ref().map(|v| v.as_ref())
    }

    pub fn proto(&self) -> Option<Ref<Proto>> {
        self.proto_ref().map(|v| v.borrow())
    }

    pub fn proto_mut(&self) -> Option<RefMut<Proto>> {
        self.proto_ref().map(|v| v.borrow_mut())
    }

    pub fn proto_id(&self) -> Option<ProtoId> {
        self.proto().map(|v| v.id())
    }

    pub fn is_dude(&self) -> bool {
        self.proto_id() == Some(ProtoId::DUDE)
    }

    pub fn pos(&self) -> EPoint {
        self.pos.unwrap()
    }

    pub fn try_pos(&self) -> Option<EPoint> {
        self.pos
    }

    pub fn set_pos(&mut self, pos: Option<EPoint>) {
        assert!(self.handle.is_none(), "use Objects::set_pos()");
        self.pos = pos;
    }

    pub fn light_emitter(&self) -> LightEmitter {
        self.light_emitter
    }

    pub fn set_light_emitter(&mut self, v: LightEmitter) {
        assert!(self.handle.is_none());
        self.light_emitter = v;
    }

    pub fn render(&mut self, canvas: &mut dyn Canvas, light: u32,
            frm_db: &FrameDb, tile_grid: &impl TileGridView,
            egg: Option<Egg>) {
        if self.flags.contains(Flag::TurnedOff) {
            return;
        }

        let light = if self.fid.kind() == EntityKind::Interface {
            0x10000
        } else {
            light
        };

        let effect = self.get_effect(tile_grid, egg);
        let sprite = self.create_sprite(light, effect, tile_grid);

        self.screen_pos = sprite.render(canvas, frm_db).top_left();
    }

    pub fn render_outline(&self, canvas: &mut dyn Canvas, frm_db: &FrameDb, tile_grid: &impl TileGridView) {
        if self.flags.contains(Flag::TurnedOff) {
            return;
        }
        if let Some(outline) = self.outline {
            if outline.disabled {
                return;
            }
            let effect = Effect::Outline {
                style: outline.style,
                translucent: outline.translucent,
            };
            let sprite = self.create_sprite(0x10000, Some(effect), tile_grid);
            sprite.render(canvas, frm_db);
        }
    }

    // obj_bound()
    pub fn bounds(&self, frm_db: &FrameDb, tile_grid: &impl TileGridView,
        include_outline: bool,
    ) -> Rect {
        self.do_with_frame_list(frm_db, |frml, frm|
            self.bounds0(frml.center, frm.size(), tile_grid, include_outline))
    }

    // critter_is_dead()
    #[must_use]
    pub fn is_critter_dead(&self) -> bool {
        // FIXME
        false
    }

    // critter_is_prone()
    #[must_use]
    pub fn is_critter_prone(&self) -> bool {
        if let Some(critter) = self.sub.as_critter() {
            critter.combat.damage_flags.contains(DamageFlag::KnockedDown | DamageFlag::KnockedOut)
                || self.fid.critter().unwrap().anim().is_prone()
        } else {
            false
        }
    }

    // obj_is_locked()
    // obj_is_lockable()
    #[must_use]
    pub fn is_locked(&self) -> Option<bool> {
        self.sub.as_scenery().and_then(|s| s.as_door()).map(|d| d.flags.contains(DoorFlag::Locked))
            .or_else(|| if self.proto().map(|p| p.kind()) == Some(ExactEntityKind::Item(ItemKind::Container)) {
                Some(self.updated_flags.contains(UpdatedFlag::Locked))
            } else {
                None
            })
    }

    // obj_lock
    // obj_unlock
    pub fn set_locked(&mut self, locked: bool) {
        match self.kind() {
            EntityKind::Item => {
                if locked {
                    self.updated_flags.insert(UpdatedFlag::Locked);
                } else {
                    self.updated_flags.remove(UpdatedFlag::Locked);
                }
            }
            EntityKind::Scenery => {
                let door = self.sub.as_scenery_mut().unwrap().as_door_mut().unwrap();
                if locked {
                    door.flags.insert(DoorFlag::Locked);
                } else {
                    door.flags.remove(DoorFlag::Locked);
                }
            }
            _ => unreachable!(),
        }
    }

    // obj_lock_is_jammed
    #[must_use]
    pub fn is_lock_jammed(&self) -> Option<bool> {
        self.is_locked()?;
        Some(match self.kind() {
            EntityKind::Item => {
                self.updated_flags.contains(UpdatedFlag::Jammed)
            }
            EntityKind::Scenery => {
                let door = self.sub.as_scenery().unwrap().as_door().unwrap();
                 door.flags.contains(DoorFlag::Jammed)
            }
            _ => unreachable!(),
        })
    }

    // obj_jam_lock
    // obj_jam_unlock
    pub fn set_lock_jammed(&mut self, jammed: bool) {
        if self.is_locked().is_none() {
            return;
        }
        match self.kind() {
            EntityKind::Item => {
                if jammed {
                    self.updated_flags.insert(UpdatedFlag::Jammed);
                } else {
                    self.updated_flags.remove(UpdatedFlag::Jammed);
                }
            }
            EntityKind::Scenery => {
                let door = self.sub.as_scenery_mut().unwrap().as_door_mut().unwrap();
                if jammed {
                    door.flags.insert(DoorFlag::Jammed);
                } else {
                    door.flags.remove(DoorFlag::Jammed);
                }
            }
            _ => {}
        }
    }

    // obj_intersects_with
    #[must_use]
    pub fn hit_test(&self, p: Point, frm_db: &FrameDb, tile_grid: &impl TileGridView) -> Option<Hit> {
        if self.flags.contains(Flag::TurnedOff) {
            return None;
        }

        // TODO The test may return None for outlined objects.
        // Need to have mask for outlined object.
        let bounds = self.bounds(frm_db, tile_grid, false);
        if !bounds.contains(p) {
            return None;
        }

        let p = p - bounds.top_left();
        if !self.do_with_frame(frm_db, |frm| frm.mask.test(p).unwrap()) {
            return None;
        }

        let translucent = self.has_trans() && !self.flags.contains(Flag::TransNone);
        Some(Hit {
            translucent,
            with_egg: false,
        })
    }

    #[must_use]
    pub fn distance(&self, other: &Object) -> Option<u32> {
        let mut r = hex::distance(self.pos?.point, other.pos?.point);
        if r > 0 && self.flags.contains(Flag::MultiHex) {
            r -= 1;
        }
        if r > 0 && other.flags.contains(Flag::MultiHex) {
            r -= 1;
        }
        Some(r)
    }

    // item_get_type
    #[must_use]
    pub fn item_kind(&self) -> Option<ItemKind> {
        let proto = self.proto()?;
        Some(if proto.id() == ProtoId::SHIV {
            ItemKind::Misc // instead of ItemKind::Weapon
        } else {
            proto.sub.as_item()?.sub.kind()
        })
    }

    // item_weight
    #[must_use]
    pub fn item_weight(&self, objects: &Objects) -> Option<u32> {
        let proto = self.proto()?;
        let item = proto.sub.as_item()?;
        let mut weight = if proto.flags_ext.contains(FlagExt::WallEastOrWest) { // TODO overloaded: ItemHidden
            0
        } else {
            item.weight
        };
        match &item.sub {
            SubItem::Container(_) => weight += self.inventory.weight(objects),
            SubItem::Weapon(_) => {
                let item_obj = self.sub.as_item().unwrap();
                if item_obj.ammo_count > 0 {
                    let ammo = if self.item_kind() == Some(ItemKind::Weapon) {
                        item_obj.ammo_proto.as_ref()
                    } else {
                        None
                    };
                    if let Some(ammo) = ammo {
                        let ammo = ammo.borrow();
                        let ammo = ammo.sub.as_item().unwrap();
                        weight += ammo.weight * ((item_obj.ammo_count - 1) /
                            ammo.sub.as_ammo().unwrap().max_ammo_count + 1);
                    }
                }
            }
            SubItem::Armor(_) => match proto.id() {
                | ProtoId::POWER_ARMOR
                | ProtoId::HARDENED_POWER_ARMOR
                | ProtoId::ADVANCED_POWER_ARMOR
                | ProtoId::ADVANCED_POWER_ARMOR_MK2
                => weight /= 2,
                _ => {}
            }
            _ => {}
        }
        Some(weight)
    }

    // inven_left_hand
    // inven_right_hand
    // inven_worn
    #[must_use]
    pub fn equipment(&self, slot: EquipmentSlot, objects: &Objects) -> Option<Handle> {
        let flag = match slot {
            EquipmentSlot::Armor => Flag::Worn,
            EquipmentSlot::Hand(Hand::Left) => Flag::LeftHand,
            EquipmentSlot::Hand(Hand::Right) => Flag::RightHand,
        };
        self.find_inventory_item(objects, |o| o.flags.contains(flag))
    }

    /// Whether this object can be talked to.
    // obj_action_can_talk_to()
    #[must_use]
    pub fn can_talk_to(&self) -> bool {
        if_chain! {
            if let SubObject::Critter(c) = &self.sub;
            if c.is_active();
            if let Some(proto) = self.proto();
            then {
                proto.can_talk_to()
            } else {
                false
            }
        }
    }

    // obj_action_can_use()
    #[must_use]
    pub fn can_use(&self) -> bool {
        if let Some(proto) = self.proto() {
            match proto.id() {
                | ProtoId::ACTIVE_DYNAMITE
                | ProtoId::ACTIVE_FLARE
                | ProtoId::ACTIVE_PLASTIC_EXPLOSIVE
                => false,
                _ => proto.can_use(),
            }
        } else {
            false
        }
    }

    // item_w_can_unload
    #[must_use]
    pub fn is_unloadable_weapon(&self) -> bool {
        if self.item_kind() != Some(ItemKind::Weapon)
            // This doesn't seem to be necessary as the Switchblade proto has ammo_proto_id == None
            // || self.proto_id().unwrap() == ProtoId::from_packed(319).unwrap()
        {
            return false;
        }
        if let Some(weapon) = self.proto().unwrap().sub.as_item().and_then(|i| i.sub.as_weapon()) {
            weapon.max_ammo_count > 0
                && weapon.ammo_proto_id.is_some()
                && self.sub.as_item().unwrap().ammo_count > 0
        } else {
            false
        }
    }

    // item_w_can_reload
    #[must_use]
    pub fn can_reload_weapon(&self, ammo: &Self) -> Option<u32> {
        let weapon_proto = self.proto()?;
        if weapon_proto.id() == ProtoId::SOLAR_SCORCHER {
            unimplemented!("TODO");
        }
        let weapon = self.sub.as_item()?;
        let ammo_proto = ammo.proto()?;
        let weapon_proto = weapon_proto.sub.as_weapon()?;
        if weapon_proto.caliber == ammo_proto.sub.as_ammo()?.caliber
            // TODO this is a weird condition
            && (weapon.ammo_count == 0 || weapon.ammo_proto.as_ref()?.borrow().id() == ammo_proto.id())
        {
            let free = weapon_proto.max_ammo_count.checked_sub(weapon.ammo_count).unwrap();
            let r = std::cmp::min(free, ammo.total_ammo_count(1).unwrap());
            Some(r)
        } else {
            None
        }
    }

    // item_w_reload
    #[must_use]
    pub fn reload_weapon(&mut self, ammo: &mut Object) -> Option<u32> {
        let ammo_count = self.can_reload_weapon(ammo)?;
        self.sub.as_item_mut()?.ammo_count += ammo_count;
        let weapon_proto = self.proto()?;
        if weapon_proto.id() == ProtoId::SOLAR_SCORCHER {
            return Some(0);
        }

        let ammo = ammo.sub.as_item_mut().unwrap();
        ammo.ammo_count -= ammo_count;
        Some(ammo.ammo_count)
    }

    #[must_use]
    pub fn total_ammo_count(&self, clip_count: u32) -> Option<u32> {
        let ammo_proto = self.proto()?;
        Some(self.sub.as_item()?.ammo_count +
            ammo_proto.sub.as_ammo()?.max_ammo_count * clip_count.saturating_sub(1))
    }

    // item_w_curr_ammo
    #[must_use]
    pub fn ammo_count(&self) -> Option<u32> {
        self.sub.as_item().map(|i| i.ammo_count)
    }

    // item_w_set_curr_ammo
    pub fn set_ammo_count(&mut self, ammo_count: u32) {
        let max = self.proto().unwrap().max_ammo_count().unwrap();
        let ammo_count = cmp::min(ammo_count, max);
        self.sub.as_item_mut().unwrap().ammo_count = ammo_count;
    }

    // item_w_range
    #[must_use]
    pub fn weapon_range(&self,
        attack_group: AttackGroup,
        rpg: &Rpg,
        objects: &Objects,
    ) -> Option<i32> {
        let proto = self.proto()?;
        // TODO if  a2 != 4 && a2 != 5 && (a2 < 8 || a2 > 19)
        let weapon = proto.sub.as_weapon()?;
        let mut r = weapon.max_ranges[attack_group];
        if weapon.attack_kinds[attack_group].category() == AttackCategory::Throw {
            r += rpg.stat(Stat::Strength, self, objects);
            if proto.id().is_dude() {
                r += 2 * rpg.perk(Perk::HeaveHo, ProtoId::DUDE) as i32;
            }
        }
        // TODO else if ( critter_flag_check_(obj->_.pid, 0x2000) )
        //   {
        //     result = 2;
        //   }
        //   else
        //   {
        //     result = 1;
        //   }
        Some(r)
    }

    // critterIsOverfloaded
    #[must_use]
    pub fn is_overloaded(&self, rpg: &Rpg, objects: &Objects) -> bool {
        rpg.stat(Stat::CarryWeight, self, objects) < self.inventory.weight(objects) as i32
    }

    // item_identical
    #[must_use]
    pub fn is_same(&self, o: &Object) -> bool {
        if self.proto_id() != o.proto_id()
            || self.script != o.script
            || self.flags.intersects(Flag::Worn | Flag::LeftHand | Flag::RightHand | Flag::Used)
            || o.flags.intersects(Flag::Worn | Flag::LeftHand | Flag::RightHand | Flag::Used)
            || !self.inventory.items.is_empty()
            || !o.inventory.items.is_empty()
        {
            return false;
        }
        let proto = self.proto().unwrap();
        if proto.kind() == ExactEntityKind::Item(ItemKind::Container) {
            return false;
        }
        match (&self.sub, &o.sub) {
            (SubObject::Item(v1), SubObject::Item(v2)) => {
                (proto.kind() == ExactEntityKind::Item(ItemKind::Ammo)
                    || proto.id() != ProtoId::BOTTLE_CAPS
                    || v1.ammo_count == v2.ammo_count)
                    && v1.ammo_proto.as_ref().map(|p| p.borrow().id()) == v2.ammo_proto.as_ref().map(|p| p.borrow().id())
            }
            (SubObject::Key(v1), SubObject::Key(v2)) => {
                v1.id == v2.id
            }
            (SubObject::Critter(_), SubObject::Critter(_)) => unimplemented!(),
            (SubObject::MapExit(_), SubObject::MapExit(_)) => unimplemented!(),
            (SubObject::Scenery(_), SubObject::Scenery(_)) => unimplemented!(),
            _ => unreachable!(),
        }
    }

    // adjust_fid
    pub fn equipped_fid(&self, objects: &Objects, rpg: &Rpg) -> FrameId {
        if self.proto().unwrap().kind() != ExactEntityKind::Critter {
            return self.fid;
        }
        let dude = self.sub.as_critter().unwrap().try_dude();

        let idx = self.equipment(EquipmentSlot::Armor, objects)
            .map(|armor| {
                let armor = objects.get(armor);
                let armor = armor.proto().unwrap();
                let armor = armor.sub.as_armor().unwrap();
                if rpg.stat(Stat::Gender, self, objects) == 1 {
                    armor.female_fidx
                } else {
                    armor.male_fidx
                }
            })
            .or(dude.map(|d| d.naked_fidx))
            .unwrap_or(self.fid.idx());

        let active_hand = dude.map(|d| d.active_hand).unwrap_or(Hand::Left);
        let weapon = self.equipment(EquipmentSlot::Hand(active_hand), objects)
            .and_then(|item| {
                objects.get(item).proto().unwrap().sub
                    .as_weapon().map(|w| w.kind)
            })
            .unwrap_or(WeaponKind::Unarmed);

        FrameId::new_critter(Some(self.direction), CritterAnim::Stand, weapon, idx).unwrap()
    }

    fn find_inventory_item(&self, objects: &Objects, f: impl Fn(&Object) -> bool) -> Option<Handle> {
        self.inventory.items.iter()
            .map(|i| i.object)
            .find(|&o| f(&objects.get(o)))
    }

    fn bounds0(&self, frame_center: Point, frame_size: Point, tile_grid: &impl TileGridView,
        include_outline: bool,
    ) -> Rect {
        let mut r = if let Some(pos) = self.pos {
            let top_left =
                tile_grid.center_to_screen(pos.point)
                + frame_center
                + self.screen_shift
                - Point::new(frame_size.x / 2, frame_size.y - 1);
            let bottom_right = top_left + frame_size;
            Rect::with_points(top_left, bottom_right)
        } else {
            Rect::with_points(self.screen_pos, self.screen_pos + frame_size)
        };

        if include_outline {
            let has_outline = self.outline.map(|o| !o.disabled).unwrap_or(false);
            if has_outline {
                // Include 1-pixel outline.
                r.left -= 1;
                r.top -= 1;
                r.right += 1;
                r.bottom += 1;
            }
        }

        r
    }

    fn create_sprite(&self, light: u32, effect: Option<Effect>, tile_grid: &impl TileGridView) -> Sprite {
        let (pos, centered) = if let Some(EPoint { point: hex_pos, .. }) = self.pos {
            (tile_grid.center_to_screen(hex_pos) + self.screen_shift, true)
        } else {
            (self.screen_pos, false)
        };
        Sprite {
            pos,
            centered,
            fid: self.fid,
            frame_idx: self.frame_idx,
            direction: self.direction,
            light,
            effect,
        }
    }

    fn get_effect(&self, tile_grid: &impl TileGridView, egg: Option<Egg>) -> Option<Effect> {
        let kind = self.fid.kind();

        if kind == EntityKind::Interface {
            return None;
        }

        let with_egg =
            egg.is_some()
            // Doesn't have any translucency flags.
            && !self.has_trans()
            // Scenery or wall with position and proto.
            && (kind == EntityKind::Scenery || kind == EntityKind::Wall)
                && self.pos.is_some() && self.proto.is_some();

        if !with_egg {
            return self.get_trans_effect();
        }

        let egg = egg.unwrap();

        let pos = self.pos().point;
        let proto_flags_ext = self.proto().unwrap().flags_ext;

        let with_egg = if proto_flags_ext.intersects(
                FlagExt::WallEastOrWest | FlagExt::WallWestCorner) {
            hex::is_in_front_of(pos, egg.pos)
                && (!hex::is_to_right_of(egg.pos, pos)
                    || !self.flags.contains(Flag::WallTransEnd))
        } else if proto_flags_ext.contains(FlagExt::WallNorthCorner) {
            hex::is_in_front_of(pos, egg.pos)
                || hex::is_to_right_of(pos, egg.pos)
        } else if proto_flags_ext.contains(FlagExt::WallSouthCorner) {
            hex::is_in_front_of(pos, egg.pos)
                && hex::is_to_right_of(pos, egg.pos)
        } else if hex::is_to_right_of(pos, egg.pos) {
            !hex::is_in_front_of(egg.pos, pos)
                && !self.flags.contains(Flag::WallTransEnd)
        } else {
            false
        };

        if with_egg {
            let mask_pos = tile_grid.center_to_screen(egg.pos)/*+ self.screen_shift ??? */;
            Some(Effect::Masked { mask_fid: egg.fid, mask_pos })
        } else {
            self.get_trans_effect()
        }
    }

    fn get_trans_effect(&self) -> Option<Effect> {
        match () {
            _ if self.flags.contains(Flag::TransEnergy) => Some(Translucency::Energy),
            _ if self.flags.contains(Flag::TransGlass) => Some(Translucency::Glass),
            _ if self.flags.contains(Flag::TransRed) => Some(Translucency::Red),
            _ if self.flags.contains(Flag::TransSteam) => Some(Translucency::Steam),
            _ if self.flags.contains(Flag::TransWall) => Some(Translucency::Wall),
            _ => None,
        }.map(Effect::Translucency)
    }

    fn do_with_frame_list<F, R>(&self, frm_db: &FrameDb, f: F) -> R
        where F: FnOnce(&FrameList, &Frame) -> R
    {
        let direction = self.direction;
        let frame_idx = self.frame_idx;
        let frms = frm_db.get(self.fid).unwrap();
        let frml = &frms.frame_lists[direction];
        let frm = &frml.frames[frame_idx];
        f(frml, frm)
    }

    fn do_with_frame<F, R>(&self, frm_db: &FrameDb, f: F) -> R
        where F: FnOnce(&Frame) -> R
    {
        self.do_with_frame_list(frm_db, |_, frm| f(frm))
    }

    fn has_trans(&self) -> bool {
        self.flags.intersects(
            Flag::TransEnergy | Flag::TransGlass | Flag::TransRed | Flag::TransSteam |
            Flag::TransWall | Flag::TransNone)
    }
}

pub struct Objects {
    tile_grid: TileGrid,
    frm_db: Rc<FrameDb>,
    proto_db: Rc<ProtoDb>,
    handles: SlotMap<Handle, ()>,
    objects: SecondaryMap<Handle, RefCell<Object>>,
    // Objects attached to tile (Object::pos is Some).
    by_pos: Box<[Array2d<Vec<Handle>>]>,
    // Objects not attached to tile (Object::pos is None).
    detached: Vec<Handle>,
    empty_object_handle_vec: Vec<Handle>,
    path_finder: RefCell<PathFinder>,
    light_grid: Option<Box<LightGrid>>,
    dude: Option<Handle>,
}

impl Objects {
    pub fn new(
        tile_grid: TileGrid,
        elevation_count: u32,
        frm_db: Rc<FrameDb>,
        proto_db: Rc<ProtoDb>,
    ) -> Self {
        let path_finder = RefCell::new(PathFinder::new(tile_grid.clone(), 5000));
        let by_pos = Vec::from_fn(elevation_count as usize,
            |_| Array2d::with_default(tile_grid.width() as usize, tile_grid.height() as usize))
            .into_boxed_slice();
        let light_grid = Some(Box::new(LightGrid::new(
            tile_grid.width(),
            tile_grid.height(),
            elevation_count)));
        Self {
            tile_grid,
            frm_db,
            proto_db,
            handles: SlotMap::with_key(),
            objects: SecondaryMap::new(),
            by_pos,
            detached: Vec::new(),
            empty_object_handle_vec: Vec::new(),
            path_finder,
            light_grid,
            dude: None,
        }
    }

    pub fn elevation_count(&self) -> u32 {
        self.by_pos.len() as u32
    }

    pub fn contains(&self, obj: Handle) -> bool {
        self.objects.contains_key(obj)
    }

    pub fn clear(&mut self) {
        self.handles.clear();
        self.objects.clear();
        for elev in self.by_pos.iter_mut() {
            for v in elev.as_slice_mut() {
                *v = Vec::new();
            }
        }
        self.detached.clear();
        self.light_grid_mut().clear();
        self.dude = None;
    }

    /// If `proto` is `None`, the `fid` must be present.
    /// `rpg` is required only when the object being created is critter.
    // obj_new, but without script initialization.
    #[must_use]
    pub fn create(&mut self,
        fid: Option<FrameId>,
        proto: Option<ProtoRef>,
        pos: Option<EPoint>,
        rpg: Option<&Rpg>,
    ) -> RefMut<Object> {
        let fid = fid.or_else(|| proto.as_ref().map(|p| p.borrow().fid)).unwrap();
        let mut obj = Object::new(fid, proto, pos, SubObject::None);
        if let Some(proto_flags) = obj.proto().map(|p| p.flags) {
            let flags = proto_flags & (
                Flag::Flat |
                Flag::NoBlock |
                Flag::MultiHex |
                Flag::LightThru |
                Flag::ShootThru |
                Flag::WallTransEnd);
            obj.flags.insert(flags);

            for &flag in &[
                Flag::TransNone,
                Flag::TransEnergy,
                Flag::TransGlass,
                Flag::TransRed,
                Flag::TransSteam,
                Flag::TransWall]
            {
                if proto_flags.contains(flag) {
                    obj.flags.insert(flag);
                    break;
                }
            }
        }
        let obj = self.insert(obj);
        let mut obj = self.get_mut(obj);
        let sub = if let Some(proto) = obj.proto() {
            // proto_update_init
            match &proto.sub {
                SubProto::Critter(p) => {
                    SubObject::Critter(Critter {
                        hit_points: rpg.unwrap().stat(Stat::HitPoints, &obj, self),
                        radiation: 0,
                        poison: 0,
                        combat: CritterCombat {
                            damage_flags: Default::default(),
                            ai_packet: p.ai_packet,
                            team_id: p.team_id,
                            who_hit_me: 0,
                        },
                        dude: None,
                    })
                }
                // proto_update_gen
                SubProto::Item(p) => {
                    match &p.sub {
                        SubItem::Ammo(p) => {
                            SubObject::Item(Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: None,
                            })
                        }
                        SubItem::Key(p) => {
                            SubObject::Key(Key {
                                id: p.id,
                            })
                        }
                        SubItem::Misc(p) => {
                            SubObject::Item(Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: p.ammo_proto_id.map(|pid| self.proto_db.proto(pid).unwrap()),
                            })
                        }
                        SubItem::Weapon(p) => {
                            SubObject::Item(Item {
                                ammo_count: p.max_ammo_count,
                                ammo_proto: p.ammo_proto_id.map(|pid| self.proto_db.proto(pid).unwrap()),
                            })
                        }
                        | SubItem::Armor(_)
                        | SubItem::Container(_)
                        | SubItem::Drug(_)
                        => SubObject::None,
                    }
                }
                SubProto::Scenery(p) => {
                    match &p.sub {
                        SubScenery::Door(proto) => {
                            SubObject::Scenery(Scenery::Door(Door {
                                flags: proto.flags,
                            }))
                        }
                        SubScenery::Stairs(proto) => {
                            SubObject::Scenery(Scenery::Stairs(proto.exit.clone().unwrap()))
                        }
                        SubScenery::Elevator(proto) => {
                            SubObject::Scenery(Scenery::Elevator(Elevator {
                                kind: proto.kind,
                                level: proto.level,
                            }))
                        }
                        SubScenery::Ladder(proto) => {
                            SubObject::Scenery(Scenery::Ladder(proto.exit.clone().unwrap()))
                        }
                        SubScenery::Misc => {
                            SubObject::None
                        }
                    }
                }
                SubProto::Wall(_)
                | SubProto::SqrTile(_)
                | SubProto::Misc
                => SubObject::None,
            }
        } else {
            SubObject::None
        };
        obj.sub = sub;
        if obj.sub.as_critter().is_some() {
            rpg.unwrap().recalc_derived_stats(&mut obj, self);
        }

        if obj.is_dude() {
            obj.sub.as_critter_mut().unwrap().dude = Some(Box::new(Dude {
                naked_fidx: 0x3e,
                active_hand: Hand::Left,
            }));
        }

        // Not initializing script here.

        obj
    }

    pub fn insert(&mut self, mut obj: Object) -> Handle {
        assert!(obj.handle.is_none());

        let dude = obj.is_dude();
        let pos = obj.pos;

        let r = self.handles.insert(());
        obj.handle = Some(r);
        self.objects.insert(r, RefCell::new(obj));

        self.insert_into_tile_grid(r, pos, true);
        self.update_light_grid(r, 1);

        if dude {
            assert!(self.dude.replace(r).is_none());
            debug!("dude obj: {:?}", r);
        }

        r
    }

    pub fn remove(&mut self, obj: Handle) -> Object {
        if self.dude == Some(obj) {
            self.dude = None;
        }
        self.update_light_grid(obj, -1);
        self.handles.remove(obj).unwrap();
        self.remove_from_tile_grid(obj);
        let mut r = self.objects.remove(obj).unwrap().into_inner();
        r.handle.take().unwrap();
        r
    }

    pub fn at(&self, pos: EPoint) -> &Vec<Handle> {
        self.by_pos[pos.elevation as usize]
            .get(pos.point.x as usize, pos.point.y as usize)
            .unwrap()
    }

    pub fn get_ref(&self, h: Handle) -> &RefCell<Object> {
        &self.objects[h]
    }

    pub fn get(&self, h: Handle) -> Ref<Object> {
        self.get_ref(h).borrow()
    }

    pub fn get_mut(&self, h: Handle) -> RefMut<Object> {
        self.get_ref(h).borrow_mut()
    }

    pub fn light_grid(&self) -> &LightGrid {
        self.light_grid.as_ref().unwrap()
    }

    fn light_grid_mut(&mut self) -> &mut LightGrid {
        self.light_grid.as_mut().unwrap()
    }

    fn light_test(&self, light_test: LightTest) -> LightTestResult {
        let mut update = true;

        let dir = light_test.direction;

        for &objh in self.at(light_test.point) {
            let obj = self.get(objh);
            if obj.flags.contains(Flag::TurnedOff) {
                continue;
            }
            let block = !obj.flags.contains(Flag::LightThru);

            if obj.fid.kind() == EntityKind::Wall {
                if !obj.flags.contains(Flag::Flat) {
                    let flags_ext = obj.proto().unwrap().flags_ext;
                    if flags_ext.contains(FlagExt::WallEastOrWest) ||
                            flags_ext.contains(FlagExt::WallEastCorner) {
                        if dir != Direction::W
                                && dir != Direction::NW
                                && (dir != Direction::NE || light_test.i >= 8)
                                && (dir != Direction::SW || light_test.i <= 15) {
                            update = false;
                        }
                    } else if flags_ext.contains(FlagExt::WallNorthCorner) {
                        if dir != Direction::NE && dir != Direction::NW {
                            update = false;
                        }
                    } else if flags_ext.contains(FlagExt::WallSouthCorner) {
                        if dir != Direction::NE
                                && dir != Direction::E
                                && dir != Direction::W
                                && dir != Direction::NW
                                && (dir != Direction::SW || light_test.i <= 15) {
                            update = false;
                        }
                    } else if dir != Direction::NE
                            && dir != Direction::E
                            && (dir != Direction::NW || light_test.i <= 7) {
                        update = false;
                    }
                }
            } else if block && dir >= Direction::E && dir <= Direction::SW {
                update = false;
            }

            if block {
                return LightTestResult {
                    block,
                    update,
                }
            }
        }

        LightTestResult {
            block: false,
            update,
        }
    }

    pub fn dude(&self) -> Handle {
        self.dude.unwrap()
    }

    pub fn dude_ref(&self) -> Ref<Object> {
        self.get(self.dude())
    }

    pub fn dude_mut(&self) -> RefMut<Object> {
        self.get_mut(self.dude())
    }

    pub fn render(&self, canvas: &mut dyn Canvas, elevation: u32, screen_rect: Rect,
            tile_grid: &impl TileGridView, egg: Option<Egg>,
            get_light: impl Fn(Option<EPoint>) -> u32) {
        let get_light = &get_light;
        self.render0(canvas, elevation, screen_rect, tile_grid, egg, get_light, true);
        self.render0(canvas, elevation, screen_rect, tile_grid, egg, get_light, false);
    }

    pub fn render_outlines(&self, canvas: &mut dyn Canvas, elevation: u32, screen_rect: Rect,
            tile_grid: &impl TileGridView) {
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in hex_rect.top..hex_rect.bottom {
            for x in (hex_rect.left..hex_rect.right).rev() {
                let pos = EPoint {
                    elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos) {
                    let obj = self.get_mut(objh);
                    obj.render_outline(canvas, &self.frm_db, tile_grid);
                }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=Handle> + '_ {
        // FIXME this should come from by_pos.
        self.handles.keys()
    }

    pub fn set_pos(&mut self, h: Handle, pos: Option<EPoint>) {
        self.update_light_grid(h, -1);
        self.remove_from_tile_grid(h);
        self.insert_into_tile_grid(h, pos, true);
        self.update_light_grid(h, 1);
    }

    pub fn set_screen_shift(&mut self, h: Handle, shift: Point) {
        let pos = self.remove_from_tile_grid(h);
        self.get_mut(h).screen_shift = shift;
        self.insert_into_tile_grid(h, pos, false);
    }

    pub fn add_screen_shift(&mut self, h: Handle, shift: Point) -> Point {
        let pos = self.remove_from_tile_grid(h);
        let new_shift = {
            let mut obj = self.get_mut(h);
            obj.screen_shift += shift;
            obj.screen_shift
        };
        self.insert_into_tile_grid(h, pos, false);
        new_shift
    }

    pub fn reset_screen_shift(&mut self, h: Handle) {
        let pos = self.remove_from_tile_grid(h);
        self.insert_into_tile_grid(h, pos, true);
    }

    pub fn set_frame(&mut self, h: Handle, to: SetFrame) {
        let shift = {
            let mut obj = self.get_mut(h);
            let frame_set = self.frm_db.get(obj.fid).unwrap();
            let frames = &frame_set.frame_lists[obj.direction].frames;
            obj.frame_idx = match to {
                SetFrame::Index(v) => v,
                SetFrame::Last => frames.len() - 1,
            };
            frames.iter().take(obj.frame_idx + 1).map(|f| f.shift).sum()
        };
        self.set_screen_shift(h, shift);
    }

    // dude_stand()
    pub fn make_standing(&mut self, h: Handle) {
        let shift = {
            let mut obj = self.get_mut(h);
            let mut shift = Point::new(0, 0);
            let fid = if let FrameId::Critter(critter_fid) = obj.fid {
                if critter_fid.weapon() != WeaponKind::Unarmed {
                    let fid = critter_fid
                        .with_anim(CritterAnim::TakeOut)
                        .into();
                    let frame_set = self.frm_db.get(fid).unwrap();
                    for frame in &frame_set.frame_lists[obj.direction].frames {
                        shift += frame.shift;
                    }

                    let fid = critter_fid
                        .with_anim(CritterAnim::Stand)
                        .with_weapon(WeaponKind::Unarmed)
                        .into();
                    shift += self.frm_db.get(fid).unwrap().frame_lists[obj.direction].center;
                }
                let anim = if critter_fid.anim() == CritterAnim::FireDance {
                    CritterAnim::FireDance
                } else {
                    CritterAnim::Stand
                };
                critter_fid
                    .with_anim(anim)
                    .into()
            } else {
                obj.fid
            };
            obj.fid = fid;
            obj.frame_idx = 0;
            shift
        };
        self.set_screen_shift(h, shift);
    }

    /// Whether the `pos` hex has any blocker objects other than `excluding_obj`.
    // obj_blocking_at()
    #[must_use]
    pub fn has_blocker_at(&self, pos: EPoint, excluding_obj: Option<Handle>) -> bool {
        let check = |h| {
            if Some(h) == excluding_obj {
                return false;
            }
            let o = self.get(h);
            match o.fid.kind() {
                | EntityKind::Critter
                | EntityKind::Scenery
                | EntityKind::Wall
                => {},
                _ => return false,
            }
            if o.flags.contains(Flag::TurnedOff) || o.flags.contains(Flag::NoBlock) {
                return false;
            }
            true
        };
        for &objh in self.at(pos) {
            if check(objh) {
                return true;
            }
        }
        for dir in Direction::iter() {
            if let Some(near) = self.tile_grid.go(pos.point, dir, 1) {
                for &objh in self.at(near.elevated(pos.elevation)) {
                    if self.get(objh).flags.contains(Flag::MultiHex) &&
                        check(objh)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Returns `true` if there's object that would block sight from `obj` through tile at `pos`.
    // obj_sight_blocking_at()
    #[must_use]
    pub fn is_sight_blocked_at(&self, obj: Handle, pos: EPoint) -> bool {
        for &h in self.at(pos) {
            let o = &self.get(h);
            if !o.flags.contains(Flag::TurnedOff) &&
                !o.flags.contains(Flag::LightThru) &&
                (o.kind() == EntityKind::Scenery || o.kind() == EntityKind::Wall) &&
                h != obj
            {
                return true;
            }
        }
        false
    }

    // obj_shoot_blocking_at()
    #[must_use]
    pub fn shot_blocker_at(&self, obj: Handle, pos: EPoint) -> Option<Handle> {
        let check = |pos, multi_hex_only| {
            for &h in self.at(pos) {
                let o = &self.get(h);
                if multi_hex_only && !o.flags.contains(Flag::MultiHex) {
                    return None;
                }
                let non_shoot_thru = if multi_hex_only {
                    // TODO For multi-hex ShootThru flag doesn't apply?
                    false
                } else {
                    !o.flags.contains(Flag::ShootThru)
                };
                if !o.flags.contains(Flag::TurnedOff) &&
                    (!o.flags.contains(Flag::NoBlock) || non_shoot_thru) &&
                    h != obj &&
                    match o.kind() {
                        EntityKind::Scenery | EntityKind::Wall | EntityKind::Critter => true,
                        _ => false,
                    }
                {
                    return Some(h);
                }
            }
            None
        };

        let r = check(pos, false);
        if r.is_some() {
            return r;
        }

        // Check for MultiHex objects in neighbor tiles.
        for dir in Direction::iter() {
            if let Some(p) = self.tile_grid.go(pos.point, dir, 1) {
                let r = check(p.elevated(pos.elevation), true);
                if r.is_some() {
                    return r;
                }
            }
        }

        None
    }

    // combat_is_shot_blocked()
    #[must_use]
    pub fn is_shot_blocked(&self, shooter: Handle, target: Handle) -> bool {
        let pos = self.get(shooter).pos();
        let target_pos = self.get(target).pos();
        assert_eq!(pos.elevation, target_pos.elevation);
        let mut last_blocker = None;
        for p in hex::ray(pos.point, target_pos.point) {
            let blocker = self.shot_blocker_at(shooter, p.elevated(pos.elevation));

            if_chain! {
                if blocker != last_blocker;
                if let Some(blocker) = blocker;
                then {
                    if blocker != shooter && blocker != target {
                        let o = self.get(blocker);
                        if o.kind() != EntityKind::Critter {
                            return true;
                        }
                    }
                    last_blocker = Some(blocker);
                }
            };
            if p == target_pos.point {
                break;
            }
        }
        false
    }

    /// Based on spatial information are the objects able to talk?
    /// Objects can talk if:
    /// 1. There's a path between them which is not sight-blocked (see `sight_blocker_for_object()`).
    /// 2. Screen distance between objects is within the limit.
    // action_can_talk_to()
    pub fn can_talk(&self, obj1: Handle, obj2: Handle) -> Result<(), CantTalkSpatial> {
        let o1 = self.get(obj1);
        let o2 = self.get(obj2);

        // TODO maybe return Unreachable error instead.
        let p1 = o1.pos();
        let p2 = o2.pos();

        if p1.elevation != p2.elevation {
            return Err(CantTalkSpatial::Unreachable);
        }

        if hex::distance(p1.point, p2.point) > 12 {
            return Err(CantTalkSpatial::TooFar);
        }

        let reachable = self.path_finder.borrow_mut().find(p1.point, p2.point, true,
            |p| {
                let p = EPoint::new(p1.elevation, p);
                if self.is_sight_blocked_at(obj1, p) {
                    TileState::Blocked
                } else {
                    TileState::Passable(0)
                }
            }).is_some();
        if reachable {
            Ok(())
        } else {
            Err(CantTalkSpatial::Unreachable)
        }
    }

    // can_talk_to
    #[must_use]
    pub fn can_talk_now(&self, obj1: Handle, obj2: Handle) -> bool {
        self.distance(obj1, obj2).unwrap() < 9 && !self.is_shot_blocked(obj1, obj2)
    }

    // action_can_be_pushed()
    #[must_use]
    pub fn can_push(&self, pusher: Handle, pushed: Handle, scripts: &Scripts,
        in_combat: bool) -> bool
    {
        let pushedo = self.get(pushed);
        if pushedo.kind() != EntityKind::Critter
            || pusher == pushed
            || !pushedo.sub.as_critter().unwrap().is_active()
            || self.can_talk(pusher, pushed).is_err()
            || pushedo.script.is_none()
        {
            return false;
        }
        let (sid, _) = pushedo.script.unwrap();
        if !scripts.has_predefined_proc(sid, PredefinedProc::Push) {
            return false;
        }
        if in_combat {
            unimplemented!("TODO")
//            pushed_team_num = pushed->_._.critter.combat_data.team_num;
//          pushed_ = &pushed->_._;
//          if ( pushed_team_num == pusher->_._.critter.combat_data.team_num
//            && pusher == pushed_->critter.combat_data.who_hit_me )
//          {
//            return 0;
//          }
//          v7 = pushed_->critter.combat_data.who_hit_me;
//          if ( v7 && v7->_._.critter.combat_data.team_num == pusher->_._.critter.combat_data.team_num )
//            result = 0;
//          else
//            result = 1;
        }
        true
    }

    /// `allow_neighbor_tile` - allows constructing path to a neighbor tile of `to` tile if the
    /// target tile is blocked.
    #[must_use]
    pub fn path(&self,
        obj: Handle,
        to: PathTo,
        smooth: bool)
        -> Option<Vec<Direction>>
    {
        let o = self.get(obj);
        let from = o.pos?;

        let (to_point, unblocked_radius) = match to {
            PathTo::Object(to_obj) => {
                let to_obj = self.get(to_obj);
                let to_point = to_obj.pos().point;
                let unblocked_radius = if to_obj.flags.contains(Flag::MultiHex) {
                    2
                } else {
                    1
                };
                (to_point, unblocked_radius)
            }
            PathTo::Point { point, neighbor_if_blocked } => {
                let unblocked_radius = if neighbor_if_blocked && self.has_blocker_at(point.elevated(from.elevation), Some(obj)) {
                    1
                } else {
                    0
                };
                (point, unblocked_radius)
            }
        };

        let mut r = self.path_finder.borrow_mut().find(from.point, to_point, smooth,
            |p| {
                let p = p.elevated(from.elevation);
                let blocked =
                    // p is not in unblocked_radius
                    hex::try_distance(p.point, to_point, unblocked_radius).map(|d| d < unblocked_radius) != Some(true) &&
                    self.has_blocker_at(p, Some(obj));
                if blocked {
                    // TODO check anim_can_use_door_(obj, v22)
                    TileState::Blocked
                } else if let Some(proto) = o.proto.as_ref() {
                    let radioactive_goo = self.at(p)
                        .iter()
                        .any(|&h| self.get(h).proto_id()
                            .map(|pid| pid.is_radioactive_goo())
                            .unwrap_or(false));
                    let cost = if radioactive_goo {
                        let gecko = if let SubProto::Critter(ref c) = proto.borrow().sub {
                            c.kill_kind == CritterKillKind::Gecko
                        } else {
                            false
                        };
                        if gecko {
                            100
                        } else {
                            400
                        }
                    } else {
                        0
                    };

                    TileState::Passable(cost)
                } else {
                    TileState::Passable(0)
                }
            });
        if let Some(path) = r.as_mut() {
            let l = path.len() - unblocked_radius as usize;
            path.truncate(l);
        }
        r
    }

    pub fn bounds(&self, obj: Handle, tile_grid: &impl TileGridView,
        include_outline: bool,
    ) -> Rect {
        self.get(obj).bounds(&self.frm_db, tile_grid, include_outline)
    }

    pub fn hit_test(&self, p: EPoint, screen_rect: Rect, tile_grid: &impl TileGridView,
        egg: Option<Egg>) -> Vec<(Handle, Hit)>
    {
        let mut r = Vec::new();
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in (hex_rect.top..hex_rect.bottom).rev() {
            for x in hex_rect.left..hex_rect.right {
                let pos = EPoint {
                    elevation: p.elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos).iter().rev() {
                    let obj = self.get(objh);

                    let mut hit = if let Some(hit) = obj.hit_test(p.point, &self.frm_db, tile_grid) {
                        hit
                    } else {
                        continue;
                    };

                    if let Some(egg) = egg {
                        if self.is_egg_hit(p.point, &*obj, egg, tile_grid) {
                            hit.with_egg = true;
                        }
                    }

                    r.push((objh, hit));
                }
            }
        }
        r
    }

    pub fn distance(&self, from: Handle, to: Handle) -> Option<u32> {
        self.get(from).distance(&self.get(to))
    }

    // item_add_force
    pub fn move_into_inventory(&mut self, inventory: Handle, item: Handle, count: u32) {
        assert!(count > 0);
        let existing_objh = {
            let mut inventory = self.get_mut(inventory);
            let existing_idx = {
                let itemo = &self.get(item);
                inventory.inventory.items.iter()
                    .position(|i| self.get(i.object).is_same(itemo))
            };
            if let Some(existing_idx) = existing_idx {
                let existing_objh = inventory.inventory.items[existing_idx].object;
                if existing_objh == item {
                    warn!("ignoring exact duplicate item in World::inventory_insert(): inventory={:?} item={:?}",
                        inventory, item);
                    return;
                }

                let existing_obj = self.get(existing_objh);
                let proto = existing_obj.proto().unwrap();
                if proto.kind() == ExactEntityKind::Item(ItemKind::Ammo) {
                    let mut item = self.get_mut(item);
                    let max_ammo_count = proto.max_ammo_count().unwrap();
                    let combined_ammo = existing_obj.ammo_count().unwrap() + item.ammo_count().unwrap();
                    let full_clips = combined_ammo / max_ammo_count;
                    let ammo = combined_ammo % max_ammo_count;
                    item.sub.as_item_mut().unwrap().ammo_count = ammo;
                    inventory.inventory.items[existing_idx].count += count - 1 + full_clips;
                } else {
                    inventory.inventory.items[existing_idx].count += count;
                }
                inventory.inventory.items[existing_idx].object = item;
                Some(existing_objh)
            } else {
                inventory.inventory.items.insert(0, InventoryItem {
                    object: item,
                    count,
                });
                None
            }
        };
        if let Some(existing) = existing_objh {
            self.remove(existing);
        }
        self.set_pos(item, None);
    }

    // item_w_unload
    pub fn unload_weapon(&mut self, weapon: Handle) -> Option<Handle> {
        let (ammo_proto, count) = {
            let mut weapon = self.get_mut(weapon);
            if !weapon.is_unloadable_weapon() {
                return None;
            }
            let weapon = weapon.sub.as_item_mut().unwrap();
            weapon.ammo_proto.as_ref()?;
            let count = std::mem::replace(&mut weapon.ammo_count, 0);
            if count == 0 {
                return None;
            }
            (weapon.ammo_proto.clone().unwrap(), count)
        };
        // TODO Original here allows weapon to have more ammo loaded than ammo clip capacity.
        assert!(count <= ammo_proto.borrow().sub.as_ammo().unwrap().max_ammo_count);
        let mut ammo = self.create(None, Some(ammo_proto), None, None);
        ammo.sub.as_item_mut().unwrap().ammo_count = count;
        Some(ammo.handle())
    }

    pub fn reload_weapon_from_inventory(&mut self,
        owner: Handle,
        weapon: Handle,
        ammo: Handle,
    ) {
        let remove = {
            let owner = &mut self.get_mut(owner);
            let weapon = &mut self.get_mut(weapon);
            let ammo = &mut self.get_mut(ammo);
            let idx = owner.inventory.items.iter()
                .position(|i| i.object == ammo.handle())
                .unwrap();
            let left = unwrap_or_return!(weapon.reload_weapon(ammo), Some);
            if left == 0 && owner.inventory.items[idx].count - 1 == 0 {
                owner.inventory.items.remove(idx);
                true
            } else {
                false
            }
        };
        if remove {
            self.remove(ammo);
        }
    }

    // obj_intersects_with()
    #[must_use]
    fn is_egg_hit(&self, p: Point, obj: &Object, egg: Egg, tile_grid: &impl TileGridView) -> bool {
        if_chain! {
            if let Some(obj_pos) = obj.pos;
            let obj_pos = obj_pos.point;
            if let Some(proto) = obj.proto.as_ref();
            let proto = proto.borrow();
            if proto.id().kind() == EntityKind::Wall || proto.id().kind() == EntityKind::Scenery;
            then {
                if !egg.hit_test(p, tile_grid, &self.frm_db) {
                    return false;
                }

                // Is masked?
                if proto.flags_ext.intersects(
                    FlagExt::WallEastOrWest | FlagExt::WallWestCorner)
                {
                    hex::is_in_front_of(obj_pos, egg.pos)
                } else if proto.flags_ext.contains(FlagExt::WallNorthCorner) {
                    hex::is_in_front_of(obj_pos, egg.pos) ||
                        hex::is_to_right_of(obj_pos, egg.pos)
                } else if proto.flags_ext.contains(FlagExt::WallSouthCorner) {
                    hex::is_in_front_of(obj_pos, egg.pos) &&
                        hex::is_to_right_of(obj_pos, egg.pos)
                } else {
                    hex::is_to_right_of(obj_pos, egg.pos)
                }
            } else {
                false
            }
        }
    }

    fn get_render_hex_rect(screen_rect: Rect, tile_grid: &impl TileGridView) -> Rect {
        tile_grid.from_screen_rect(Rect {
            left: -320,
            top: -190,
            right: screen_rect.width() + 320,
            bottom: screen_rect.height() + 190
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn render0(&self, canvas: &mut dyn Canvas, elevation: u32,
            screen_rect: Rect, tile_grid: &impl TileGridView, egg: Option<Egg>,
            get_light: impl Fn(Option<EPoint>) -> u32,
            flat: bool) {
        let hex_rect = Self::get_render_hex_rect(screen_rect, tile_grid);
        for y in hex_rect.top..hex_rect.bottom {
            for x in (hex_rect.left..hex_rect.right).rev() {
                let pos = EPoint {
                    elevation,
                    point: Point::new(x, y),
                };
                for &objh in self.at(pos) {
                    let mut obj = self.get_mut(objh);
                    if flat && !obj.flags.contains(Flag::Flat) {
                        break;
                    } else if !flat && obj.flags.contains(Flag::Flat) {
                        continue;
                    }
                    let light = get_light(obj.pos);
                    assert!(light <= 0x10000);
                    obj.render(canvas, light, &self.frm_db, tile_grid, egg);
                }
            }
        }
    }

    fn at_mut(&mut self, pos: EPoint) -> &mut Vec<Handle> {
        self.by_pos[pos.elevation as usize]
            .get_mut(pos.point.x as usize, pos.point.y as usize)
            .unwrap()
    }

    fn cmp_objs(&self, o1: &Object, o2: &Object) -> cmp::Ordering {
        assert_eq!(o1.pos().elevation, o2.pos().elevation);

        // By flatness, flat first.
        let flat = o1.flags.contains(Flag::Flat);
        let other_flat = o2.flags.contains(Flag::Flat);
        if flat != other_flat {
            return if flat {
                cmp::Ordering::Less
            } else {
                cmp::Ordering::Greater
            };
        }


        let shift = o1.screen_shift + o1.do_with_frame(&self.frm_db, |frm| frm.shift);
        let other_shift = o2.screen_shift + o2.do_with_frame(&self.frm_db, |frm| frm.shift);

        // By shift_y, less first.
        if shift.y < other_shift.y {
            return cmp::Ordering::Less;
        }
        if shift.y > other_shift.y {
            return cmp::Ordering::Greater;
        }

        // By shift_x, less first.
        shift.x.cmp(&other_shift.x)
    }

    fn insert_into_tile_grid(&mut self, h: Handle, pos: Option<EPoint>, reset_screen_shift: bool) {
        if let Some(pos) = pos {
            {
                let mut obj = self.get_mut(h);
                obj.pos = Some(pos);
                if reset_screen_shift {
                    obj.screen_shift = Point::new(0, 0);
                }
            }

            let i = {
                let list = self.at(pos);
                let obj = self.get(h);
                match list.binary_search_by(|&h| {
                    let o = self.get(h);
                    self.cmp_objs(&o, &obj)
                }) {
                    Ok(mut i) =>  {
                        // Append to the current group of equal objects.
                        while i < list.len()
                            && self.cmp_objs(&obj, &self.get(list[i])) == cmp::Ordering::Equal
                        {
                            i += 1;
                        }
                        i
                    }
                    Err(i) => i,
                }
            };
            self.at_mut(pos).insert(i, h);
        } else {
            self.detached.push(h);
        }
    }

    fn remove_from_tile_grid(&mut self, h: Handle) -> Option<EPoint> {
        let old_pos = mem::replace(&mut self.get_mut(h).pos, None);
        let list = if let Some(old_pos) = old_pos {
            self.at_mut(old_pos)
        } else {
            &mut self.detached
        };
        // TODO maybe use binary_search for detaching.
        list.retain(|&hh| hh != h);
        old_pos
    }

    fn update_light_grid(&mut self, obj: Handle, factor: i32) {
        let (pos, light_emitter) = {
            let obj = self.get(obj);
            (obj.pos, obj.light_emitter)
        };
        if let Some(pos) = pos {
            let mut light_grid = self.light_grid.take().unwrap();
            self.update_light_grid0(&mut light_grid, pos, light_emitter, factor);
            self.light_grid = Some(light_grid);
        }
    }

    fn update_light_grid0(&self,
        light_grid: &mut LightGrid,
        pos: EPoint,
        light_emitter: LightEmitter,
        factor: i32,
    ) {
        light_grid.update(pos,
                    light_emitter.radius,
                    factor * light_emitter.intensity as i32,
                    |lt| self.light_test(lt));
    }

    pub fn rebuild_light_grid(&mut self) {
        let mut light_grid = self.light_grid.take().unwrap();
        light_grid.clear();
        for (_, obj) in self.objects.iter() {
            let obj = obj.borrow();
            if let Some(pos) = obj.pos {
                self.update_light_grid0(&mut light_grid, pos, obj.light_emitter, 1);
            }
        }
        self.light_grid = Some(light_grid);
    }
}

#[derive(Debug, enum_as_inner::EnumAsInner)]
pub enum SubObject {
    None,
    Critter(Critter),
    Item(Item),
    Key(Key),
    MapExit(MapExit),
    Scenery(Scenery),
}

#[derive(Debug)]
pub struct Dude {
    pub naked_fidx: Idx,
    pub active_hand: Hand,
}

#[derive(Debug)]
pub struct Critter {
    pub hit_points: i32,
    pub radiation: i32,
    pub poison: i32,
    pub combat: CritterCombat,
    pub dude: Option<Box<Dude>>,
}

impl Critter {
    // critter_is_active()
    pub fn is_active(&self) -> bool {
        !self.combat.damage_flags.intersects(
            DamageFlag::LoseTurn |
            DamageFlag::Dead |
            DamageFlag::KnockedOut)
    }

    // critter_is_dead()
    pub fn is_dead(&self) -> bool {
        self.combat.damage_flags.contains(DamageFlag::Dead)
        // TODO
//        if ( stat_level_(result, STAT_current_hp) <= 0 )
//      return 1;
    }

    pub fn dude(&self) -> &Dude {
        self.try_dude().unwrap()
    }

    pub fn dude_mut(&mut self) -> &mut Dude {
        self.try_dude_mut().unwrap()
    }

    pub fn try_dude(&self) -> Option<&Dude> {
        self.dude.as_deref()
    }

    pub fn try_dude_mut(&mut self) -> Option<&mut Dude> {
        self.dude.as_deref_mut()
    }
}

#[derive(Debug, Default)]
pub struct CritterCombat {
    pub damage_flags: BitFlags<DamageFlag>,
    pub ai_packet: i32,
    pub team_id: i32,
    pub who_hit_me: i32,
}

#[derive(Clone, Copy, Debug, EnumFlags, Primitive)]
#[repr(u32)]
pub enum DamageFlag {
  KnockedOut = 0x1,
  KnockedDown = 0x2,
  CripLegLeft = 0x4,
  CripLegRight = 0x8,
  CripArmLeft = 0x10,
  CripArmRight = 0x20,
  Blind = 0x40,
  Dead = 0x80,
  Hit = 0x100,
  Critical = 0x200,
  OnFire = 0x400,
  Bypass = 0x800,
  Explode = 0x1000,
  Destroy = 0x2000,
  Drop = 0x4000,
  LoseTurn = 0x8000,
  HitSelf = 0x10000,
  LoseAmmo = 0x20000,
  Dud = 0x40000,
  HurtSelf = 0x80000,
  RandomHit = 0x100000,
  CripRandom = 0x200000,
  Backwash = 0x400000,
  PerformReverse = 0x800000,
}

#[derive(Debug, enum_as_inner::EnumAsInner)]
pub enum Scenery {
    Door(Door),
    Elevator(Elevator),
    Ladder(MapExit),
    Stairs(MapExit),
}

#[derive(Debug, Default)]
pub struct Door {
    pub flags: BitFlags<DoorFlag>,
}

#[derive(Clone, Copy, Debug, EnumFlags)]
#[repr(u32)]
pub enum UpdatedFlag {
    Locked = 0x2_00_00_00,
    Jammed = 0x4_00_00_00,
}

#[derive(Debug, Default)]
pub struct Elevator {
    pub kind: u32,
    pub level: u32,
}

#[derive(Debug)]
pub struct Item {
    pub ammo_count: u32,
    // TODO When can this be different from Weapon::ammo_proto_id?
    pub ammo_proto: Option<ProtoRef>,
}

#[derive(Debug)]
pub struct Key {
    pub id: i32,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::graphics::geometry::hex::View;

    #[test]
    fn bounds() {
        let screen_shift = Point::new(10, 20);
        let base = Point::new(2384, 468) + screen_shift;

        let mut obj = Object::new(FrameId::BLANK, None, Some((0, (55, 66)).into()), SubObject::None);
        obj.screen_shift = screen_shift;
        assert_eq!(obj.bounds0(Point::new(-1, 3), Point::new(29, 63), &View::default(), true),
            Rect::with_points(Point::new(1, -51), Point::new(30, 12))
                .translate(base));
    }
}
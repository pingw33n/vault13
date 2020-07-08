use bstring::{bstr, BString};
use bstring::bfmt::ToBString;
use if_chain::if_chain;
use sdl2::mouse::MouseButton;
use std::time::Duration;

use crate::asset::*;
use crate::asset::frame::FrameId;
use crate::asset::message::{Messages, MessageId};
use crate::fs::FileSystem;
use crate::game::object::{self, EquipmentSlot, Hand, Object, InventoryItem};
use crate::game::rpg::Rpg;
use crate::game::ui::action_menu::{self, Action};
use crate::game::ui::inventory_list::{self, InventoryList, Scroll, MouseMode};
use crate::game::ui::move_window::MoveWindow;
use crate::game::world::WorldRef;
use crate::graphics::{Point, Rect};
use crate::graphics::color::{GREEN, RED};
use crate::graphics::font::*;
use crate::graphics::sprite::{Anchor, Sprite};
use crate::sequence::{Sequence, Sequencer};
use crate::sequence::cancellable::Cancel;
use crate::ui::{self, Ui, button, Widget, HandleEvent, Cursor};
use crate::ui::button::Button;
use crate::ui::command::{move_window, UiCommand, UiCommandData};
use crate::ui::command::inventory::Command;
use crate::ui::panel::{self, Panel};
use crate::ui::sequence::background_anim::BackgroundAnim;
use crate::util::sprintf;

const MSG_NO_ITEM: MessageId = 14;
const MSG_DMG: MessageId = 15;
const MSG_RNG: MessageId = 16;
const MSG_AMMO: MessageId = 17;
const MSG_NOT_WORN: MessageId = 18;
const MSG_TOTAL_WEIGHT: MessageId = 20;
const MSG_UNARMED_DMG: MessageId = 24;

pub struct Inventory {
    msgs: Option<Messages>,
    world: WorldRef,
    internal: Option<Internal>
}

impl Inventory {
    pub fn new(world: WorldRef, fs: &FileSystem, language: &str) -> Self {
        let msgs = Some(Messages::read_file(fs, language, "game/inventry.msg").unwrap());
        Self {
            msgs,
            world,
            internal: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.internal.is_some()
    }

    pub fn handle(&mut self, cmd: UiCommand, rpg: &Rpg, ui: &mut Ui, ui_sequencer: &mut Sequencer) {
        if let UiCommandData::Inventory(c) = cmd.data {
            match c {
                Command::Show => {
                    self.show(rpg, ui, ui_sequencer);
                }
                Command::Hide => {
                    self.hide(ui);
                }
                _ => {}
            }
        }
        if let Some(v) = self.internal.as_mut() {
            v.handle(cmd, rpg, ui);
        }
    }

    pub fn examine(&self, obj: object::Handle, description: &bstr, ui: &Ui) {
        self.internal.as_ref().unwrap().examine(obj, description, ui);
    }

    fn show(&mut self, rpg: &Rpg, ui: &mut Ui, ui_sequencer: &mut Sequencer) {
        let owner = self.world.borrow().objects().dude();
        let internal = Internal::new(
            self.msgs.take().unwrap(), self.world.clone(), owner, ui, ui_sequencer);
        internal.sync_mouse_mode_to_ui(ui);
        internal.sync_to_ui(rpg, ui);
        assert!(self.internal.replace(internal).is_none());
    }

    fn hide(&mut self, ui: &mut Ui) {
        let i = self.internal.take().unwrap();
        self.msgs = Some(i.hide(ui));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Slot {
    Equipment(EquipmentSlot),
    Inventory,
}

struct Internal {
    msgs: Messages,
    world: WorldRef,
    owner: object::Handle,
    win: ui::Handle,
    mouse_mode: MouseMode,
    list: ui::Handle,
    list_scroll_up: ui::Handle,
    list_scroll_down: ui::Handle,
    wearing: ui::Handle,
    left_hand: ui::Handle,
    right_hand: ui::Handle,
    item_descr: ui::Handle,

    stat_misc: ui::Handle,
    /// Four columns of stats:
    /// ST   9   Hit Points   10/20
    stat_columns: [ui::Handle; 4],
    total_weight: ui::Handle,

    action_menu: Option<ui::Handle>,
    move_window: Option<InventoryMoveWindow>,

    owner_image: ui::Handle,
    owner_image_seq: Cancel,
}

impl Internal {
    fn new(
        msgs: Messages,
        world: WorldRef,
        owner: object::Handle,
        ui: &mut Ui,
        ui_sequencer: &mut Sequencer,
    ) -> Self {
        let win = ui.new_window(Rect::with_size(80, 0, 499, 377),
            Some(Sprite::new(FrameId::INVENTORY_WINDOW)));
        ui.widget_base_mut(win).set_modal(true);

        let mut owner_image = Sprite::new(FrameId::BLANK);
        owner_image.anchor = Anchor::Center;
        let owner_image = ui.new_widget(win, Rect::with_size(175, 35, 60, 100), None,
            Some(owner_image), Panel::new());

        let (seq, owner_image_seq) =
            BackgroundAnim::new(owner_image, Duration::from_millis(182)).cancellable();
        ui_sequencer.start(seq);

        let mut list_scroll_up = Button::new(FrameId::INVENTORY_SCROLL_UP_UP,
            FrameId::INVENTORY_SCROLL_UP_DOWN,
            Some(UiCommandData::Inventory(Command::Scroll(Scroll::Up))));
        list_scroll_up.config_mut(button::State::Disabled).background =
            Some(Sprite::new(FrameId::INVENTORY_SCROLL_UP_DISABLED));
        let list_scroll_up = ui.new_widget(win, Rect::with_size(128, 39, 22, 23),
            None, None, list_scroll_up);

        let mut list_scroll_down = Button::new(FrameId::INVENTORY_SCROLL_DOWN_UP,
            FrameId::INVENTORY_SCROLL_DOWN_DOWN,
            Some(UiCommandData::Inventory(Command::Scroll(Scroll::Down))));
        list_scroll_down.config_mut(button::State::Disabled).background =
            Some(Sprite::new(FrameId::INVENTORY_SCROLL_DOWN_DISABLED));
        let list_scroll_down = ui.new_widget(win, Rect::with_size(128, 62, 22, 23),
            None, None, list_scroll_down);

        fn text_panel(horz_align: HorzAlign) -> Panel {
            let mut p = Panel::new();
            p.set_text(Some(panel::Text {
                text: "".into(),
                font: FontKey::antialiased(1),
                color: GREEN,
                options: DrawOptions {
                    horz_align,
                    ..Default::default()
                },
            }));
            p
        }

        let mut item_descr = text_panel(HorzAlign::Left);
        item_descr.text_mut().unwrap().options.horz_overflow = Some(Overflow {
            size: 0,
            boundary: OverflowBoundary::Word,
            action: OverflowAction::Wrap,
        });
        let item_descr = ui.new_widget(win, Rect::with_size(297, 54, 158, 193), None, None,
            item_descr);

        let x = 297;
        let mut y = 44;

        let mut stat_misc = text_panel(HorzAlign::Left);
        stat_misc.text_mut().unwrap().options.horz_overflow = Some(Overflow {
            size: 0,
            boundary: OverflowBoundary::Word,
            action: OverflowAction::Truncate,
        });
        let stat_misc = ui.new_widget(win, Rect::with_size(x, y, 150, 1), None, None,
            stat_misc);

        y += 18;
        let stat_columns = [
            ui.new_widget(win, Rect::with_size(x, y, 1, 1), None, None, text_panel(HorzAlign::Left)),
            ui.new_widget(win, Rect::with_size(x + 24, y, 1, 1), None, None, text_panel(HorzAlign::Left)),
            ui.new_widget(win, Rect::with_size(x + 40, y, 1, 1), None, None, text_panel(HorzAlign::Left)),
            ui.new_widget(win, Rect::with_size(x + 105, y, 1, 1), None, None, text_panel(HorzAlign::Left)),
        ];
        let total_weight = ui.new_widget(win, Rect::with_size(x, y + 160, 150, 1),
            None, None, text_panel(HorzAlign::Center));

        let _done = ui.new_widget(win, Rect::with_size(437, 329, 15, 16), None, None,
            Button::new(FrameId::SMALL_RED_BUTTON_UP, FrameId::SMALL_RED_BUTTON_DOWN,
                Some(UiCommandData::Inventory(Command::Hide))));

        // The order of lists relative to other widgets is important because the action icon is drawn
        // by the widget itself.
        // TODO Remove the above comment once https://trello.com/c/15GY3Deb is done
        let list = ui.new_widget(win, Rect::with_size(48, 40, 56, 50 * 6), None, None,
            InventoryList::new(40, 10));
        let wearing = ui.new_widget(win, Rect::with_size(154, 183, 90, 61), None, None,
            InventoryList::new(61, 0));
        let right_hand = ui.new_widget(win, Rect::with_size(245, 289, 90, 61), None, None,
            InventoryList::new(61, 0));
        let left_hand = ui.new_widget(win, Rect::with_size(154, 289, 90, 61), None, None,
            InventoryList::new(61, 0));

        let mouse_mode_toggler = ui.new_widget(win, Rect::with_size(0, 0, 1, 1), None, None,
            MouseModeToggler);
        let mut mouse_mode_toggler = ui.widget_base_mut(mouse_mode_toggler);
        mouse_mode_toggler.set_visible(false);
        mouse_mode_toggler.set_listener(true);

        Self {
            msgs,
            world,
            owner,
            win,
            mouse_mode: MouseMode::Drag,
            list,
            list_scroll_up,
            list_scroll_down,
            wearing,
            left_hand,
            right_hand,
            item_descr,
            stat_misc,
            stat_columns,
            total_weight,
            action_menu: None,
            move_window: None,
            owner_image,
            owner_image_seq,
        }
    }

    fn hide(self, ui: &mut Ui) -> Messages {
        ui.remove(self.win);
        self.owner_image_seq.cancel();
        self.msgs
    }

    fn sync_to_ui(&self, rpg: &Rpg, ui: &Ui) {
        let list = &mut ui.widget_mut::<InventoryList>(self.list);
        let wearing = &mut ui.widget_mut::<InventoryList>(self.wearing);
        let left_hand = &mut ui.widget_mut::<InventoryList>(self.left_hand);
        let right_hand = &mut ui.widget_mut::<InventoryList>(self.right_hand);

        let list_scroll_idx = list.scroll_idx();
        list.clear();

        wearing.clear();
        left_hand.clear();
        right_hand.clear();

        let world = self.world.borrow();
        let owner = world.objects().get(self.owner);
        for item in &owner.inventory.items {
            let item_obj = &world.objects().get(item.object);
            let inv_list_item = Self::make_list_item(item, item_obj);
            match () {
                _ if item_obj.flags.contains(Flag::Worn) => {
                    assert!(wearing.items().is_empty());
                    wearing.push(inv_list_item);
                }
                _ if item_obj.flags.contains(Flag::LeftHand) => {
                    assert!(left_hand.items().is_empty());
                    left_hand.push(inv_list_item);
                }
                _ if item_obj.flags.contains(Flag::RightHand) => {
                    assert!(right_hand.items().is_empty());
                    right_hand.push(inv_list_item);
                }
                _ => list.push(inv_list_item),
            }
        }

        list.set_scroll_idx(list_scroll_idx);
        self.update_list_scroll_buttons(list, ui);
        self.update_stats(rpg, ui);

        ui.widget_base_mut(self.owner_image).background_mut().unwrap().fid = owner.fid;
    }

    // display_inventory_info
    fn make_list_item(item: &InventoryItem, obj: &Object) -> inventory_list::Item {
        let proto = obj.proto().unwrap();
        let count = obj.total_ammo_count(item.count).unwrap_or(item.count);
        inventory_list::Item {
            object: item.object,
            fid: proto.sub.as_item().unwrap().inventory_fid.unwrap(),
            count,
        }
    }

    fn scroll(&self, scroll: Scroll, ui: &Ui) {
        let list = &mut ui.widget_mut::<InventoryList>(self.list);
        list.scroll(scroll);
        self.update_list_scroll_buttons(list, ui);
    }

    fn update_list_scroll_buttons(&self, list: &InventoryList, ui: &Ui) {
        ui.widget_mut::<Button>(self.list_scroll_up).set_enabled(list.can_scroll(Scroll::Up));
        ui.widget_mut::<Button>(self.list_scroll_down).set_enabled(list.can_scroll(Scroll::Down));
    }

    fn actions(&self, object: object::Handle) -> Vec<(Action, UiCommandData)> {
        let mut r = Vec::new();
        let world = self.world.borrow();
        let obj = world.objects().get(object);

        r.push(Action::Look);
        if obj.is_unloadable_weapon() {
            r.push(Action::Unload);
        }
        // TODO handle containers: https://fallout.fandom.com/wiki/Bag
        if obj.can_use() || obj.proto().unwrap().can_use_on() {
            r.push(Action::UseHand);
        }
        r.extend_from_slice(&[Action::Drop, Action::Cancel]);

        r.iter()
            .map(|&action| (action, UiCommandData::Inventory(
                Command::Action { object, action: Some(action) })))
            .collect()
    }

    fn show_action_menu(&mut self, obj: object::Handle, ui: &mut Ui) {
        let actions = self.actions(obj);
        assert!(self.action_menu.replace(action_menu::show(actions, self.win, ui)).is_none());
    }

    fn hide_action_menu(&mut self, ui: &mut Ui) {
        if let Some(v) = self.action_menu.take() {
            action_menu::hide(v, ui);
        }
    }

    // display_stats
    fn update_stats(&self, rpg: &Rpg, ui: &Ui) {
        let world = self.world.borrow();
        let name = world.object_name(self.owner).unwrap();
        let owner = &world.objects().get(self.owner);

        let stat = |stat| rpg.stat(stat, owner, world.objects());
        let msg = |id| &self.msgs.get(id).unwrap().text;

        let mut cols = [BString::new(), BString::new(), BString::new(), BString::new()];

        for (i, s) in Stat::base().iter().copied().enumerate() {
            cols[0].push_str(msg(i as MessageId));
            cols[0].push(b'\n');
            cols[1].push_str(stat(s).to_bstring());
            cols[1].push(b'\n');
        }

        #[derive(Clone, Copy)]
        struct Entry {
            stat1: Stat,
            stat2: Option<Stat>,
            percent: bool,
        }

        for (i, &Entry { stat1, stat2, percent }) in [
            Entry { stat1: Stat::CurrentHitPoints, stat2: Some(Stat::HitPoints), percent: false },
            Entry { stat1: Stat::ArmorClass, stat2: None, percent: false },
            Entry { stat1: Stat::DmgThresh, stat2: Some(Stat::DmgResist), percent: true },
            Entry { stat1: Stat::DmgThreshLaser, stat2: Some(Stat::DmgResistLaser), percent: true },
            Entry { stat1: Stat::DmgThreshFire, stat2: Some(Stat::DmgResistFire), percent: true },
            Entry { stat1: Stat::DmgThreshPlasma, stat2: Some(Stat::DmgResistPlasma), percent: true },
            Entry { stat1: Stat::DmgThreshExplosion, stat2: Some(Stat::DmgResistExplosion), percent: true },
        ].iter().enumerate() {
            cols[2].push_str(msg(7 + i as MessageId));
            cols[2].push(b'\n');

            if stat2.is_none() {
                cols[3].push_str("   ");
            }
            cols[3].push_str(stat(stat1).to_bstring());
            if let Some(stat2) = stat2.map(stat) {
                cols[3].push(b'/');
                cols[3].push_str(stat2.to_bstring());
            }
            if percent {
                cols[3].push(b'%');
            }
            cols[3].push(b'\n');
        }

        let mut misc = BString::new();
        misc.push_str(name);
        misc.push_str("\n---------------------\n\n\n\n\n\n\n\n");

        for &slot in &[EquipmentSlot::Hand(Hand::Left), EquipmentSlot::Hand(Hand::Right)] {
            let item = owner.equipment(slot, world.objects());
            misc.push_str("---------------------\n");
            if let Some(item) = item {
                let item = &world.objects().get(item);
                let proto = item.proto().unwrap();
                misc.push_str(proto.name().unwrap());
                misc.push(b'\n');
                match proto.kind() {
                    ExactEntityKind::Item(ItemKind::Weapon) => {
                        let weapon = proto.sub.as_weapon().unwrap();
                        let cat = weapon.attack_kinds[AttackGroup::Primary].category();
                        let melee_dmg = if cat.is_melee() {
                            stat(Stat::MeleeDmg)
                        } else {
                            0
                        };
                        let max_dmg = weapon.damage.end + melee_dmg;

                        // Dmg: 10-20
                        misc.push_str(msg(MSG_DMG));
                        misc.push(b' ');
                        misc.push_str(weapon.damage.start.to_bstring());
                        misc.push(b'-');
                        misc.push_str(max_dmg.to_bstring());

                        match cat {
                            | AttackCategory::Stand
                            | AttackCategory::MeleeUnarmed
                            => {}
                            | AttackCategory::MeleeWeapon
                            | AttackCategory::Throw
                            | AttackCategory::Fire
                            => {
                                let range = item.weapon_range(AttackGroup::Primary, rpg, world.objects()).unwrap();
                                //    Rng: 2
                                misc.push_str("   ");
                                misc.push_str(msg(MSG_RNG));
                                misc.push(b' ');
                                misc.push_str(range.to_bstring());
                            }
                        }
                    }
                    ExactEntityKind::Item(ItemKind::Armor) => {
                        misc.push_str(msg(MSG_NOT_WORN));
                    }
                    _ => {}
                }
                misc.push(b'\n');

                if_chain! {
                    if let Some(max_ammo) = proto.max_ammo_count();
                    if max_ammo > 0;
                    then {
                        let item = item.sub.as_item().unwrap();

                        // Ammo: 5/10
                        misc.push_str(msg(MSG_AMMO));
                        misc.push(b' ');
                        misc.push_str(item.ammo_count.to_bstring());
                        misc.push(b'/');
                        misc.push_str(max_ammo.to_bstring());

                        if let Some(ammo_proto) = item.ammo_proto.as_ref() {
                            let ammo_proto = ammo_proto.borrow();
                            let ammo_name = ammo_proto.name().unwrap();
                            // .44 Magnum JHP
                            misc.push(b' ');
                            misc.push_str(ammo_name);
                        }
                    }
                }
                misc.push(b'\n');
            } else {
                misc.push_str(msg(MSG_NO_ITEM));
                misc.push(b'\n');
                misc.push_str(msg(MSG_UNARMED_DMG));
                misc.push_str(" 1-");
                misc.push_str(stat(Stat::MeleeDmg).to_bstring());
                misc.push_str("\n\n");
            }
        }

        let mut total_weight = BString::new();
        if owner.kind() == EntityKind::Critter {
            let cw = stat(Stat::CarryWeight);
            let w = owner.inventory.weight(world.objects());
            // Total Wt: 100/200
            total_weight.push_str(msg(MSG_TOTAL_WEIGHT));
            total_weight.push(b' ');
            total_weight.push_str(w.to_bstring());
            total_weight.push(b'/');
            total_weight.push_str(cw.to_bstring());
        }
        let overloaded = owner.is_overloaded(rpg, world.objects());
        {
            let mut w = ui.widget_mut::<Panel>(self.total_weight);
            let w = w.text_mut().unwrap();
            w.text = total_weight;
            w.color = if overloaded { RED } else { GREEN };
        }

        ui.widget_mut::<Panel>(self.stat_misc).text_mut().unwrap().text = misc;
        for (i, s) in cols.iter_mut().enumerate() {
            ui.widget_mut::<Panel>(self.stat_columns[i]).text_mut().unwrap().text =
                std::mem::replace(s, BString::new());
        }
        self.switch_text_panels(true, ui);
    }

    // inven_obj_examine_func
    fn examine(&self, obj: object::Handle, description: &bstr, ui: &Ui) {
        let world = self.world.borrow();
        let name = world.object_name(obj).unwrap();
        let obj = world.objects().get(obj);
        let weight = obj.item_weight(world.objects()).filter(|&v| v > 0).unwrap_or(0);
        let weight_msg = if weight > 0 {
            let msg_id = if weight == 1 { 541 } else { 540 };
            let msg = &world.proto_db().messages().get(msg_id).unwrap().text;
            sprintf(&msg, &[&weight.to_bstring()])
        } else {
            "".into()
        };
        let msg = BString::concat(&[
            name.as_bytes(),
            b"\n--------------------\n",
            description.as_bytes(),
            b"\n",
            weight_msg.as_bytes()]);
        ui.widget_mut::<Panel>(self.item_descr).text_mut().unwrap().text = msg;
        self.switch_text_panels(false, ui);
    }

    fn switch_text_panels(&self, stats: bool, ui: &Ui) {
        for &w in self.stat_columns.iter()
            .chain([self.stat_misc, self.total_weight].iter())
        {
            ui.widget_base_mut(w).set_visible(stats);
        }
        ui.widget_base_mut(self.item_descr).set_visible(!stats);
    }

    fn toggle_mouse_mode(&mut self, rpg: &Rpg, ui: &Ui) {
        self.mouse_mode = match self.mouse_mode {
            MouseMode::Action => MouseMode::Drag,
            MouseMode::Drag => MouseMode::Action,
        };
        self.sync_mouse_mode_to_ui(ui);
        self.update_stats(rpg, ui);
    }

    fn sync_mouse_mode_to_ui(&self, ui: &Ui) {
        for &w in &[self.list, self.wearing, self.left_hand, self.right_hand] {
            let w = &mut ui.widget_mut::<InventoryList>(w);
            w.set_mouse_mode(self.mouse_mode);
        }
        let cursor = match self.mouse_mode {
            MouseMode::Action => Cursor::ActionArrow,
            MouseMode::Drag => Cursor::Hand,
        };
        ui.widget_base_mut(self.win).set_cursor(Some(cursor));
    }

    fn slot_from_widget(&self, widget: ui::Handle) -> Option<Slot> {
        Some(match () {
            _ if widget == self.list => Slot::Inventory,
            _ if widget == self.wearing => Slot::Equipment(EquipmentSlot::Armor),
            _ if widget == self.left_hand => Slot::Equipment(EquipmentSlot::Hand(Hand::Left)),
            _ if widget == self.right_hand => Slot::Equipment(EquipmentSlot::Hand(Hand::Right)),
            _ => return None,
        })
    }

    // switch_hands
    fn handle_list_drop(&mut self,
        src: ui::Handle,
        pos: Point,
        src_obj: object::Handle,
        rpg: &Rpg,
        ui: &mut Ui,
    ) {
        let src_slot = self.slot_from_widget(src).unwrap();

        let target = unwrap_or_return!(ui.widget_at(pos), Some);
        if target == src {
            return;
        }
        let target_slot = unwrap_or_return!(self.slot_from_widget(target), Some);

        assert_ne!(src_slot, target_slot);

        let world = self.world.borrow();

        enum Action {
            MoveTo {
                item: object::Handle,
                slot: Slot,
            },
            Reload {
                weapon: object::Handle,
                max_count: u32,
            },
            ArmorChange {
                old_armor: Option<object::Handle>,
                new_armor: Option<object::Handle>,
            },
        }

        let mut actions = Vec::new();

        match target_slot {
            Slot::Inventory => {
                actions.push(Action::MoveTo {
                    item: src_obj,
                    slot: Slot::Inventory,
                });
            }
            Slot::Equipment(eq_slot) => {
                let owner = world.objects().get(self.owner);
                let target_obj = owner.equipment(eq_slot, world.objects());
                let src_obj = world.objects().get(src_obj);

                if eq_slot == EquipmentSlot::Armor {
                    if src_obj.proto().unwrap().kind() != ExactEntityKind::Item(ItemKind::Armor) {
                        return;
                    }
                    actions.push(Action::ArmorChange {
                        old_armor: target_obj,
                        new_armor: Some(src_obj.handle()),
                    });
                }

                // Check for weapon reload.
                let reload = if_chain! {
                    if let Some(target_obj) = target_obj;
                    let weapon = world.objects().get(target_obj);
                    if let Some(max_count) = weapon.can_reload_weapon(&src_obj);
                    then {
                        actions.push(Action::Reload {
                            weapon: weapon.handle(),
                            max_count,
                        });
                        true
                    } else {
                        false
                    }
                };

                // Default actions are to move/replace.
                if !reload {
                    actions.push(Action::MoveTo {
                        item: src_obj.handle(),
                        slot: Slot::Equipment(eq_slot),
                    });
                    if let Some(target_obj) = target_obj {
                        // Move target out of the slot.
                        actions.push(Action::MoveTo {
                            item: target_obj,
                            slot: Slot::Inventory,
                        });
                    }
                }
            }
        };
        if src_slot == Slot::Equipment(EquipmentSlot::Armor) {
            actions.push(Action::ArmorChange {
                old_armor: Some(src_obj),
                new_armor: None,
            });
        }

        for action in actions {
            let owner = &mut world.objects().get_mut(self.owner);
            match action {
                Action::MoveTo { item, slot } => {
                    let mut item = world.objects().get_mut(item);
                    item.flags.remove(Flag::Worn | Flag::LeftHand | Flag::RightHand);
                    match slot {
                        Slot::Inventory => {
                            let i = owner.inventory.items.iter()
                                .position(|i| i.object == item.handle()).unwrap();
                            if i > 0 {
                                let item = owner.inventory.items.remove(i);
                                owner.inventory.items.insert(0, item);
                            }
                        }
                        Slot::Equipment(eq_slot) => match eq_slot {
                            EquipmentSlot::Armor => item.flags.insert(Flag::Worn),
                            EquipmentSlot::Hand(Hand::Left) => item.flags.insert(Flag::LeftHand),
                            EquipmentSlot::Hand(Hand::Right) => item.flags.insert(Flag::RightHand),
                        }
                    }
                }
                Action::Reload { weapon, max_count } => {
                    if max_count <= 1 {
                        let mut weapon = world.objects().get_mut(weapon);
                        let mut ammo = world.objects().get_mut(src_obj);
                        weapon.sub.as_item_mut().unwrap().ammo_count += max_count;
                        let ammo = ammo.sub.as_item_mut().unwrap();
                        ammo.ammo_count = ammo.ammo_count.checked_sub(max_count).unwrap();
                    } else {
                        let win = InventoryMoveWindow::show(
                            weapon,
                            &world.objects().get(src_obj),
                            max_count,
                            &self.msgs,
                            ui);
                        assert!(self.move_window.replace(win).is_none());
                    }
                }
                Action::ArmorChange { old_armor, new_armor } => {
                    let old_armor = old_armor.map(|obj| world.objects().get(obj));
                    let new_armor = new_armor.map(|obj| world.objects().get(obj));
                    rpg.apply_armor_change(owner, old_armor.as_deref(), new_armor.as_deref(), world.objects());
                }
            }
        }

        self.sync_owner_fid(rpg);
        self.sync_to_ui(rpg, ui);
    }

    fn sync_owner_fid(&self, rpg: &Rpg) {
        let world = self.world.borrow();
        let mut owner = world.objects().get_mut(self.owner);
        owner.fid = owner.equipped_fid(world.objects(), rpg);
    }

    fn unload(&self, weapon: object::Handle, rpg: &Rpg, ui: &Ui) {
        {
            let mut world = self.world.borrow_mut();
            let ammo = unwrap_or_return!(world.objects_mut().unload_weapon(weapon), Some);
            world.objects_mut().move_into_inventory(self.owner, ammo, 1);
        }
        self.sync_to_ui(rpg, ui);
    }

    fn handle(&mut self, cmd: UiCommand, rpg: &Rpg, ui: &mut Ui) {
        match cmd.data {
            UiCommandData::Inventory(c) => match c {
                Command::Show | Command::Hide => {}
                Command::Scroll(scroll) => {
                    self.scroll(scroll, ui);
                }
                Command::Hover { .. } => {}
                Command::ActionMenu { object } => {
                    self.show_action_menu(object, ui);
                }
                Command::Action { object, action } => {
                    self.hide_action_menu(ui);
                    if let Some(Action::Unload) = action {
                        self.unload(object, rpg, ui);
                    }
                }
                Command::ListDrop { pos, object } => {
                    self.handle_list_drop(cmd.source, pos, object, rpg, ui);
                }
                Command::ToggleMouseMode => {
                    self.toggle_mouse_mode(rpg, ui);
                }
            }
            UiCommandData::MoveWindow(c) => {
                if let move_window::Command::Hide { ok } = c {
                    let win = self.move_window.take().unwrap();
                    if ok {
                        self.world.borrow_mut().objects_mut()
                            .reload_weapon_from_inventory(self.owner, win.weapon, win.ammo);
                        self.sync_to_ui(rpg, ui);
                    }
                    win.win.hide(ui);
                }
            }
            _ => {}
        }
        if let Some(v) = self.move_window.as_mut() {
            v.win.handle(cmd, ui);
        }
    }
}

struct MouseModeToggler;

impl Widget for MouseModeToggler {
    fn handle_event(&mut self, mut ctx: HandleEvent) {
        if let ui::UiEvent::MouseDown { button: MouseButton::Right, .. } = ctx.event {
            ctx.out(UiCommandData::Inventory(Command::ToggleMouseMode));
        }
    }
}

struct InventoryMoveWindow {
    weapon: object::Handle,
    ammo: object::Handle,
    win: MoveWindow,
}

impl InventoryMoveWindow {
    pub fn show(
        weapon: object::Handle,
        ammo: &Object,
        max: u32,
        msgs: &Messages,
        ui: &mut Ui,
    ) -> Self {
        let fid = ammo.proto().unwrap().sub.as_item().unwrap().inventory_fid.unwrap();
        let win = MoveWindow::show(fid, max, msgs, ui);
        Self {
            weapon,
            ammo: ammo.handle(),
            win,
        }
    }
}
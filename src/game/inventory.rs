use bstring::{bstr, BString};
use bstring::bfmt::ToBString;
use if_chain::if_chain;

use crate::asset::*;
use crate::asset::frame::FrameId;
use crate::asset::message::{Messages, MessageId};
use crate::fs::FileSystem;
use crate::game::object::{self, Object, InventoryItem};
use crate::game::rpg::Rpg;
use crate::game::ui::action_menu::{self, Action};
use crate::game::ui::inventory_list::{self, InventoryList, Scroll};
use crate::game::world::WorldRef;
use crate::graphics::Rect;
use crate::graphics::color::{GREEN, RED};
use crate::graphics::font::*;
use crate::graphics::sprite::Sprite;
use crate::ui::{self, Ui, button};
use crate::ui::button::Button;
use crate::ui::command::{UiCommandData, UiCommand};
use crate::ui::command::inventory::Command;
use crate::ui::panel::{self, Panel};
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

    pub fn handle(&mut self, cmd: UiCommand, rpg: &Rpg, ui: &mut Ui) {
        match cmd.data {
            UiCommandData::Inventory(cmd) => match cmd {
                Command::Show => {
                    self.show(rpg, ui);
                }
                Command::Hide => {
                    self.hide(ui);
                }
                Command::Scroll(scroll) => {
                    self.internal.as_ref().unwrap().scroll(scroll, ui);
                }
                Command::Hover { .. } => {}
                Command::ActionMenu { object } => {
                    self.internal.as_mut().unwrap().show_action_menu(object, ui);
                }
                Command::Action { .. } => {
                    self.internal.as_mut().unwrap().hide_action_menu(ui);
                }
                Command::ListDrop { pos: _, object: _ } => {}
            }
            _ => {}
        }
    }

    pub fn examine(&self, obj: object::Handle, description: &bstr, ui: &Ui) {
        self.internal.as_ref().unwrap().examine(obj, description, ui);
    }

    fn show(&mut self, rpg: &Rpg, ui: &mut Ui) {
        let obj = self.world.borrow().dude_obj().unwrap();
        let internal = Internal::new(self.msgs.take().unwrap(), self.world.clone(), obj, ui);
        internal.sync_from_obj(rpg, ui);
        assert!(self.internal.replace(internal).is_none());
    }

    fn hide(&mut self, ui: &mut Ui) {
        let i = self.internal.take().unwrap();
        i.remove(ui);
        self.msgs = Some(i.msgs);
    }
}

struct Internal {
    msgs: Messages,
    world: WorldRef,
    obj: object::Handle,
    win: ui::Handle,
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
}

impl Internal {
    fn new(msgs: Messages, world: WorldRef, obj: object::Handle, ui: &mut Ui) -> Self {
        let win = ui.new_window(Rect::with_size(80, 0, 499, 377),
            Some(Sprite::new(FrameId::INVENTORY_WINDOW)));
        ui.set_modal_window(Some(win));
        ui.widget_base_mut(win).set_cursor(Some(crate::ui::Cursor::ActionArrow));

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

        Self {
            msgs,
            world,
            obj,
            win,
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
        }
    }

    fn remove(&self, ui: &mut Ui) {
        ui.remove(self.win);
    }

    fn sync_from_obj(&self, rpg: &Rpg, ui: &Ui) {
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
        let obj = world.objects().get(self.obj);
        for item in &obj.inventory.items {
            let item_obj = &world.objects().get(item.object);
            let inv_list_item = Self::to_inv_list_item(item, item_obj);
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
    }

    // display_inventory_info
    fn to_inv_list_item(item: &InventoryItem, obj: &Object) -> inventory_list::Item {
        let proto = obj.proto().unwrap();
        let count = if obj.item_kind() == Some(ItemKind::Ammo) {
            obj.proto().unwrap().max_ammo_count().unwrap() * (item.count - 1)
                + obj.ammo_count().unwrap()
        } else {
            item.count
        };
        inventory_list::Item {
            object: item.object,
            fid: proto.sub.as_item().unwrap().inventory_fid.unwrap(),
            count,
        }
    }

    fn scroll(&self, scroll: Scroll, ui: &mut Ui) {
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
        let name = world.object_name(self.obj).unwrap();
        let obj = &world.objects().get(self.obj);

        let stat = |stat| rpg.stat(stat, obj, world.objects());
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

        for &item in &[
            obj.in_left_hand(world.objects()),
            obj.in_right_hand(world.objects()),
        ] {
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
        if obj.kind() == EntityKind::Critter {
            let cw = stat(Stat::CarryWeight);
            let w = obj.inventory.weight(world.objects());
            // Total Wt: 100/200
            total_weight.push_str(msg(MSG_TOTAL_WEIGHT));
            total_weight.push(b' ');
            total_weight.push_str(w.to_bstring());
            total_weight.push(b'/');
            total_weight.push_str(cw.to_bstring());
        }
        let overloaded = obj.is_overloaded(rpg, world.objects());
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
            "\n--------------------\n".as_bytes(),
            description.as_bytes(),
            "\n".as_bytes(),
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
}
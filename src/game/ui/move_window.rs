use crate::ui::{self, Ui};
use crate::graphics::sprite::{Sprite, Effect};
use crate::asset::frame::FrameId;
use crate::graphics::Rect;
use crate::ui::image_text::ImageText;
use crate::asset::message::Messages;
use crate::ui::panel::{self, Panel};
use crate::graphics::font::{FontKey, DrawOptions, HorzAlign, VertAlign};
use crate::graphics::color::Rgb15;
use crate::ui::button::{Button, Text};
use bstring::bfmt::ToBString;
use crate::ui::command::move_window::Command;
use crate::ui::command::{UiCommand, UiCommandData};

pub struct MoveWindow {
    max: u32,
    win: ui::Handle,
    count: ui::Handle,
    value: u32,
}

impl MoveWindow {
    pub fn show(item_fid: FrameId, max: u32, msgs: &Messages, ui: &mut Ui) -> Self {
        assert!(max > 0);

        let win = ui.new_window(Rect::with_size(140, 80, 259, 162),
            Some(Sprite::new(FrameId::INVENTORY_MOVE_MULTIPLE_WINDOW)));
        ui.widget_base_mut(win).set_modal(true);

        let mut header = Panel::new();
        header.set_text(Some(panel::Text {
            text: msgs.get(21).unwrap().text.clone(),
            font: FontKey::antialiased(3),
            color: Rgb15::from_packed(0x5263),
            options: DrawOptions {
                horz_align: HorzAlign::Center,
                ..Default::default()
            },
        }));
        ui.new_widget(win, Rect::with_size(0, 9, 259, 162), None, None, header);

        let mut item = Sprite::new(item_fid);
        item.effect = Some(Effect::Fit {
            width: 90,
            height: 61,
        });
        ui.new_widget(win, Rect::with_size(16, 46, 1, 1), None, Some(item), Panel::new());

        let count = ui.new_widget(win, Rect::with_size(125, 45, 1, 1), None, None,
            ImageText::big_numbers());

        ui.new_widget(win, Rect::with_size(200, 46, 16, 12), None, None,
            Button::new(FrameId::BUTTON_PLUS_UP, FrameId::BUTTON_PLUS_DOWN,
            Some(UiCommandData::MoveWindow(Command::Inc))));
        ui.new_widget(win, Rect::with_size(200, 46 + 12, 16, 12), None, None,
            Button::new(FrameId::BUTTON_MINUS_UP, FrameId::BUTTON_MINUS_DOWN,
            Some(UiCommandData::MoveWindow(Command::Dec))));

        ui.new_widget(win, Rect::with_size(98, 128, 15, 16), None, None,
            Button::new(FrameId::SMALL_RED_BUTTON_UP, FrameId::SMALL_RED_BUTTON_DOWN,
            Some(UiCommandData::MoveWindow(Command::Hide { ok: true }))));
        ui.new_widget(win, Rect::with_size(148, 128, 15, 16), None, None,
            Button::new(FrameId::SMALL_RED_BUTTON_UP, FrameId::SMALL_RED_BUTTON_DOWN,
            Some(UiCommandData::MoveWindow(Command::Hide { ok: false }))));

        let mut text = Text::new(msgs.get(22).unwrap().text.clone(), FontKey::antialiased(3));
        text.color = Rgb15::from_packed(0x5263);
        text.options.horz_align = HorzAlign::Center;
        text.options.vert_align = VertAlign::Middle;
        let mut all = Button::new(FrameId::BUTTON_ALL_UP, FrameId::BUTTON_ALL_DOWN,
            Some(UiCommandData::MoveWindow(Command::Max)));
        all.set_text(Some(text));
        ui.new_widget(win, Rect::with_size(121, 80, 94, 33), None, None, all);

        let r = Self {
            max: std::cmp::min(max, 99999),
            win,
            count,
            value: 1,
        };
        r.sync(ui);
        r
    }

    pub fn hide(self, ui: &mut Ui) {
        ui.remove(self.win);
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn handle(&mut self, cmd: UiCommand, ui: &Ui) {
        if let UiCommandData::MoveWindow(cmd) = cmd.data {
            let new_value = match cmd {
                Command::Hide { .. } => {
                    return;
                }
                Command::Inc => std::cmp::min(self.value + 1, self.max),
                Command::Dec => std::cmp::max(self.value - 1, 1),
                Command::Max => self.max,
            };
            if new_value != self.value {
                self.value = new_value;
                self.sync(ui);
            }
        }
    }

    fn sync(&self, ui: &Ui) {
        *ui.widget_mut::<ImageText>(self.count).text_mut() =
            format!("{:05}", self.value).to_bstring();
    }
}
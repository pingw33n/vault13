use crate::asset::frame::FrameId;
use crate::graphics::Rect;
use crate::graphics::sprite::{Sprite, Effect};
use crate::ui::*;
use crate::ui::message_panel::{MessagePanel, MouseControl};
use crate::ui::panel::Panel;
use crate::graphics::font::FontKey;
use crate::graphics::color::{Rgb15, GREEN};
use crate::asset::message::BULLET_STR;
use bstring::{bstr, BString};
use crate::game::script::Sid;

pub struct OptionInfo {
    pub proc_id: Option<u32>,
}

pub struct Dialog {
    window: Handle,
    reply: Handle,
    options_widget: Handle,
    options: Vec<OptionInfo>,
    sid: Sid,
    pub running: bool,
}

impl Dialog {
    pub fn show(ui: &mut Ui, sid: Sid) -> Self {
        let window = ui.new_window(Rect::with_size(0, 0, 640, 480),
            Some(Sprite::new(FrameId::ALLTLK)));

        ui.new_widget(window, Rect::with_size(0, 480 - 190, 640, 480), None,
            Some(Sprite::new(FrameId::DI_TALK)), Panel::new());

        let reply = MessagePanel::new(ui.fonts().clone(), FontKey::antialiased(1), GREEN);
        let reply = ui.new_widget(window, Rect::with_size(135, 235, 382, 47), None, None, reply);

        let mut options = MessagePanel::new(ui.fonts().clone(), FontKey::antialiased(1), GREEN);
        options.set_mouse_control(MouseControl::Pick);
        options.set_highlight_color(Rgb15::new(31, 31, 15));
        options.set_message_spacing(2);
        let options_widget = ui.new_widget(window, Rect::with_size(127, 340, 397, 100), None, None, options);

        let mut spr = Sprite::new(FrameId::HILIGHT1);
        spr.effect = Some(Effect::Highlight { color: Rgb15::from_packed(0x4631) });
        ui.new_widget(window, Rect::with_size(426, 15, 1, 1), None, Some(spr),
            Panel::new());

        let mut spr = Sprite::new(FrameId::HILIGHT2);
        spr.effect = Some(Effect::Highlight { color: Rgb15::from_packed(0x56ab) });
        ui.new_widget(window, Rect::with_size(129, 214 - 2 - 131, 1, 1), None, Some(spr),
            Panel::new());

        Self {
            window,
            reply,
            options_widget,
            options: Vec::new(),
            running: false,
            sid,
        }
    }

    pub fn hide(&self, ui: &mut Ui) {
        ui.remove(self.window);
    }

    pub fn is(&self, widget: Handle) -> bool {
        self.options_widget == widget
    }

    pub fn set_reply(&self, ui: &mut Ui, reply: impl AsRef<bstr>) {
        let mut replyw = ui.widget_mut::<MessagePanel>(self.reply);
        replyw.clear_messages();
        replyw.push_message(BString::concat(&[&b"  "[..], reply.as_ref().as_bytes()]))
    }

    pub fn clear_options(&mut self, ui: &mut Ui) {
        ui.widget_mut::<MessagePanel>(self.options_widget).clear_messages();
        self.options.clear();
    }

    pub fn add_option(&mut self, ui: &mut Ui, text: impl AsRef<bstr>, proc_id: Option<u32>) {
        let mut optionsw = ui.widget_mut::<MessagePanel>(self.options_widget);
        optionsw.push_message(Self::build_option(text.as_ref()));
        self.options.push(OptionInfo {
            proc_id,
        });
    }

    pub fn option(&self, id: u32) -> &OptionInfo {
        &self.options[id as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    pub fn sid(&self) -> Sid {
        self.sid
    }

    fn build_option(option: &bstr) -> BString {
        BString::concat(&[&b"  "[..], BULLET_STR, &b" "[..], option.as_bytes()])
    }
}


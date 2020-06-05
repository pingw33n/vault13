use bstring::{bstr, BString};

use crate::asset::frame::FrameId;
use crate::asset::message::BULLET_STR;
use crate::game::object;
use crate::game::script::ScriptIId;
use crate::game::world::World;
use crate::graphics::{Point, Rect};
use crate::graphics::color::{Rgb15, GREEN};
use crate::graphics::font::FontKey;
use crate::graphics::sprite::{Sprite, Effect};
use crate::ui::*;
use crate::ui::message_panel::{MessagePanel, MouseControl};
use crate::ui::panel::Panel;

pub struct OptionInfo {
    pub proc_id: Option<u32>,
}

pub struct Dialog {
    window: Handle,
    reply: Handle,
    options_widget: Handle,
    options: Vec<OptionInfo>,
    sid: ScriptIId,
    saved_camera_origin: Point,
    pub obj: object::Handle,
    pub running: bool,
}

impl Dialog {
    pub fn show(ui: &mut Ui, world: &mut World, obj: object::Handle) -> Self {
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

        let (obj_pos, sid) = {
            let obj = world.objects().get(obj);
            let (sid, _) = obj.script.unwrap();
            (obj.pos.unwrap().point, sid)
        };

        let saved_camera_origin = world.camera().origin;
        world.camera_mut().align(obj_pos, Point::new(640 / 2, 235 / 2));

        Self {
            window,
            reply,
            options_widget,
            options: Vec::new(),
            running: false,
            sid,
            saved_camera_origin,
            obj,
        }
    }

    pub fn hide(self, ui: &mut Ui, world: &mut World) {
        ui.remove(self.window);
        world.camera_mut().origin = self.saved_camera_origin;
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

    pub fn sid(&self) -> ScriptIId {
        self.sid
    }

    fn build_option(option: &bstr) -> BString {
        BString::concat(&[&b"  "[..], BULLET_STR, &b" "[..], option.as_bytes()])
    }
}


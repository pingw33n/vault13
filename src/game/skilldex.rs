use enum_map::EnumMap;
use std::convert::TryInto;

use crate::asset::frame::FrameId;
use crate::asset::message::{Messages, MessageId};
use crate::fs::FileSystem;
use crate::game::object;
use crate::graphics::{Rect, Point};
use crate::graphics::color::Rgb15;
use crate::graphics::font::{FontKey, HorzAlign, VertAlign};
use crate::graphics::sprite::Sprite;
use crate::ui::*;
use crate::ui::panel::{self, Panel};
use crate::ui::button::{self, Button};
use crate::ui::command::{UiCommandData, SkilldexCommand};
use crate::util::EnumExt;
use crate::ui::image_text::ImageText;

const TEXT_FONT: FontKey = FontKey::antialiased(3);
const TEXT_COLOR: Rgb15 = unsafe { Rgb15::rgb15_from_packed_unchecked(0x4a23) };
const TEXT_COLOR_DOWN: Rgb15 = unsafe { Rgb15::rgb15_from_packed_unchecked(0x3983) };

#[derive(Clone, Copy, Debug, enum_map_derive::Enum, Eq, PartialEq)]
pub enum Skill {
    // Order is important here.
    Sneak,
    Lockpick,
    Steal,
    Traps,
    FirstAid,
    Doctor,
    Science,
    Repair,
}

impl Into<crate::asset::Skill> for Skill {
    fn into(self) -> crate::asset::Skill {
        use crate::asset::Skill::*;
        match self {
            Self::Sneak => Sneak,
            Self::Lockpick => Lockpick,
            Self::Steal => Steal,
            Self::Traps => Traps,
            Self::FirstAid => FirstAid,
            Self::Doctor => Doctor,
            Self::Science => Science,
            Self::Repair => Repair,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Cancel,
    Skill(Skill),
}

pub struct Skilldex {
    msgs: Messages,
    window: Option<Handle>,
}

impl Skilldex {
    pub fn new(fs: &FileSystem, language: &str) -> Self {
        let msgs = Messages::read_file(fs, language, "game/skilldex.msg").unwrap();
        Self {
            msgs,
            window: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_some()
    }

    pub fn show(&mut self,
        ui: &mut Ui,
        levels: EnumMap<Skill, i32>,
        target: Option<object::Handle>,
    ) {
        assert!(self.window.is_none());

        let win_size = ui.frm_db().get(FrameId::SKILLDEX_WINDOW).unwrap().first().size();
        let window = ui.new_window(Rect::with_size(
            640 - win_size.x - 4, 379 - win_size.y - 6, win_size.x, win_size.y),
            Some(Sprite::new(FrameId::SKILLDEX_WINDOW)));
        ui.set_modal_window(Some(window));

        let mut header = Panel::new();
        header.set_text(Some(panel::Text {
            text: self.msgs.get(100).unwrap().text.clone(),
            font: TEXT_FONT,
            color: TEXT_COLOR,
            options: Default::default(),
        }));
        ui.new_widget(window, Rect::with_size(55, 14, 1, 1), None, None, header);

        let btn_size = ui.frm_db().get(FrameId::SKILLDEX_BUTTON_UP).unwrap().first().size();
        for (i, skill) in Skill::iter().enumerate() {
            let mut btn = Button::new(FrameId::SKILLDEX_BUTTON_UP, FrameId::SKILLDEX_BUTTON_DOWN,
                Some(UiCommandData::Skilldex(SkilldexCommand::Skill {
                    skill: skill.into(),
                    target,
                })));
            let pos = (15, 45 + (btn_size.y + 3) * i as i32).into();
            let text = self.msgs.get(102 + i as MessageId).unwrap().text.clone();
            let mut text = button::Text::new(text, TEXT_FONT);
            text.pos = Point::new(1, 1);
            text.options.horz_align = HorzAlign::Center;
            text.options.vert_align = VertAlign::Middle;
            btn.set_text(Some(text));
            btn.config_mut(button::State::Up).text.as_mut().unwrap().color = TEXT_COLOR;
            btn.config_mut(button::State::Down).text.as_mut().unwrap().color = TEXT_COLOR_DOWN;

            ui.new_widget(window, Rect::with_points(pos, pos + btn_size), None, None, btn);

            let level: u32 = levels[skill].try_into().unwrap_or(0);
            let mut level_wid = ImageText::standard_digits(FrameId::BIG_NUMBERS, 14);
            *level_wid.text_mut() = format!("{:03}", level).into();
            let pos = pos + Point::new(96, 3);
            ui.new_widget(window, Rect::with_size(pos.x, pos.y, 1, 1), None, None, level_wid);
        }

        let btn_size = ui.frm_db().get(FrameId::SMALL_RED_BUTTON_UP).unwrap().first().size();
        let mut cancel = Button::new(FrameId::SMALL_RED_BUTTON_UP, FrameId::SMALL_RED_BUTTON_DOWN,
            Some(UiCommandData::Skilldex(SkilldexCommand::Cancel)));
        let mut text = button::Text::new(self.msgs.get(101).unwrap().text.clone(), TEXT_FONT);
        text.pos = Point::new(btn_size.x + 9, 1);
        text.color = TEXT_COLOR;
        text.options.vert_align = VertAlign::Middle;
        cancel.set_text(Some(text));
        ui.new_widget(window, Rect::with_size(48, 338, 90, btn_size.y), None, None, cancel);

        self.window = Some(window);
    }

    pub fn hide(&mut self, ui: &mut Ui) {
        let window = self.window.take().unwrap();
        ui.remove(window);
    }
}
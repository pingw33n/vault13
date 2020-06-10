use bstring::BString;
use enum_map::{enum_map, EnumMap};

use crate::graphics::font::{DrawOptions, FontKey, HorzAlign, VertAlign};
use crate::graphics::color::Rgb15;
use crate::graphics::sprite::Sprite;
use crate::ui::command::UiCommandData;
use super::*;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd)]
pub enum State {
    Up,
    Down,
}

pub struct Config {
    pub background: Option<Sprite>,
    pub text: Option<Text>,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub pos: Point,
    pub text: BString,
    pub font: FontKey,
    pub color: Rgb15,
    pub options: DrawOptions,
}

impl Text {
    pub fn new(text: BString, font: FontKey) -> Self {
        Self {
            text,
            font,
            pos: Default::default(),
            color: Default::default(),
            options: Default::default(),
        }
    }
}

pub struct Button {
    configs: EnumMap<State, Config>,
    command: Option<UiCommandData>,
    state: State,
}

impl Button {
    pub fn new(up: FrameId, down: FrameId, command: Option<UiCommandData>) -> Self {
        Self {
            configs: enum_map! {
                State::Up => Config {
                    background: Some(Sprite::new(up)),
                    text: None,
                },
                State::Down => Config {
                    background: Some(Sprite::new(down)),
                    text: None,
                },
            },
            command,
            state: State::Up,
        }
    }

    pub fn config(&self, state: State) -> &Config {
        &self.configs[state]
    }

    pub fn config_mut(&mut self, state: State) -> &mut Config {
        &mut self.configs[state]
    }

    pub fn set_text(&mut self, text: Option<Text>) {
        self.configs[State::Down].text = text.clone();
        self.configs[State::Up].text = text;
    }
}

impl Widget for Button {
    fn handle_event(&mut self, mut ctx: HandleEvent) {
        match ctx.event {
            Event::MouseDown { button, .. } if button == MouseButton::Left => {
                self.state = State::Down;
                ctx.capture();
            }
            Event::MouseMove { pos } if ctx.is_captured() => {
                // FIXME should optionally hit test the frame as in original.
                self.state = if ctx.base.rect.contains(pos) {
                    State::Down
                } else {
                    State::Up
                }
            }
            Event::MouseUp { pos, button } if button == MouseButton::Left => {
                self.state = State::Up;
                // FIXME should optionally hit test the frame as in original.
                if ctx.base.rect.contains(pos) {
                    if let Some(cmd) = self.command.clone() {
                        ctx.out(cmd);
                    }
                }
                ctx.release();
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let config = &self.configs[self.state];
        let base_rect = ctx.base.unwrap().rect;
        if let Some(mut background) = config.background {
            background.pos += base_rect.top_left();
            background.render(ctx.canvas, ctx.frm_db);
        }
        if let Some(text) = config.text.as_ref() {
            let mut pos = base_rect.top_left() + text.pos;
            if text.options.horz_align == HorzAlign::Center {
                pos.x += base_rect.width() / 2;
            }
            if text.options.vert_align == VertAlign::Middle {
                pos.y += base_rect.height() / 2;
            }
            ctx.canvas.draw_text(
                &text.text,
                pos,
                text.font,
                text.color,
                &text.options);
        }
    }
}
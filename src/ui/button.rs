use enum_map::{enum_map, EnumMap};

use super::*;
use crate::graphics::sprite::Sprite;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Ord, PartialOrd)]
enum State {
    Up,
    Down,
}

pub struct Button {
    sprites: EnumMap<State, Sprite>,
    state: State,
}

impl Button {
    pub fn new(up: FrameId, down: FrameId) -> Self {
        Self {
            sprites: enum_map! {
                State::Up => Sprite::new(up),
                State::Down => Sprite::new(down),
            },
            state: State::Up,
        }
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
                    dbg!("clicked");
                }
                ctx.release();
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let sprite = &mut self.sprites[self.state];
        sprite.pos = ctx.base.unwrap().rect.top_left();
        sprite.render(ctx.canvas, ctx.frm_db);
    }
}
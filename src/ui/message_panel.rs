use bstring::BString;
use std::cmp;
use std::collections::VecDeque;

use super::*;
use crate::graphics::color::Rgb15;
use crate::graphics::font::{self, FontKey, Fonts};

pub struct MessagePannel {
    fonts: Rc<Fonts>,
    font: FontKey,
    color: Rgb15,
    messages: VecDeque<(BString, u32)>,
    lines: VecDeque<BString>,
    capacity: usize,
    width: Option<i32>,
    scroll_pos: i32,
}

impl MessagePannel {
    pub fn new(fonts: Rc<Fonts>, font: FontKey, color: Rgb15, capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            fonts,
            font,
            color,
            messages: VecDeque::with_capacity(capacity),
            lines: VecDeque::new(),
            capacity,
            width: None,
            scroll_pos: i32::max_value(),
        }
    }

    pub fn push_message(&mut self, message: impl Into<BString>) {
        while self.messages.len() >= self.capacity {
            let line_count = self.messages.pop_front().unwrap().1;
            for _ in 0..line_count {
                self.lines.pop_front().unwrap();
            }
        }

        let mut message = message.into();
        message.insert(0, b'\x95');

        let font = self.fonts.get(self.font);
        let new_lines: VecDeque<_> = font.lines(&message, Some(font::Overflow {
                size: self.width.unwrap(),
                mode: font::OverflowMode::WordWrap,
            }))
            .map(|s| s.to_owned())
            .collect();

        self.messages.push_back((message, new_lines.len() as u32));
        for line in new_lines {
            self.lines.push_back(line);
        }

        dbg!(&self.lines);
    }
}

impl Widget for MessagePannel {
    fn init(&mut self, ctx: Init) {
        self.width = Some(ctx.base.rect.width());
    }

    fn handle_event(&mut self, ctx: HandleEvent) {
        match ctx.event {
            Event::MouseMove { pos } => {
                let half = ctx.base.rect.height() / 2;

                let mut top = ctx.base.rect.clone();
                top.bottom -= half;
                if top.contains(pos.x, pos.y) {
                    ctx.base.cursor = Some(Cursor::ArrowUp);
                    return;
                }

                let mut bottom = ctx.base.rect.clone();
                bottom.top += half;
                if bottom.contains(pos.x, pos.y) {
                    ctx.base.cursor = Some(Cursor::ArrowDown);
                    return;
                }

                ctx.base.cursor = None;
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let font = self.fonts.get(self.font);

        let line_height = font.height + font.vert_spacing;
        let base = ctx.base.unwrap();
        assert_eq!(base.rect.width(), self.width.unwrap());
        let line_count = cmp::max(base.rect.height() / line_height, 1);

        let mut x = base.rect.left;
        let mut y = base.rect.top;
        let scroll_pos = cmp::min(cmp::max(self.scroll_pos, -line_count + 1),
            self.lines.len() as i32 - line_count);
        for i in scroll_pos..scroll_pos + line_count {
            if i >= 0 {
                ctx.canvas.draw_text(&self.lines[i as usize], x, y, self.font, self.color,
                    &font::DrawOptions::default());
            }
            x += 1;
            y += line_height;
        }
    }
}
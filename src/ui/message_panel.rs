use bstring::BString;
use std::cmp;
use std::collections::VecDeque;
use std::time::Duration;

use super::*;
use crate::graphics::color::Rgb15;
use crate::graphics::font::{self, FontKey, Fonts};

#[derive(Clone, Copy, Debug)]
struct Layout {
    width: i32,
    visible_line_count: i32,
}

struct RepeatState<T> {
    last: Instant,
    value: T,
}

struct Repeat<T> {
    interval: Duration,
    state: Option<RepeatState<T>>,
}

impl<T> Repeat<T> {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: None,
        }
    }

    pub fn start(&mut self, now: Instant, value: T) {
        self.state = Some(RepeatState { last: now, value });
    }

    pub fn stop(&mut self) {
        self.state = None;
    }

    #[must_use]
    pub fn update(&mut self, now: Instant) -> Option<&T> {
        let state = self.state.as_mut().unwrap();
        if now >= state.last + self.interval {
            state.last = now;
            Some(&state.value)
        } else {
            None
        }
    }

    #[must_use]
    pub fn update_if_running(&mut self, now: Instant) -> Option<&T> {
        if self.state.is_some() {
            self.update(now)
        } else {
            None
        }
    }
}

pub struct MessagePanel {
    fonts: Rc<Fonts>,
    font: FontKey,
    color: Rgb15,
    messages: VecDeque<(BString, u32)>,
    lines: VecDeque<BString>,
    capacity: Option<usize>,
    layout: Option<Layout>,
    scroll_pos: i32,
    repeat_scroll: Repeat<Scroll>,
    scrollable: bool,
    skew: i32,
}

impl MessagePanel {
    pub fn new(fonts: Rc<Fonts>, font: FontKey, color: Rgb15) -> Self {
        Self {
            fonts,
            font,
            color,
            messages: VecDeque::new(),
            lines: VecDeque::new(),
            capacity: None,
            layout: None,
            scroll_pos: 0,
            repeat_scroll: Repeat::new(Duration::from_millis(300)),
            scrollable: true,
            skew: 0,
        }
    }

    pub fn push_message(&mut self, message: impl Into<BString>) {
        self.ensure_capacity(1);

        let message = message.into();

        let font = self.fonts.get(self.font);
        let new_lines: VecDeque<_> = font.lines(&message, Some(font::Overflow {
                size: self.layout().width,
                mode: font::OverflowMode::WordWrap,
            }))
            .map(|s| s.to_owned())
            .collect();

        self.messages.push_back((message, new_lines.len() as u32));
        for line in new_lines {
            self.lines.push_back(line);
        }
    }

    /// Horizontal offset added to each line.
    pub fn set_skew(&mut self, skew: i32) {
        self.skew = skew;
    }

    pub fn set_capacity(&mut self, capacity: Option<usize>) {
        assert!(capacity.is_none() || capacity.unwrap() > 0);
        self.capacity = capacity;
        self.ensure_capacity(0);
    }

    pub fn set_scrollable(&mut self, scrollable: bool) {
        if scrollable != self.scrollable {
            self.scrollable = scrollable;
            self.scroll_pos = 0;
            self.repeat_scroll.stop();
        }
    }

    fn layout(&self) -> Layout {
        self.layout.expect("Widget::init() wasn't called")
    }

    fn scroll(&mut self, scroll: Scroll) {
        self.scroll_pos += match scroll {
            Scroll::Up => -1,
            Scroll::Down => 1,
        };
        self.scroll_pos = cmp::max(cmp::min(self.scroll_pos, 0),
            self.layout().visible_line_count - self.lines.len() as i32);
    }

    fn ensure_capacity(&mut self, extra: usize) {
        if let Some(capacity) = self.capacity {
            while self.messages.len() >= capacity - extra {
                let line_count = self.messages.pop_front().unwrap().1;
                for _ in 0..line_count {
                    self.lines.pop_front().unwrap();
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Scroll {
    Up,
    Down,
}

impl Widget for MessagePanel {
    fn init(&mut self, ctx: Init) {
        let width = ctx.base.rect.width();
        let font = self.fonts.get(self.font);
        let visible_line_count = cmp::max(ctx.base.rect.height() / font.vert_advance(), 1);
        self.layout = Some(Layout {
            width,
            visible_line_count,
        });
    }

    fn handle_event(&mut self, mut ctx: HandleEvent) {
        fn scroll_intent(rect: &Rect, pos: Point) -> Scroll {
            let half_y = rect.top + rect.height() / 2;
            if pos.y < half_y {
                Scroll::Up
            } else {
                Scroll::Down
            }
        }

        fn update_cursor(ctx: &mut HandleEvent, pos: Point) {
            ctx.base.cursor = Some(match scroll_intent(&ctx.base.rect, pos) {
                Scroll::Up => Cursor::ArrowUp,
                Scroll::Down => Cursor::ArrowDown,
            });
        }

        match ctx.event {
            Event::MouseDown { pos, .. } => if self.scrollable {
                let scroll = scroll_intent(&ctx.base.rect, pos);
                self.scroll(scroll);
                update_cursor(&mut ctx, pos);
                self.repeat_scroll.start(ctx.now, scroll);
                ctx.capture();
            }
            Event::MouseUp { pos, .. } => if self.scrollable {
                update_cursor(&mut ctx, pos);
                self.repeat_scroll.stop();
                ctx.release();
            }
            Event::MouseMove { pos } => if self.scrollable {
                update_cursor(&mut ctx, pos);
            }
            Event::Tick => {
                if let Some(&scroll) = self.repeat_scroll.update_if_running(ctx.now) {
                    self.scroll(scroll);
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: Render) {
        let font = self.fonts.get(self.font);

        let vert_advance = font.vert_advance();
        let layout = self.layout();

        let base = ctx.base.unwrap();
        let mut x = base.rect.left;
        let mut y = base.rect.top;

        let end_i = self.lines.len() as i32 + self.scroll_pos;

        for i in end_i - layout.visible_line_count..end_i {
            if i >= self.lines.len() as i32 {
                break;
            }
            if i >= 0 {
                ctx.canvas.draw_text(&self.lines[i as usize], x, y, self.font, self.color,
                    &font::DrawOptions::default());
            }
            x += self.skew;
            y += vert_advance;
        }
    }
}
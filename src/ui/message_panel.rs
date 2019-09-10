use bstring::{bstr, BString};
use num_traits::clamp;
use std::cmp;
use std::collections::VecDeque;
use std::ops::Range;
use std::time::Duration;

use super::*;
use crate::graphics::color::{Rgb15, WHITE};
use crate::graphics::font::{self, FontKey, Fonts};
use crate::ui::out::OutEventData;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anchor {
    /// Lines are anchored to the top.
    Top,

    /// Lines are anchored to the bottom.
    Bottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseControl {
    /// Mouse scrolls.
    Scroll,

    /// Mouse highlights and picks on click.
    Pick,
}

pub struct MessagePanel {
    fonts: Rc<Fonts>,
    font: FontKey,
    color: Rgb15,
    highlight_color: Rgb15,
    highlighted: Option<usize>,
    messages: VecDeque<Message>,
    lines: VecDeque<Line>,
    capacity: Option<usize>,
    layout: Option<Layout>,
    /// Scrolling position. It's the number of lines scrolled up or down from the origin (0).
    /// Can be negative if in `Anchor::Bottom` mode.
    scroll_pos: i32,
    repeat_scroll: Repeat<Scroll>,
    mouse_control: MouseControl,
    skew: i32,
    anchor: Anchor,
}

impl MessagePanel {
    pub fn new(fonts: Rc<Fonts>, font: FontKey, color: Rgb15) -> Self {
        Self {
            fonts,
            font,
            color,
            highlight_color: WHITE,
            highlighted: None,
            messages: VecDeque::new(),
            lines: VecDeque::new(),
            capacity: None,
            layout: None,
            scroll_pos: 0,
            repeat_scroll: Repeat::new(Duration::from_millis(300)),
            mouse_control: MouseControl::Scroll,
            skew: 0,
            anchor: Anchor::Top,
        }
    }

    pub fn push_message(&mut self, message: impl Into<BString>) {
        self.ensure_capacity(1);

        let message = message.into();

        let font = self.fonts.get(self.font);
        let new_lines: VecDeque<_> = font.line_ranges(&message, Some(font::Overflow {
                size: self.layout().width,
                mode: font::OverflowMode::WordWrap,
            }))
            .map(|s| s.to_owned())
            .collect();

        self.messages.push_back(Message {
            text: message,
            line: new_lines.len(),
        });
        for range in new_lines {
            self.lines.push_back(Line {
                message: self.messages.len() - 1,
                range,
            });
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

    pub fn set_mouse_control(&mut self, mouse_control: MouseControl) {
        self.mouse_control = mouse_control;
    }

    pub fn set_anchor(&mut self, anchor: Anchor) {
        assert!(self.messages.is_empty(), "not supported");
        self.anchor = anchor;
    }

    pub fn set_highlight_color(&mut self, color: Rgb15) {
        self.highlight_color = color;
    }

    fn layout(&self) -> Layout {
        self.layout.expect("Widget::init() wasn't called")
    }

    /// Index of the first line of the last page. Can be negative if number of lines is less
    /// than the page len.
    fn last_page(&self) -> i32 {
        self.lines.len() as i32 - self.layout().visible_line_count
    }

    fn scroll(&mut self, scroll: Scroll) {
        self.scroll_pos += match scroll {
            Scroll::Up => -1,
            Scroll::Down => 1,
        };
        self.scroll_pos = match self.anchor {
            Anchor::Top => clamp(self.scroll_pos, 0, self.last_page()),
            Anchor::Bottom => clamp(self.scroll_pos, -self.last_page(), 0),
        };
    }

    fn ensure_capacity(&mut self, extra: usize) {
        if let Some(capacity) = self.capacity {
            while self.messages.len() >= capacity - extra {
                let line_count = self.messages.pop_front().unwrap().line;
                for _ in 0..line_count {
                    self.lines.pop_front().unwrap();
                }
            }
        }
    }

    fn scroll_for_cursor(&self, rect: &Rect, cursor_pos: Point) -> Option<Scroll> {
        if self.mouse_control != MouseControl::Scroll {
            return None;
        }
        let half_y = rect.top + rect.height() / 2;
        if cursor_pos.y < half_y {
            match self.anchor {
                Anchor::Top if self.scroll_pos > 0 => {
                    Some(Scroll::Up)
                }
                Anchor::Bottom if self.scroll_pos > -self.last_page() => {
                    Some(Scroll::Up)
                }
                _ => None
            }
        } else {
            match self.anchor {
                Anchor::Top if self.scroll_pos < self.last_page() => {
                    Some(Scroll::Down)
                }
                Anchor::Bottom if self.scroll_pos < 0 => {
                    Some(Scroll::Down)
                }
                _ => None
            }
        }
    }

    fn cursor(&self, scroll: Option<Scroll>) -> Option<Cursor> {
        scroll.map(|s| match s {
            Scroll::Up => Cursor::ArrowUp,
            Scroll::Down => Cursor::ArrowDown,
        })
    }

    fn update_cursor(&self, ctx: &mut HandleEvent) {
        let scroll = self.scroll_for_cursor(&ctx.base.rect, ctx.cursor_pos);
        ctx.base.cursor = self.cursor(scroll);
    }
}

struct Line {
    message: usize,
    range: Range<usize>,
}

impl Line {
    pub fn message_str<'a>(&self, messages: &'a VecDeque<Message>) -> &'a bstr {
        &messages[self.message].text[self.range.clone()]
    }
}

struct Message {
    text: BString,
    line: usize,
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
        match ctx.event {
            Event::MouseDown { .. } => {
                if let Some(scroll) = self.scroll_for_cursor(&ctx.base.rect, ctx.cursor_pos) {
                    self.scroll(scroll);
                    self.update_cursor(&mut ctx);
                    self.repeat_scroll.start(ctx.now, scroll);
                }
                ctx.capture();
            }
            Event::MouseUp { .. } => {
                ctx.release();
                match self.mouse_control {
                    MouseControl::Scroll => {
                        self.repeat_scroll.stop();
                    }
                    MouseControl::Pick => {
                        if let Some(highlighted) = self.highlighted {
                            ctx.out.push(OutEvent {
                                source: ctx.this,
                                data: OutEventData::Pick { id: highlighted as u32 },
                            });
                        }
                    }
                }
            }
            Event::MouseMove { .. } => match self.mouse_control {
                MouseControl::Scroll => self.update_cursor(&mut ctx),
                MouseControl::Pick => {
                    assert_eq!(self.scroll_pos, 0);
                    self.highlighted = if ctx.base.rect.contains(ctx.cursor_pos.x, ctx.cursor_pos.y) {
                        let font = self.fonts.get(self.font);
                        let i = (ctx.cursor_pos.y - ctx.base.rect.top) / font.vert_advance()
                            + if self.anchor == Anchor::Bottom { self.last_page() } else { 0 };
                        if i >= 0 && i < self.lines.len() as i32 {
                            Some(self.lines[i as usize].message)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                }
            }
            Event::Tick => {
                if let Some(&scroll) = self.repeat_scroll.update_if_running(ctx.now) {
                    self.scroll(scroll);
                    self.update_cursor(&mut ctx);
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

        let end_i = self.scroll_pos + match self.anchor {
            Anchor::Top => layout.visible_line_count,
            Anchor::Bottom => self.lines.len() as i32,
        };

        for i in end_i - layout.visible_line_count..end_i {
            if i >= self.lines.len() as i32 {
                break;
            }
            if i >= 0 {
                let line = &self.lines[i as usize];
                let s = line.message_str(&self.messages);
                let color = if Some(line.message) == self.highlighted {
                    self.highlight_color
                } else {
                    self.color
                };
                ctx.canvas.draw_text(s, x, y, self.font, color,
                    &font::DrawOptions::default());
            }
            x += self.skew;
            y += vert_advance;
        }
    }
}
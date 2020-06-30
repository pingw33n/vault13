use bstring::{bstr, BString};
use std::cmp;
use std::time::{Duration, Instant};

use crate::game::object;
use crate::graphics::{Point, Rect};
use crate::graphics::color::*;
use crate::graphics::font::{self, FontKey, Fonts};
use crate::graphics::render::{Canvas, Outline};

#[derive(Clone, Debug)]
pub struct Options {
    pub font_key: FontKey,
    pub color: Rgb15,
    pub outline_color: Option<Rgb15>,
}

pub(in super) struct FloatingText {
    pub obj: Option<object::Handle>,
    /// (line, width)
    lines: Vec<(BString, i32)>,
    width: i32,
    height: i32,
    vert_advance: i32,
    options: Options,
    time: Instant,
}

impl FloatingText {
    pub fn new(obj: Option<object::Handle>,
        text: &bstr,
        fonts: &Fonts,
        options: Options,
        time: Instant,
    ) -> Self {
        let font = fonts.get(options.font_key);

        let lines: Vec<_> = font.lines(text, Some(font::Overflow {
                size: 200,
                boundary: font::OverflowBoundary::Word,
                action: font::OverflowAction::Wrap,
            }))
            .map(|l| (l.to_owned(), font.line_width(l)))
            .collect();
        let vert_advance = font.vert_advance() + 1;
        let mut width = lines.iter().map(|&(_, w)| w).max().unwrap();
        let mut height = lines.len() as i32 * vert_advance;
        if options.outline_color.is_some() {
            width += 2;
            height += 2;
        }
        Self {
            obj,
            lines,
            width,
            height,
            vert_advance,
            options,
            time,
        }
    }

    pub fn render(&self, pos: Point, rect: Rect, canvas: &mut dyn Canvas) {
        let pos = Self::fit(
            Rect::with_size(pos.x - self.width / 2, pos.y - self.height,
                self.width, self.height),
            rect);

        let mut y = pos.y;
        for (line, line_width) in &self.lines {
            let x = pos.x + (self.width - *line_width) / 2;
            canvas.draw_text(line, Point::new(x, y), self.options.font_key, self.options.color,
                &font::DrawOptions {
                    outline: self.options.outline_color
                        .map(|color| Outline::Fixed { color, trans_color: None }),
                    ..Default::default()
                });
            y += self.vert_advance;
        }
    }

    pub fn expires_at(&self, initial_delay: Duration, per_line_delay: Duration) -> Instant {
        let d = initial_delay + per_line_delay * self.lines.len() as u32;
        self.time + d
    }

    fn fit(rect: Rect, bound_rect: Rect) -> Point {
        #[inline(always)]
        fn fit0(lo: i32, hi: i32, bound_lo: i32, bound_hi: i32,
            lo_max: i32, hi_max: i32) -> i32
        {
            if bound_lo - lo > 0 {
                lo + cmp::min(bound_lo - lo, lo_max)
            } else if hi - bound_hi > 0 {
                lo - cmp::min(hi - bound_hi, hi_max)
            } else {
                lo
            }
        }
        let x = fit0(rect.left, rect.right, bound_rect.left, bound_rect.right,
            rect.width() / 2, rect.width() / 2);
        let y = fit0(rect.top, rect.bottom, bound_rect.top, bound_rect.bottom,
            rect.height(), 0);
        Point::new(x, y)
    }
}
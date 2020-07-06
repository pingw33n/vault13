use bstring::BString;
use std::collections::HashMap;

use crate::asset::frame::FrameId;
use crate::graphics::{Point, Rect};

use super::*;

pub struct ImageText {
    fid: FrameId,
    chars: HashMap<u8, Rect>,
    text: BString,
}

impl ImageText {
    pub fn standard_digits(fid: FrameId, width: i32) -> Self {
        let mut chars = HashMap::new();
        for (i, c) in (b'0'..=b'9').enumerate() {
            let i = i as i32;
            chars.insert(c, Rect::with_size(width * i, 0, width, 0xffff));
        }
        Self {
            fid,
            chars,
            text: BString::new(),
        }
    }

    pub fn big_numbers() -> Self {
        Self::standard_digits(FrameId::BIG_NUMBERS, 14)
    }

    pub fn text(&self) -> &BString {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut BString {
        &mut self.text
    }
}

impl Widget for ImageText {
    fn render(&mut self, ctx: Render) {
        let frm = ctx.frm_db.get(self.fid).unwrap();
        let tex = &frm.first().texture;
        let base_rect = ctx.base.unwrap().rect;
        let mut x = base_rect.left;
        for &c in &self.text {
            if let Some(&rect) = self.chars.get(&c) {
                ctx.canvas.set_clip_rect(Rect::with_size(x, 0, rect.width(), rect.height()));
                let pos = Point::new(x - rect.left, base_rect.top);
                ctx.canvas.draw(tex, pos, 0x10000);
                x += rect.width();
            }
        }
        ctx.canvas.reset_clip_rect();
    }
}
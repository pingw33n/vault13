use bstring::BString;

use crate::graphics::color::Rgb15;
use crate::graphics::font::{DrawOptions, FontKey, HorzAlign, VertAlign};
use super::*;

#[derive(Clone, Debug)]
pub struct Text {
    pub text: BString,
    pub font: FontKey,
    pub color: Rgb15,
    pub options: DrawOptions,
}

/// Simple widget that can be used to utilize base widget features (background, cursor etc) and
/// to draw text.
pub struct Panel {
    text: Option<Text>,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            text: None,
        }
    }

    pub fn text(&self) -> Option<&Text> {
        self.text.as_ref()
    }

    pub fn text_mut(&mut self) -> Option<&mut Text> {
        self.text.as_mut()
    }

    pub fn set_text(&mut self, text: Option<Text>) {
        self.text = text;
    }
}

impl Widget for Panel {
    fn render(&mut self, ctx: Render) {
        if let Some(text) = self.text() {
            let rect = ctx.base.unwrap().rect;
            let x = match text.options.horz_align  {
                HorzAlign::Left => rect.left,
                HorzAlign::Center => rect.center().x,
                HorzAlign::Right => rect.right,
            };
            let y = match text.options.vert_align  {
                VertAlign::Top => rect.top,
                VertAlign::Middle => rect.center().y,
                VertAlign::Bottom => rect.bottom,
            };

            let mut options = text.options.clone();
            if let Some(o) = options.horz_overflow.as_mut()
                && o.size == 0
            {
                o.size = ctx.base.unwrap().rect.width();
            }
            ctx.canvas.draw_text(
                &text.text,
                Point::new(x, y),
                text.font,
                text.color,
                &options);
        }
    }
}
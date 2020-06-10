use bstring::BString;

use crate::graphics::color::Rgb15;
use crate::graphics::font::{DrawOptions, FontKey};
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
            ctx.canvas.draw_text(
                &text.text,
                ctx.base.unwrap().rect.top_left(),
                text.font,
                text.color,
                &text.options);
        }
    }
}
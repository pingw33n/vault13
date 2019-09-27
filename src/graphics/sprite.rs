use enum_map::EnumMap;
use enum_map_derive::Enum;
use std::rc::Rc;

use crate::asset::frame::{FrameId, FrameDb};
use crate::graphics::color::*;
use crate::graphics::geometry::hex::Direction;
use crate::graphics::{Point, Rect};
use crate::graphics::render::{Canvas, Outline, TextureHandle};

#[derive(Clone, Debug)]
pub struct FrameSet {
    pub fps: u16,
    pub action_frame: u16,
    pub frame_lists: EnumMap<Direction, FrameList>,
}

impl FrameSet {
    pub fn first(&self) -> &Frame {
        &self.frame_lists[Direction::NE].frames[0]
    }
}

#[derive(Clone, Debug)]
pub struct FrameList {
    pub center: Point,
    pub frames: Vec<Frame>,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub shift: Point,
    pub width: i32,
    pub height: i32,
    pub texture: TextureHandle,
    pub mask: Mask,
}

impl Frame {
    pub fn size(&self) -> Point {
        Point {
            x: self.width,
            y: self.height,
        }
    }

    pub fn bounds_centered(&self, p: Point, center: Point) -> Rect {
        let p = p + center;
        Rect {
            left: p.x - self.width / 2,
            top: p.y - self.height + 1,
            right: p.x + self.width / 2,
            bottom: p.y + 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mask {
    bitmask: Rc<[u8]>,
    width: i32,
}

impl Mask {
    pub fn new(width: i32, pixels: &[u8]) -> Self {
        let pix_len = pixels.len();
        assert_eq!(pix_len as i32 % width, 0);

        // Convert to bit mask.

        #[inline(always)]
        fn bit(b: u8) -> u8 {
            (b != 0) as u8
        }

        #[inline(always)]
        fn byte(pixels: &[u8], i: usize) -> u8 {
            bit(pixels[i + 0]) << 0 |
                bit(pixels[i + 1]) << 1 |
                bit(pixels[i + 2]) << 2 |
                bit(pixels[i + 3]) << 3 |
                bit(pixels[i + 4]) << 4 |
                bit(pixels[i + 5]) << 5 |
                bit(pixels[i + 6]) << 6 |
                bit(pixels[i + 7]) << 7
        }

        let mut bitmask = vec![0; (pix_len + 7) / 8];
        let mut i = 0;
        let mut j = 0;
        let end_i = pix_len / 8 * 8;
        while i < end_i {
            bitmask[j] = byte(pixels, i);
            i += 8;
            j += 1;
        }
        if end_i < pix_len {
            let mut p = [0; 8];
            let mut i = end_i;
            loop {
                p[0] = pixels[i]; i += 1;
                if i < pix_len { p[1] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[2] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[3] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[4] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[5] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[6] = pixels[i]; i += 1; } else { break; }
                if i < pix_len { p[7] = pixels[i]; }
                break;
            }
            bitmask[j] = byte(&p, 0);
        }


        Self {
            bitmask: bitmask.into(),
            width,
        }
    }

    #[must_use]
    pub fn test(&self, point: Point) -> bool {
        let Point { x, y } = point.into();
        let i = x + y * self.width;
        let bit = i % 8;
        let i = i as usize / 8;
        self.bitmask[i] & (1 << bit) != 0
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum Translucency {
    Energy,
    Glass,
    Red,
    Steam,
    Wall,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum OutlineStyle {
    GlowingRed,
    Red,
    Gray,
    GlowingGreen,
    Yellow,
    Brown,
    Purple,
}

#[derive(Clone, Copy, Debug)]
pub enum Effect {
    Translucency(Translucency),
    Masked {
        mask_pos: Point,
        mask_fid: FrameId,
    },
    Highlight {
        color: Rgb15,
    },
    Outline {
        style: OutlineStyle,
        translucent: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub pos: Point,
    pub centered: bool,
    pub fid: FrameId,
    pub frame_idx: usize,
    pub direction: Direction,
    pub light: u32,
    pub effect: Option<Effect>,
}

impl Sprite {
    pub fn new(fid: FrameId) -> Self {
        Sprite {
            pos: Point::new(0, 0),
            centered: false,
            fid,
            frame_idx: 0,
            direction: Direction::NE,
            light: 0x10000,
            effect: None,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, frm_db: &FrameDb) -> Rect {
        let frms = frm_db.get(self.fid).unwrap();
        let frml = &frms.frame_lists[self.direction];
        let frm = &frml.frames[self.frame_idx];

        let bounds = if self.centered {
            frm.bounds_centered(self.pos, frml.center)
        } else {
            Rect::with_size(self.pos.x, self.pos.y, frm.width, frm.height)
        };

        match self.effect {
            Some(Effect::Translucency(trans)) => {
                let color = match trans {
                    Translucency::Energy => TRANS_ENERGY,
                    Translucency::Glass => TRANS_GLASS,
                    Translucency::Red => TRANS_RED,
                    Translucency::Steam => TRANS_STEAM,
                    Translucency::Wall => TRANS_WALL,
                };
                canvas.draw_translucent_dark(&frm.texture, bounds.left, bounds.top, color,
                    self.light);
            }
            Some(Effect::Masked { mask_pos, mask_fid }) => {
                let mask_frms = frm_db.get(mask_fid).unwrap();
                let mask_frml = &mask_frms.frame_lists[Direction::NE];
                let mask_frm = &mask_frml.frames[0];
                let mask_bounds = mask_frm.bounds_centered(mask_pos, mask_frml.center);
                canvas.draw_masked(&frm.texture, bounds.left, bounds.top,
                    &mask_frm.texture, mask_bounds.left, mask_bounds.top,
                    self.light);
            }
            Some(Effect::Highlight { color }) => {
                canvas.draw_highlight(color, bounds.left, bounds.top, &frm.texture);
            }
            Some(Effect::Outline { style, translucent }) => {
                use self::OutlineStyle::*;
                let trans_color = if translucent { Some(()) } else { None };
                let outline = match style {
                    GlowingRed => GLOWING_RED_OUTLINE,
                    Red => Outline::Fixed {
                        color: Rgb15::from_packed(0x7c00),
                        trans_color: trans_color.map(|_| TRANS_RED),
                    },
                    Gray => Outline::Fixed {
                        color: Rgb15::from_packed(0x3def),
                        trans_color: trans_color.map(|_| TRANS_WALL),
                    },
                    GlowingGreen => GLOWING_GREEN_OUTLINE,
                    Yellow => Outline::Fixed {
                        color: Rgb15::from_packed(0x77a8),
                        trans_color: trans_color.map(|_| TRANS_RED),
                    },
                    Brown => Outline::Fixed {
                        // Originally this was color index 61, do we need support this?
                        color: Rgb15::from_packed(0x5226),
                        trans_color: None,
                    },
                    Purple => Outline::Fixed {
                        color: Rgb15::from_packed(0x7c1f),
                        trans_color: None,
                    },
                };
                canvas.draw_outline(&frm.texture, bounds.left, bounds.top, outline);
            }
            None => canvas.draw(&frm.texture, bounds.left, bounds.top, self.light),
        }

        bounds
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(test)]
    mod mask {
        use super::*;

        #[test]
        fn test() {
            let mask = Mask::new(3, &[0, 1, 0, 2, 0, 100]);
            assert_eq!(&*mask.bitmask, &[0b101010]);
            assert_eq!(mask.test((0, 0).into()), false);
            assert_eq!(mask.test((1, 0).into()), true);
            assert_eq!(mask.test((0, 1).into()), true);
            assert_eq!(mask.test((1, 1).into()), false);
        }
    }
}
use enum_map::EnumMap;
use enum_map_derive::Enum;

use crate::asset::frm::{Fid, FrmDb};
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
        mask_fid: Fid,
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
    pub fid: Fid,
    pub frame_idx: usize,
    pub direction: Direction,
    pub light: u32,
    pub effect: Option<Effect>,
}

impl Sprite {
    pub fn new(fid: Fid) -> Self {
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

    pub fn render(&self, canvas: &mut Canvas, frm_db: &FrmDb) -> Rect {
        let frms = frm_db.get(self.fid);
        let frml = &frms.frame_lists[self.direction];
        let frm = &frml.frames[self.frame_idx];

        let bounds = if self.centered {
            Self::bounds_centered(self.pos, frml.center, frm)
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
                let mask_frms = frm_db.get(mask_fid);
                let mask_frml = &mask_frms.frame_lists[Direction::NE];
                let mask_frm = &mask_frml.frames[0];
                let mask_bounds = Self::bounds_centered(mask_pos, mask_frml.center, &mask_frm);
                canvas.draw_masked(&frm.texture, bounds.left, bounds.top,
                    &mask_frm.texture, mask_bounds.left, mask_bounds.top,
                    self.light);
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

    fn bounds_centered(p: Point, center: Point, frm: &Frame) -> Rect {
        let p = p + center;
        Rect {
            left: p.x - frm.width / 2,
            top: p.y - frm.height + 1,
            right: p.x + frm.width / 2,
            bottom: p.y + 1,
        }
    }
}
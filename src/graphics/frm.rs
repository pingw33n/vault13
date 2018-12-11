use enum_map::EnumMap;

use asset::frm::{Fid, FrmDb};
use graphics::color::*;
use graphics::geometry::Direction;
use graphics::{Point, Rect};
use graphics::render::{Renderer, TextureHandle};

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

#[derive(Clone, Copy, Debug)]
pub enum Translucency {
    Energy,
    Glass,
    Red,
    Steam,
    Wall,
}

#[derive(Clone, Copy, Debug)]
pub enum Effect {
    Translucency(Translucency),
    Masked {
        mask_pos: Point,
        mask_fid: Fid,
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
    pub fn render(&self, renderer: &mut Renderer, frm_db: &FrmDb) -> Rect {
        let frms = frm_db.get(self.fid);
        let frml = &frms.frame_lists[self.direction];
        let frm = &frml.frames[self.frame_idx];

        fn compute_bounds(p: Point, center: Point, frm: &Frame) -> Rect {
            let p = p + center;
            Rect {
                left: p.x - frm.width / 2,
                top: p.y - frm.height + 1,
                right: p.x + frm.width / 2,
                bottom: p.y + 1,
            }
        }

        let bounds = if self.centered {
            compute_bounds(self.pos, frml.center, frm)
        } else {
            Rect::with_size(self.pos.x, self.pos.y, frm.width, frm.height - 1)
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
                renderer.draw_translucent_dark(&frm.texture, bounds.left, bounds.top, color,
                    self.light);
            }
            Some(Effect::Masked { mask_pos, mask_fid }) => {
                let mask_frms = frm_db.get(mask_fid);
                let mask_frml = &mask_frms.frame_lists[Direction::NE];
                let mask_frm = &mask_frml.frames[0];
                let mask_bounds = compute_bounds(mask_pos, mask_frml.center, &mask_frm);
                renderer.draw_masked(&frm.texture, bounds.left, bounds.top,
                    &mask_frm.texture, mask_bounds.left, mask_bounds.top,
                    self.light);
            }
            None => renderer.draw(&frm.texture, bounds.left, bounds.top, self.light),
        }

        bounds
    }
}
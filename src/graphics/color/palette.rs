pub mod overlay;

use super::*;

#[derive(Clone)]
pub struct Palette {
    color_idx_to_rgb18: [Rgb18; 256],
    rgb15_to_color_idx: [u8; 32768],
    mapped_colors: [bool; 256],
}

impl Palette {
    pub fn new(color_idx_to_rgb18: [Rgb18; 256], rgb15_to_color_idx: [u8; 32768],
               mapped_colors: [bool; 256]) -> Self {
        Self {
            color_idx_to_rgb18,
            rgb15_to_color_idx,
            mapped_colors,
        }
    }

    pub fn rgb<P: ColorPrecision>(&self, color_idx: u8) -> Rgb<P> {
        self.rgb18(color_idx).scale()
    }

    pub fn rgb15(&self, color_idx: u8) -> Rgb15 {
        self.rgb::<Color5>(color_idx)
    }

    pub fn rgb18(&self, color_idx: u8) -> Rgb18 {
        self.color_idx_to_rgb18[color_idx as usize]
    }

//    pub fn rgb24(&self, color_idx: u8) -> Rgb24 {
//        self.rgb::<Color8>(color_idx)
//    }

    pub fn color_idx<P: ColorPrecision>(&self, rgb: Rgb<P>) -> u8 {
        self.rgb15_to_color_idx[rgb.scale::<Color5>().pack() as usize]
    }

    pub fn quantize<P: ColorPrecision>(&self, rgb: Rgb<P>) -> Rgb<P> {
        self.rgb(self.color_idx(rgb))
    }

    pub fn darken(&self, color_idx: u8, amount: u8) -> u8 {
        if self.mapped_colors[color_idx as usize] {
            self.color_idx(self.rgb15(color_idx).darken(amount))
        } else {
            color_idx
        }
    }

    pub fn lighten(&self, color_idx: u8, amount: u8) -> u8 {
        if self.mapped_colors[color_idx as usize] {
            self.color_idx(self.rgb15(color_idx).lighten(amount))
        } else {
            color_idx
        }
    }

    pub fn blend(&self, color_idx1: u8, color_idx2: u8) -> u8 {
        let c1 = self.rgb15(color_idx1);
        let c2 = self.rgb15(color_idx2);
        self.color_idx(c1.blend(c2, |c| self.quantize(c)))
    }

//    pub fn grayscale(&self, color_idx: u8) -> u8 {
//        self.color_idx(self.rgb15(color_idx).grayscale())
//    }
//
//    pub fn grayscale_dark(&self, color_idx: u8) -> u8 {
//        self.color_idx(self.rgb15(color_idx).grayscale_dark())
//    }

    // alpha is [0..7]
    pub fn alpha_blend(&self, color_idx1: u8, color_idx2: u8, alpha: u8) -> u8 {
        let a = self.rgb15(color_idx1);
        let x = self.rgb15(color_idx2);
        let c = a.alpha_blend(x, alpha);
//        println!("{:x} {:x} {:x}", a, x, c);
        self.color_idx(c)
    }
}


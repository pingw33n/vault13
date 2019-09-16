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

    // alpha is [0..7]
    pub fn alpha_blend(&self, color_idx1: u8, color_idx2: u8, alpha: u8) -> u8 {
        let c1 = self.rgb15(color_idx1);
        let c2 = self.rgb15(color_idx2);
        let r = c1.alpha_blend(c2, alpha);
        self.color_idx(r)
    }

    /// Simulates color blend table lookup as it's done in the original.
    /// Blend table is combined from:
    /// 1. Tables for alpha blending `color_idx` into `base_color_idx` when `x` goes
    ///    from 0 (opaque `color_idx`) to 7 (opaque `base_color_idx`).
    /// 2. Tables for lightening/darkening the `base_color_idx`. When `x == 8` the `base_color_idx`
    ///    is darkened with amount 127. When `x` is in [9..15] lightening effect is applied with the
    ///    [9..14] range mapped to [18..237] and 15 mapped to 18 (wrapped).
    ///    TODO The darkening looks like a bug, likely darkening isn't desired at all and only
    ///    lightening should be applied for the [9..15] range. The primary usage of the full blend
    ///    table is for screen glare effect in dialog window.
    pub fn blend_lookup(&self, base_color_idx: u8, color_idx: u8, x: u8) -> u8 {
        match x {
            0 => color_idx,
            1...7 => self.alpha_blend(color_idx, base_color_idx, 7 - x),
            amount => {
                let amount = amount - 8;
                let amount = ((amount as u32 * 0x10000 / 7 + 0xffff) >> 9) as u8;
                if amount <= 128 {
                    self.darken(base_color_idx, amount)
                } else {
                    self.lighten(base_color_idx, amount - 128)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::test::ungz;

    fn palette() -> Palette {
        let data = ungz(include_bytes!("color.pal.gz"));
        crate::asset::palette::read_palette(&mut std::io::Cursor::new(&data[..])).unwrap()
    }

    #[test]
    fn color_idx() {
        let exp = ungz(include_bytes!("expected_rgb15_to_color_idx.bin.gz"));
        let pal = palette();
        for rgb15 in 0..0x8000 {
            assert_eq!(pal.color_idx(Rgb15::from_packed(rgb15)), exp[rgb15 as usize]);
        }
    }

    #[test]
    fn blend_table() {
        let pal = palette();
        let exp = ungz(include_bytes!("expected_blend_table_4631.bin.gz"));

        let base_idx = pal.color_idx(Rgb15::from_packed(0x4631));

        for x in 0..8 {
            for c in 0..=255 {
                assert_eq!(pal.blend_lookup(base_idx, c,  x),
                    exp[x as usize * 256 + c as usize], "{} {}", x, c);
            }
        }
    }


}
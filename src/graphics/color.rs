pub mod palette;

use std::cmp;
use std::fmt;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use crate::graphics::render::Outline;

macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        Rgb { r: $r, g: $g, b: $b, _p: PhantomData }
    };
}

pub const TRANS_WALL    : Rgb15 = rgb!(24, 26, 31);
pub const TRANS_GLASS   : Rgb15 = rgb!(9, 31, 31);
pub const TRANS_STEAM   : Rgb15 = rgb!(31, 31, 31);
pub const TRANS_ENERGY  : Rgb15 = rgb!(31, 31, 1);
pub const TRANS_RED     : Rgb15 = rgb!(31, 0, 1);
pub const RED           : Rgb15 = rgb!(31, 0, 0);
pub const WHITE         : Rgb15 = rgb!(31, 31, 31);
pub const BLACK         : Rgb15 = rgb!(0, 0, 0);
pub const GREEN         : Rgb15 = rgb!(0, 31, 0);
pub const BLUE          : Rgb15 = rgb!(0, 0, 31);

pub const GLOWING_RED_OUTLINE : Outline = Outline::Cycled {
    start: SLIME_PALETTE_START, len: SLIME_LEN as u8 };
pub const GLOWING_GREEN_OUTLINE : Outline = Outline::Cycled {
    start: FAST_FIRE_PALETTE_START, len: FAST_FIRE_LEN as u8 };

const SLIME: [Rgb18; SLIME_LEN] = [
    rgb!(0, 27, 0),
    rgb!(2, 28, 1),
    rgb!(6, 30, 3),
    rgb!(10, 32, 6),
];
const SLIME_LEN: usize = 4;
const SLIME_PERIOD_MILLIS: u64 = 200;
const SLIME_PALETTE_START: u8 = 229;

const SHORE: [Rgb18; SHORE_LEN] = [
    rgb!(20, 15, 10),
    rgb!(18, 14, 10),
    rgb!(16, 13, 9),
    rgb!(15, 12, 9),
    rgb!(13, 11, 8),
    rgb!(12, 10, 8),
];
const SHORE_LEN: usize = 6;
const SHORE_PERIOD_MILLIS: u64 = 200;
const SHORE_PALETTE_START: u8 = 248;

const SLOW_FIRE: [Rgb18; SLOW_FIRE_LEN] = [
    rgb!(63, 0, 0),
    rgb!(53, 0, 0),
    rgb!(36, 10, 2),
    rgb!(63, 29, 0),
    rgb!(63, 14, 0),
];
const SLOW_FIRE_LEN: usize = 5;
const SLOW_FIRE_PERIOD_MILLIS: u64 = 200;
const SLOW_FIRE_PALETTE_START: u8 = 238;

const FAST_FIRE: [Rgb18; FAST_FIRE_LEN] = [
    rgb!(17, 0, 0),
    rgb!(30, 0, 0),
    rgb!(44, 0, 0),
    rgb!(30, 0, 0),
    rgb!(17, 0, 0),
];
const FAST_FIRE_LEN: usize = 5;
const FAST_FIRE_PERIOD_MILLIS: u64 = 142;
const FAST_FIRE_PALETTE_START: u8 = 243;

const COMPUTER_SCREEN: [Rgb18; COMPUTER_SCREEN_LEN] = [
    rgb!(26, 26, 27),
    rgb!(24, 25, 31),
    rgb!(21, 26, 35),
    rgb!(0, 36, 40),
    rgb!(26, 46, 63),
];
const COMPUTER_SCREEN_LEN: usize = 5;
const COMPUTER_SCREEN_PERIOD_MILLIS: u64 = 100;
const COMPUTER_SCREEN_PALETTE_START: u8 = 233;

const ALARM_PERIOD_MILLIS: u64 = 33;
const ALARM_PALETTE_START: u8 = 254;

pub trait ColorPrecision: Clone + Copy + Eq + PartialEq + Ord + PartialOrd {
    const BITS: u32;
    const MASK: u32 = (1 << Self::BITS) - 1;
    const MAX: u8 = Self::MASK as u8;

    #[inline(always)]
    fn scale<P: ColorPrecision>(v: u8) -> u8 {
        if P::BITS > Self::BITS {
            v << (P::BITS - Self::BITS)
        } else {
            v >> (Self::BITS - P::BITS)
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Color5;
impl ColorPrecision for Color5 {
    const BITS: u32 = 5;
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Color6;
impl ColorPrecision for Color6 {
    const BITS: u32 = 6;
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Color8;
impl ColorPrecision for Color8 {
    const BITS: u32 = 8;
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rgb<P: ColorPrecision> {
    r: u8,
    g: u8,
    b: u8,
    _p: PhantomData<P>,
}

impl<P: ColorPrecision> Rgb<P> {
    #[inline(always)]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        debug_assert!(r <= P::MAX);
        debug_assert!(g <= P::MAX);
        debug_assert!(b <= P::MAX);
        Self {
            r,
            g,
            b,
            _p: PhantomData,
        }
    }

    #[inline(always)]
    pub fn black() -> Self {
        Self::new(0, 0, 0)
    }

    #[inline(always)]
    pub fn from_packed(rgb: u32) -> Self {
        Self::new(
            (rgb >> (P::BITS * 2)) as u8,
            (rgb >> P::BITS & P::MASK) as u8,
            (rgb & P::MASK) as u8)
    }

    #[inline(always)]
    pub fn pack(self) -> u32 {
        (self.r as u32) << (P::BITS * 2) |
            (self.g as u32) << P::BITS |
            (self.b as u32)
    }

    #[inline(always)]
    pub fn scale<O: ColorPrecision>(self) -> Rgb<O> {
        Rgb::new(
            self.r_::<O>(),
            self.g_::<O>(),
            self.b_::<O>())
    }

    #[inline(always)]
    pub fn r(self) -> u8 {
        self.r
    }

    #[inline(always)]
    pub fn g(self) -> u8 {
        self.g
    }

    #[inline(always)]
    pub fn b(self) -> u8 {
        self.b
    }

    #[inline(always)]
    pub fn r_<O: ColorPrecision>(self) -> u8 {
        P::scale::<O>(self.r)
    }

    #[inline(always)]
    pub fn g_<O: ColorPrecision>(self) -> u8 {
        P::scale::<O>(self.g)
    }

    #[inline(always)]
    pub fn b_<O: ColorPrecision>(self) -> u8 {
        P::scale::<O>(self.b)
    }

    #[inline(always)]
    pub fn colors(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    #[inline(always)]
    pub fn colors_u32(self) -> (u32, u32, u32) {
        (self.r as u32, self.g as u32, self.b as u32)
    }

    // amount is [0..128], 0 - fully black, 128 - original color.
    #[inline(always)]
    pub fn darken(self, amount: u8) -> Self {
        assert!(amount <= 128);
        let (r, g, b) = self.colors_u32();
        let f = (amount as u32) << 9;
        let (r, g, b) = (
            (f * r) >> 16,
            (f * g) >> 16,
            (f * b) >> 16);
        Self::new(r as u8, g as u8, b as u8)
    }

    // amount is [0..128], 0 - original color, 128 - fully white.
    #[inline(always)]
    pub fn lighten(self, amount: u8) -> Self {
        assert!(amount <= 128);
        let (r, g, b) = self.colors_u32();
        let f = (amount as u32) << 9;
        let (r, g, b) = (
            r + ((f * (P::MASK - r)) >> 16),
            g + ((f * (P::MASK - g)) >> 16),
            b + ((f * (P::MASK - b)) >> 16));
        Self::new(r as u8, g as u8, b as u8)
    }

    // additive blending
    #[inline(always)]
    pub fn blend(self, other: Self, q: impl FnOnce(Self) -> Self) -> Self {
        let (r, g, b) = self.colors();
        let (or, og, ob) = other.colors();
        let mix_r = r + or;
        let mix_g = g + og;
        let mix_b = b + ob;
        let max = cmp::max(mix_r, cmp::max(mix_g, mix_b));
        if max <= 31 {
           Self::new(mix_r, mix_g, mix_b)
        } else {
            let shift = max - P::MAX;
            let mix_r = cmp::max(mix_r as i32 - shift as i32, 0) as u8;
            let mix_g = cmp::max(mix_g as i32 - shift as i32, 0) as u8;
            let mix_b = cmp::max(mix_b as i32 - shift as i32, 0) as u8;
            let mixed = Self::new(mix_r, mix_g, mix_b);
            let mixed = q(mixed);
            mixed.lighten(shift)
        }
    }

    #[inline(always)]
    pub fn grayscale(self) -> u8 {
        let (r, g, b) = self.colors_u32();
        ((3 * r + 6 * g + b) / 10) as u8
    }

    #[inline(always)]
    pub fn grayscale_dark(self) -> u8 {
        let (r, g, b) = self.colors_u32();
        ((r + 5 * g + 4 * b) / 10) as u8
    }

    // alpha is [0..7], alpha 0 - opaque other, alpha 7 - opaque self
    #[inline(always)]
    pub fn alpha_blend(self, other: Self, alpha: u8) -> Self {
        assert!(alpha < 8);
        let (r, g, b) = self.colors_u32();
        let (or, og, ob) = other.colors_u32();
        let alpha = alpha as u32;
        Self::new(
            ((r * alpha + or * (7 - alpha)) / 7) as u8,
            ((g * alpha + og * (7 - alpha)) / 7) as u8,
            ((b * alpha + ob * (7 - alpha)) / 7) as u8)
    }
}

impl<P: ColorPrecision> fmt::Debug for Rgb<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rgb{}{:?}", P::BITS * 3, self.colors())
    }
}

impl<P: ColorPrecision> fmt::LowerHex for Rgb<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rgb{}(0x{:x})", P::BITS * 3, self.pack())
    }
}

impl<P: ColorPrecision> fmt::UpperHex for Rgb<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rgb{}(0x{:X})", P::BITS * 3, self.pack())
    }
}

pub type Rgb15 = Rgb<Color5>;
pub type Rgb18 = Rgb<Color6>;
pub type Rgb24 = Rgb<Color8>;

#[cfg(test)]
mod test_rgb {
    use super::*;

    fn rgb15(r: u8, g: u8, b: u8) -> Rgb15 {
        Rgb::new(r, g, b)
    }

    #[test]
    fn darken() {
        let c = rgb15(5, 10, 29);
        assert_eq!(c.darken(0), rgb15(0, 0, 0));
        assert_eq!(c.darken(64), rgb15(2, 5, 14));
        assert_eq!(c.darken(128), c);
    }

    #[test]
    fn lighten() {
        let c = rgb15(5, 10, 29);
        assert_eq!(c.lighten(0), c);
        assert_eq!(c.lighten(64), rgb15(18, 20, 30));
        assert_eq!(c.lighten(128), rgb15(31, 31, 31));
    }

    #[test]
    fn blend() {
        let d = vec![
            ([0, 0, 0], [0, 0, 0], [0, 0, 0]),
            ([15, 30, 0], [16, 1, 31], [31, 31, 31]),
            ([15, 30, 0], [16, 1, 31], [31, 31, 31]),
            ([31, 31, 31], [31, 31, 31], [31, 31, 31]),
            ([29, 29, 29], [29, 29, 29], [31, 31, 31]),
        ];
        for d in d {
            assert_eq!(rgb15(d.0[0], d.0[1], d.0[2])
                        .blend(rgb15(d.1[0], d.1[1], d.1[2]), |c| c),
                       rgb15(d.2[0], d.2[1], d.2[2]));
        }
    }

    #[test]
    fn blend_quantize() {
        assert_eq!(rgb15(20, 30, 31).blend(rgb15(12, 13, 14),
            |c| {
                let (r, g, b) = c.colors();
                rgb15(r >> 1 << 1, g >> 1 << 1, b >> 1 << 1)
            }), rgb15(19, 28, 30));
    }
}


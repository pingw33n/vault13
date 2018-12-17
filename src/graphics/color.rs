use std::cmp;
use std::fmt;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use graphics::render::Outline;

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
            v << P::BITS - Self::BITS
        } else {
            v >> Self::BITS - P::BITS
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
            (rgb >> P::BITS * 2) as u8,
            (rgb >> P::BITS & P::MASK) as u8,
            (rgb & P::MASK) as u8)
    }

    #[inline(always)]
    pub fn pack(self) -> u32 {
        (self.r as u32) << P::BITS * 2 |
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
            f * r >> 16,
            f * g >> 16,
            f * b >> 16);
        Self::new(r as u8, g as u8, b as u8)
    }

    // amount is [0..128], 0 - original color, 128 - fully white.
    #[inline(always)]
    pub fn lighten(self, amount: u8) -> Self {
        assert!(amount <= 128);
        let (r, g, b) = self.colors_u32();
        let f = (amount as u32) << 9;
        let (r, g, b) = (
            r + (f * (P::MASK - r) >> 16),
            g + (f * (P::MASK - g) >> 16),
            b + (f * (P::MASK - b) >> 16));
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

#[derive(Debug)]
pub struct PaletteOverlay {
    ranges: Vec<PaletteOverlayRange>,
}

impl PaletteOverlay {
    pub fn new(mut ranges: Vec<PaletteOverlayRange>) -> Self {
        ranges.sort_by_key(|r| r.start);
        Self {
            ranges,
        }
    }

    pub fn standard() -> Self {
        fn make_alarm_colors() -> Vec<Rgb18> {
            let mut colors = Vec::new();
            for r in 1..16 {
                colors.push(Rgb::new(r * 4, 0, 0));
            }
            for r in (0..15).rev() {
                colors.push(Rgb::new(r * 4, 0, 0));
            }
            colors
        }

        fn overlay_range<C: AsRef<[Rgb18]>>(colors: C, start: u8, period_millis: u64) -> PaletteOverlayRange {
            let colors = colors.as_ref();
            PaletteOverlayRange::new(colors.into(), start, colors.len() as u8,
                Duration::from_millis(period_millis))
        }

        let ranges = vec![
            overlay_range(SLIME, SLIME_PALETTE_START, SLIME_PERIOD_MILLIS),
            overlay_range(SHORE, SHORE_PALETTE_START, SHORE_PERIOD_MILLIS),
            overlay_range(SLOW_FIRE, SLOW_FIRE_PALETTE_START, SLOW_FIRE_PERIOD_MILLIS),
            overlay_range(FAST_FIRE, FAST_FIRE_PALETTE_START, FAST_FIRE_PERIOD_MILLIS),
            overlay_range(COMPUTER_SCREEN, COMPUTER_SCREEN_PALETTE_START, COMPUTER_SCREEN_PERIOD_MILLIS),
            PaletteOverlayRange::new(make_alarm_colors(), ALARM_PALETTE_START, 1,
                Duration::from_millis(ALARM_PERIOD_MILLIS)),
        ];
        Self::new(ranges)
    }

    pub fn get(&self, color_idx: u8) -> Option<Rgb18> {
        match self.ranges.binary_search_by(|r| {
            if color_idx < r.start as u8 {
                cmp::Ordering::Greater
            } else if color_idx < r.end() {
                cmp::Ordering::Equal
            } else {
                cmp::Ordering::Less
            }
        }) {
            Ok(i) => Some(self.ranges[i].get(color_idx)),
            Err(_) => None,
        }
    }

    pub fn rotate(&mut self, time: Instant) {
        for range in &mut self.ranges {
            range.rotate(time);
        }
    }
}

#[derive(Debug)]
struct Rotation {
    pos: u8,
    period: Duration,
    last_time: Option<Instant>,
}

impl Rotation {
    fn rotate(&mut self, time: Instant, len: u8) {
        if self.last_time.map(|lt| time - lt < self.period).unwrap_or(false) {
            return;
        }
        if self.pos == 0 {
            self.pos = len - 1;
        } else {
            self.pos -= 1;
        }
        assert!(self.last_time.is_none() || self.last_time.unwrap() <= time);
        self.last_time = Some(time);
    }
}

#[derive(Debug)]
pub struct PaletteOverlayRange {
    colors: Vec<Rgb18>,
    start: u8,
    len: u8,
    rotation: Rotation,
}

impl PaletteOverlayRange {
    pub fn new(colors: Vec<Rgb18>, start: u8, len: u8, rotation_period: Duration) -> Self {
        assert!(!colors.is_empty());
        assert!(start as u32 + len as u32 <= 256);
        assert!(len as usize <= colors.len());
        Self {
            colors,
            start,
            len,
            rotation: Rotation {
                pos: 0,
                period: rotation_period,
                last_time: None,
            }
        }
    }

    fn rotate(&mut self, time: Instant) {
        self.rotation.rotate(time, self.colors.len() as u8);
    }

    fn get(&self, color_idx: u8) -> Rgb18 {
        assert!(color_idx >= self.start && color_idx < self.end());
        self.colors[(color_idx - self.start + self.rotation.pos) as usize % self.colors.len()].scale()
    }

    fn end(&self) -> u8 {
        self.start + self.len
    }
}

#[cfg(test)]
mod test_palette_overlay {
    use super::*;

    #[test]
    fn test() {
        let mut t = PaletteOverlay::new(vec![
            PaletteOverlayRange::new(vec![Rgb18::new(1, 1, 1), Rgb18::new(2, 2, 2)], 50, 2, Duration::from_millis(100)),
            PaletteOverlayRange::new(vec![Rgb18::new(5, 5, 5), Rgb18::new(6, 6, 6)], 100, 1, Duration::from_millis(200)),
        ]);

        assert_eq!(t.get(0), None);
        assert_eq!(t.get(255), None);

        assert_eq!(t.get(49), None);
        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(52), None);

        assert_eq!(t.get(99), None);
        assert_eq!(t.get(100), Some(Rgb18::new(5, 5, 5)));
        assert_eq!(t.get(101), None);

        let tm = Instant::now();
        t.rotate(tm);

        assert_eq!(t.get(49), None);
        assert_eq!(t.get(50), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(51), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(52), None);

        assert_eq!(t.get(99), None);
        assert_eq!(t.get(100), Some(Rgb18::new(6, 6, 6)));
        assert_eq!(t.get(101), None);

        t.rotate(tm + Duration::from_millis(199));

        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(100), Some(Rgb18::new(6, 6, 6)));

        t.rotate(tm + Duration::from_millis(200));
        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(100), Some(Rgb18::new(5, 5, 5)));
    }
}
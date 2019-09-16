use byteorder::ReadBytesExt;
use std::io::{self, prelude::*};

use crate::graphics::color::Rgb;
use crate::graphics::color::palette::Palette;

pub fn read_palette(rd: &mut impl Read) -> io::Result<Palette> {
    let mut color_idx_to_rgb18 = [Rgb::black(); 256];
    let mut mapped_colors = [false; 256];
    for i in 0..256 {
        let r = rd.read_u8()?;
        let g = rd.read_u8()?;
        let b = rd.read_u8()?;
        let valid = r < 64 && b < 64 && g < 64;
        if valid {
            color_idx_to_rgb18[i] = Rgb::new(r, g, b);
        }
        mapped_colors[i] = valid;
    }

    let mut rgb15_to_color_idx = [0; 32768];
    rd.read_exact(&mut rgb15_to_color_idx[..])?;

    Ok(Palette::new(color_idx_to_rgb18, rgb15_to_color_idx, mapped_colors))
}
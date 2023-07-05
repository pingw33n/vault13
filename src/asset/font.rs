use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use log::*;
use std::cmp;
use std::io::{self, Error, ErrorKind, prelude::*};

use crate::fs::FileSystem;
use crate::graphics::font::{Font, Glyph, FontKey, Fonts};
use crate::graphics::render::TextureFactory;

fn read_aaf(rd: &mut impl Read, texture_factory: &TextureFactory) -> io::Result<Font> {
    let mut magic = [0u8; 4];
    rd.read_exact(&mut magic[..])?;
    if &magic != b"AAFF" {
        return Err(Error::new(ErrorKind::InvalidData, "no AAFF magic bytes found"));
    }

    let height = rd.read_i16::<BigEndian>()? as i32;
    let horz_spacing = rd.read_i16::<BigEndian>()? as i32;
    let space_width = rd.read_i16::<BigEndian>()? as i32;
    let vert_spacing = rd.read_i16::<BigEndian>()? as i32;

    let mut glyph_sizes = Vec::with_capacity(256);
    for _ in 0..256 {
        let width = rd.read_i16::<BigEndian>()? as i32;
        let height = rd.read_i16::<BigEndian>()? as i32;
        let _offset = rd.read_u32::<BigEndian>()?;
        glyph_sizes.push((width, height));
    }
    let mut glyphs = Vec::with_capacity(256);
    for (c, &(width, height)) in glyph_sizes.iter().enumerate() {
        let mut data = vec![0; (width * height) as usize];
        rd.read_exact(&mut data)?;
        let texture = texture_factory.new_texture(width, height, data.into_boxed_slice());
        let width = if c == b' ' as usize {
            space_width
        } else {
            width
        };
        glyphs.push(Glyph {
            width,
            height,
            texture,
        });
    }

    Ok(Font {
        height,
        horz_spacing,
        vert_spacing,
        glyphs: glyphs.into_boxed_slice(),
    })
}

fn read_fon(rd: &mut impl Read, texture_factory: &TextureFactory) -> io::Result<Font> {
    let glyph_count = rd.read_i32::<LittleEndian>()?;
    if !(0..=256).contains(&glyph_count) {
        return Err(Error::new(ErrorKind::InvalidData, "invalid glyph_count in FON file"));
    }
    let glyph_count = glyph_count as usize;
    let height = rd.read_i32::<LittleEndian>()?;
    let horz_spacing = rd.read_i32::<LittleEndian>()?;
    let _garbage = rd.read_u32::<LittleEndian>()?;
    let _garbage = rd.read_u32::<LittleEndian>()?;

    let row_bytes = |w| (w as usize + 7) / 8;
    let glyph_bytes = |w| row_bytes(w) * height as usize;

    let mut glyph_info = Vec::with_capacity(glyph_count);
    let mut data_len = 0;
    for _ in 0..glyph_count {
        let width = rd.read_i32::<LittleEndian>()?;
        let offset = rd.read_u32::<LittleEndian>()?;
        glyph_info.push((width, offset as usize));
        data_len = cmp::max(data_len, offset as usize + glyph_bytes(width));
    }

    let mut data = vec![0; data_len];
    rd.read_exact(&mut data)?;

    let mut glyphs = Vec::with_capacity(glyph_count);
    for (width, offset) in glyph_info {
        let data = &data[offset..];
        let row_len = row_bytes(width);
        let mut glyph_pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height as usize {
            let data = &data[y * row_len..];
            for x in 0..width as usize {
                let b = data[x / 8] & (1 << (7 - (x % 8))) != 0;
                glyph_pixels.push(if b { 7 } else { 0 });
            }
        }

        let texture = texture_factory.new_texture(width, height, glyph_pixels.into_boxed_slice());
        glyphs.push(Glyph {
            width,
            height,
            texture,
        });
    }

    Ok(Font {
        height,
        horz_spacing,
        vert_spacing: 0,
        glyphs: glyphs.into_boxed_slice(),
    })
}

pub fn load_fonts(fs: &FileSystem, texture_factory: &TextureFactory) -> Fonts {
    let mut fonts = Fonts::new();

    let load_fon = |name: &str| {
        let mut rd = fs.reader(name)?;
        read_fon(&mut rd, texture_factory)
    };
    for id in 0..10 {
        let name = format!("font{}.fon", id);
        match load_fon(&name) {
            Ok(font) => {
                info!("loaded FON font: {}", name);
                fonts.insert(FontKey { id, antialiased: false }, font);
            }
            Err(e) => {
                debug!("couldn't load FON font `{}`: {}", name, e);
            }
        }
    }

    let load_aaf = |name: &str| {
        let mut rd = fs.reader(name)?;
        read_aaf(&mut rd, texture_factory)
    };
    for id in 0..16 {
        let name = format!("font{}.aaf", id);
        match load_aaf(&name) {
            Ok(font) => {
                info!("loaded AAF font: {}", name);
                fonts.insert(FontKey { id, antialiased: true }, font);
            }
            Err(e) => {
                debug!("couldn't load AAF font `{}`: {}", name, e);
            }
        }
    }

    fonts
}

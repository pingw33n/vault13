mod db;
mod id;

use byteorder::{BigEndian, ReadBytesExt};
use enum_map::EnumMap;
use std::io::{self, prelude::*};

pub use id::{FrameId, Idx};
pub use db::FrameDb;

use crate::graphics::Point;
use crate::graphics::geometry::hex::Direction;
use crate::graphics::render::TextureFactory;
use crate::graphics::sprite::*;
use crate::util::EnumExt;

pub fn read_frm(rd: &mut impl Read, texture_factory: &TextureFactory) -> io::Result<FrameSet> {
    let _version = rd.read_u32::<BigEndian>()?;

    let fps = rd.read_u16::<BigEndian>()?;
    let fps = if fps == 0 {
        10
    } else {
        fps
    };

    let action_frame = rd.read_u16::<BigEndian>()?;
    let frames_per_direction = rd.read_u16::<BigEndian>()? as usize;
    assert!(frames_per_direction > 0);

    let mut centers_x = EnumMap::new();
    for dir in Direction::iter() {
        centers_x[dir] = rd.read_i16::<BigEndian>()? as i32;
    }
    let mut centers_y = EnumMap::new();
    for dir in Direction::iter() {
        centers_y[dir] = rd.read_i16::<BigEndian>()? as i32;
    }

    let mut frame_offsets = EnumMap::new();
    for dir in Direction::iter() {
        frame_offsets[dir] = rd.read_u32::<BigEndian>()?;
    }

    let _data_len = rd.read_u32::<BigEndian>()?;

    let mut loaded_offsets: EnumMap<Direction, Option<u32>> = EnumMap::new();
    let mut frame_lists: EnumMap<Direction, Option<FrameList>> = EnumMap::new();
    for dir in Direction::iter() {
        let offset = frame_offsets[dir];
        let already_loaded_dir = loaded_offsets
            .iter()
            .filter_map(|(d, o)| o.filter(|&o| o == offset).map(|_| d))
            .next();
        if let Some(already_loaded_dir) = already_loaded_dir {
            frame_lists[dir] = frame_lists[already_loaded_dir].clone();
            continue;
        }

        loaded_offsets[dir] = Some(offset);

        let mut frames = Vec::with_capacity(frames_per_direction);
        for _ in 0..frames_per_direction {
            let width = rd.read_i16::<BigEndian>()? as i32;
            let height = rd.read_i16::<BigEndian>()? as i32;
            let _len = rd.read_u32::<BigEndian>()?;
            let shift = Point::new(
                rd.read_i16::<BigEndian>()? as i32,
                rd.read_i16::<BigEndian>()? as i32,
            );

            let len = (width * height) as usize;
            let mut pixels = vec![0; len].into_boxed_slice();
            rd.read_exact(&mut pixels)?;

            let mask = Mask::new(width, &pixels);
            let texture = texture_factory.new_texture(width, height, pixels);

            frames.push(Frame {
                shift,
                width,
                height,
                texture,
                mask,
            });
        }
        frame_lists[dir] = Some(FrameList {
            center: Point::new(centers_x[dir], centers_y[dir]),
            frames,
        });
    }

    Ok(FrameSet {
        fps,
        action_frame,
        frame_lists: EnumMap::from(|k| frame_lists[k].take().unwrap()),
    })
}
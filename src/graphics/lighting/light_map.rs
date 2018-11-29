use std::cmp;

use graphics::Point;

pub const VERTEX_COUNT: usize = 10;
pub const VERTEX_HEXES: [[Point; VERTEX_COUNT]; 2] = [
    [
        Point { x: 0, y: -1}, Point { x: 1, y: -1 },
        Point { x: -1, y: -1}, Point { x: 0, y: 0}, Point { x: 1, y: 0},
        Point { x: -1, y: 0}, Point { x: 0, y: 1}, Point { x: 1, y: 1},
        Point { x: -1, y: 1}, Point { x: 0, y: 2},
    ],
    [
        Point { x: 0, y: -1 }, Point { x: 1, y: 0 },
        Point { x: -1, y: 0 }, Point { x: 0, y: 0 }, Point { x: 1, y: 1 },
        Point { x: -1, y: 1 }, Point { x: 0, y: 1 }, Point { x: 1, y: 2 },
        Point { x: -1, y: 2 }, Point { x: 0, y: 2 },
    ],
];

pub struct LightMap {
    data: Box<[u32]>,
}

impl LightMap {
    pub const TRI_WIDTH: i32 = 32;
    pub const TRI_HEIGHT: i32 = 13;
    pub const WIDTH: i32 = Self::TRI_WIDTH * 3 - Self::TRI_WIDTH / 2;
    pub const HEIGHT: i32 = Self::TRI_HEIGHT * 3 + 2 /* as in original */;

    pub fn new() -> Self {
        Self::with_data(vec![0; (Self::WIDTH * Self::HEIGHT) as usize].into_boxed_slice())
    }

    pub fn with_data(data: Box<[u32]>) -> Self {
        assert_eq!(data.len(), (Self::WIDTH * Self::HEIGHT) as usize);
        Self {
            data
        }
    }

    pub fn build(&mut self, lights: &[u32]) {
        type FillInfo = [i32; LightMap::TRI_HEIGHT as usize];

        static UP_TRI_FILL: FillInfo = [2, 2, 6, 8, 10, 14, 16, 18, 20, 24, 26, 28, 32];
        static DOWN_TRI_FILL: FillInfo = [32, 32, 30, 26, 24, 22, 18, 16, 14, 12, 8, 6, 4];

        static TRI_VERTEX_INDEXES: [[usize; 3]; VERTEX_COUNT] = [
            [0, 3, 2],
            [0, 1, 3],
            [1, 4, 3],
            [2, 3, 5],
            [3, 6, 5],
            [3, 4, 6],
            [4 ,7, 6],
            [5, 6, 8],
            [6, 9, 8],
            [6, 7, 9],
        ];

        assert_eq!(lights.len(), VERTEX_COUNT);

        fn fill_tri(light_map: &mut [u32], lights: &[u32],
                    tri_idx: usize, fill_info: &FillInfo, leftmost_vertex_idx: usize) {
            let tri_vert_idx = TRI_VERTEX_INDEXES[tri_idx];
            let x_inc = (lights[tri_vert_idx[1]] as i32 - lights[tri_vert_idx[leftmost_vertex_idx]] as i32) / LightMap::TRI_WIDTH;
            let y_inc = (lights[tri_vert_idx[2]] as i32 - lights[tri_vert_idx[0]] as i32) / LightMap::TRI_HEIGHT;
            let mut row_light = lights[tri_vert_idx[0]] as i32;
            for y in 0..LightMap::TRI_HEIGHT {
                let len = fill_info[y as usize];
                let half_len = len / 2;
                let mut light = row_light;
                for x in LightMap::TRI_WIDTH / 2 - half_len..LightMap::TRI_WIDTH / 2 + half_len {
                    // Computed light value can be out of [0..0x10000] bounds.
                    // Original engine does nothing to handle this.
                    let clipped_light = cmp::min(cmp::max(light, 0), 0x10000) as u32;
                    light_map[(y * LightMap::WIDTH + x) as usize] = clipped_light;
                    light += x_inc;
                }
                row_light += y_inc;
            }
        }

        // Up-facing tris are filled first, then go the down-facing tris.
        for tri_kind in 0..2 {
            let mut tri_idx = 0;
            let mut y = 0;
            let mut start_x = -Self::TRI_WIDTH / 2;
            while y <= Self::HEIGHT - Self::TRI_HEIGHT {
                let mut x = start_x;
                for _ in 0..2 {
                    let tri_x = x;
                    if tri_x >= 0 {
                        if tri_kind == 1 {
                            // fill down-facing tri
                            fill_tri(self.slice_mut(tri_x, y), lights, tri_idx, &DOWN_TRI_FILL, 0);
                        }
                        tri_idx += 1;
                    }

                    let tri_x = x + Self::TRI_WIDTH / 2;
                    if tri_x < Self::WIDTH - Self::TRI_WIDTH / 2 {
                        if tri_kind == 0 {
                            // fill up-facing tri
                            fill_tri(self.slice_mut(tri_x, y), lights, tri_idx, &UP_TRI_FILL, 2);
                        }
                        tri_idx += 1;
                    }
                    x += Self::TRI_WIDTH;
                }
                y += Self::TRI_HEIGHT - 1; // triangles overlap
                start_x += Self::TRI_WIDTH / 2;
            }
        }
    }

    pub fn data(&self) -> &[u32] {
        &self.data
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        self.data[(y * Self::WIDTH + x) as usize]
    }

    fn slice_mut(&mut self, x: i32, y: i32) -> &mut [u32] {
        &mut self.data[(y * Self::WIDTH + x) as usize..]
    }
}

#[cfg(test)]
mod test {
    use byteorder::{ByteOrder, LittleEndian};
    use super:: *;

    #[test]
    fn test() {
        let expected: Vec<_> = include_bytes!("light_map_expected.bin")
            .chunks(4)
            .map(LittleEndian::read_i32)
            .collect();

        let mut actual = LightMap::new();
        actual.build(&[0x10000, 0, 0, 0x10000, 0x10000, 0, 0x10000, 0, 0x10000, 0]);

        for y in 0..LightMap::HEIGHT {
            for x in 0..LightMap::WIDTH {
                let i = (y * LightMap::HEIGHT + x) as usize;
                let expected = expected[i];
                let expected = cmp::min(cmp::max(expected, 0), 0x10000) as u32;
                assert_eq!(actual.data[i], expected, "{} {}", x, y);
            }
        }
    }
}
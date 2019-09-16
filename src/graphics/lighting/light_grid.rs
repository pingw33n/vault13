use enum_map::EnumMap;
use num_traits::clamp;

use crate::graphics::geometry::hex::{self, Direction};
use crate::graphics::{EPoint, Point};
use crate::util::{EnumExt, VecExt};

const MAX_EMITTER_RADIUS: u32 = 8;
/// Number of points inside the light cone of MAX_EMITTER_RADIUS.
const LIGHT_CONE_LEN: usize = 36;
const DEFAULT_LIGHT_INTENSITY: i32 = 655;
const MAX_INTENSITY: u32 = 0x10000;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LightTest {
    /// Index of the point inside the light cone.
    pub i: usize,

    /// Light cone direction.
    pub direction: Direction,

    /// Hex coords of the point inside the light cone.
    pub point: EPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightTestResult {
    pub block: bool,
    pub update: bool,
}

impl Default for LightTestResult {
    fn default() -> Self {
        Self {
            block: false,
            update: true,
        }
    }
}

pub struct LightGrid {
    width: i32,
    light_cones: LightCones,
    grid: Box<[Box<[i32]>]>,
    block: LightBlock,
}

impl LightGrid {
    pub fn new(width: i32, height: i32, elevation_count: u32) -> Self {
        assert!(elevation_count > 0);
        let light_cones = LightCones::new(MAX_EMITTER_RADIUS);
        let len = (width * height) as usize;
        let grid = Vec::from_fn(elevation_count as usize,
            |_| vec![DEFAULT_LIGHT_INTENSITY; len].into_boxed_slice()).into_boxed_slice();

        Self {
            width,
            light_cones,
            grid,
            block: LightBlock::new(),
        }
    }

    pub fn clear(&mut self) {
        for g in self.grid.iter_mut() {
            for p in g.iter_mut() {
                *p = DEFAULT_LIGHT_INTENSITY;
            }
        }
    }

    pub fn update(&mut self, p: impl Into<EPoint>, radius: u32, delta: i32,
                  mut tester: impl FnMut(LightTest) -> LightTestResult) {
        assert!(radius <= MAX_EMITTER_RADIUS, "{}", radius);

        let p = p.into();
        assert!((p.elevation as usize) < self.grid.len());
        assert!(p.point.x >= 0 && p.point.x < self.width);
        assert!(p.point.y >= 0 && p.point.y < self.grid[0].len() as i32 / self.width);

        if delta == 0 {
            return;
        }

        Self::update_at(&mut self.grid, self.width, p.elevation, p.point, delta);

        let delta_sign = delta.signum();
        let delta_abs = delta.abs();
        let falloff = (delta_abs - 655) / (radius as i32 + 1);

        let light_cones = &self.light_cones.cones(p.point.x % 2 != 0);
        for i in 0..self.light_cones.len() {
            if radius < self.light_cones.radiuses()[i] {
                continue;
            }
            let amount = delta_sign *
                (delta_abs - falloff * self.light_cones.radiuses()[i] as i32);
            for dir in Direction::iter() {
                let light_cone_point = p.point + light_cones[dir][i];
                let blocked = self.block.get(i, dir);
                let blocked = if !blocked  {
                    let LightTestResult { block, update } = tester(LightTest {
                        i,
                        direction: dir,
                        point: EPoint { elevation: p.elevation, point: light_cone_point },
                    });
                    if update {
                        Self::update_at(&mut self.grid, self.width, p.elevation,
                            light_cone_point, amount);
                    }
                    block
                } else {
                    blocked
                };
                self.block.set(i, dir, blocked);
            }
        }
    }

    pub fn grid(&self) -> &[Box<[i32]>] {
        &self.grid
    }

    pub fn get(&self, p: impl Into<EPoint>) -> i32 {
        let p = p.into();
        self.grid[p.elevation as usize][(self.width * p.point.y + p.point.x) as usize]
    }

    pub fn get_clipped(&self, p: impl Into<EPoint>) -> u32 {
        clamp(self.get(p), 0, 0x10000) as u32
    }

    fn update_at(grid: &mut Box<[Box<[i32]>]>, width: i32, elevation: u32, p: Point, delta: i32) {
        let i = (width * p.y + p.x) as usize;
        grid[elevation as usize][i] += delta;
    }
}

#[derive(Debug)]
struct LightCones {
    cones: [EnumMap<Direction, Box<[Point]>>; 2],
    radiuses: Box<[u32]>,
}

impl LightCones {
    pub fn new(radius: u32) -> Self {
        let mut radiuses = Vec::with_capacity(radius as usize);
        let cones = [
            EnumMap::from(|dir| Self::make(false, dir, radius,
                |r| if dir == Direction::NE { radiuses.push(r) })),
            EnumMap::from(|dir| Self::make(true, dir, radius, |_| {})),
        ];
        Self {
            cones,
            radiuses: radiuses.into_boxed_slice(),
        }
    }

    pub fn len(&self) -> usize {
        self.radiuses.len()
    }

    pub fn cones(&self, odd: bool) -> &EnumMap<Direction, Box<[Point]>> {
        &self.cones[odd as usize]
    }

    pub fn radiuses(&self) -> &[u32] {
        &self.radiuses
    }

    fn make(odd: bool, direction: Direction, radius: u32, mut radius_out: impl FnMut(u32))
        -> Box<[Point]>
    {
        let mut r = Vec::new();
        let origin = Point::new(odd as i32, 0);
        let next_dir = direction.rotate_cw();
        for start_dist in 0..radius {
            let start = hex::go(origin, next_dir, start_dist);
            for dist in 1..=(radius - start_dist) {
                let point = hex::go(start, direction, dist);
                r.push(point - origin);
                radius_out(start_dist + dist);
            }
        }
        r.into_boxed_slice()
    }
}

struct LightBlock {
    cones: EnumMap<Direction, [bool; LIGHT_CONE_LEN]>,
}

impl LightBlock {
    pub fn new() -> Self {
        Self {
            cones: EnumMap::from(|_| [false; LIGHT_CONE_LEN]),
        }
    }

    pub fn get(&mut self, i: usize, direction: Direction) -> bool {
        let c = &self.cones[direction];
        let nc = &self.cones[direction.rotate_cw()];
        match i {
            0  => false,
            1  => c[0],
            2  => c[1],
            3  => c[2],
            4  => c[3],
            5  => c[4],
            6  => c[5],
            7  => c[6],
            8  => nc[0] && c[0],
            9  => c[1] && c[8],
            10 => c[2] && c[9],
            11 => c[3] && c[10],
            12 => c[4] && c[11],
            13 => c[5] && c[12],
            14 => c[6] && c[13],
            15 => nc[1] && c[8],
            16 => c[15] && c[9] || c[8],
            17 => {
                let v9 = c[10] || c[9];
                let v10 = c[16] & v9 | v9 & c[8];
                let v11 = (c[15] || c[10]) && c[9];
                v11 | v10
            }
            18 => (c[11] || c[10] || c[9] || c[0]) && c[17]
                || c[9]
                || c[16] && c[10],
            19 => c[18] && c[12]
                 || c[10]
                 || c[9]
                 || (c[18] || c[17]) && c[11],
            20 => {
                let v13 = c[12] || c[11] || c[2];
                let v10 = c[10] | c[9] & v13 | v13 & c[8];
                let v11 = (c[19] || c[18] || c[17] || c[16]) && c[11];
                v11 | v10
            }
            21 => nc[2] && c[15] || nc[1] && c[8],

            22 => c[16] && (c[21] || c[15])
                || c[15] && (c[21] || c[9])
                || (c[21]
                || c[15]
                || nc[1]) && c[8],
            23 => c[22] && c[17]
                || c[15] && c[9]
                || c[3]
                || c[16],
            24 => c[23] && c[18]
                || c[17]
                && (c[23] || c[22] || c[15])
                || c[8]
                || c[9]
                && (c[23]
                || c[16]
                || c[15])
                || (c[18]
                || c[17]
                || c[10]
                || c[9]
                || c[0])
                && c[16],
            25 => {
                let v15 = c[16] || c[8];
                let v10 = c[18] & (c[24] | c[23] | v15) |
                    c[17] |
                    c[10] & (c[24] | v15 | c[17]) |
                    (c[1] && c[8] || (c[24] || c[23] || c[16] || c[15] || c[8]) && c[9]);
                let v11 = (c[19] || c[0]) && c[24];
                v11 | v10
            }
            26 => {
                let v10 = c[8] && nc[1]
                    || nc[2] && c[15];
                let v11 = nc[3] && c[21];
                v11 | v10
            }
            27 => (c[16] || c[8])
                && c[21]
                || c[15]
                || nc[1] && c[8]
                || (c[26]
                || c[21]
                || c[15]
                || nc[0])
                && c[22],
            28 => c[27] && c[23]
                || c[22]
                && (c[23]
                || c[17]
                || c[9])
                || c[16]
                && (c[27]
                || c[22]
                || c[21]
                || nc[0])
                || c[8]
                || c[15]
                && (c[23]
                || c[16]
                || c[9]),
            29 => c[28] && c[24]
                || c[22] && c[17]
                || c[15] && c[9]
                || c[16]
                || c[8]
                || c[23],
            30 => {
                let v10 = nc[2] && c[15]
                    || c[8] && nc[1]
                    || nc[3] && c[21];
                let v11 = nc[4] && c[26];
                v11 | v10
            }
            31 => c[30] && c[27]
                || c[26]
                && (c[27]
                || c[22]
                || c[8])
                || c[15]
                || nc[1] && c[8]
                || c[21],
            32 => {
                let v18 = nc[1] && c[8]
                    || (c[28]
                    || c[23]
                    || c[16]
                    || c[9]
                    || c[8])
                    && c[15];
                let v19 = c[16] || c[8];
                (c[28] && (c[31] || c[0]))
                    | c[27] & (c[28] | c[23] | v19)
                    | c[22]
                    | v18
                    | c[21] & (v19 | c[28])
            }
            33 => {
                let v10 = nc[3] && c[21]
                    || nc[2] && c[15]
                    || nc[1] && c[8]
                    || nc[4] && c[26];
                let v11 = nc[5] && c[30];
                v11 | v10
            }
            34 => {
                let v21 = c[30]
                    || c[26]
                    || nc[2];
                let v10 = c[21] | c[15] & v21 | v21 & c[8];
                let v11 = (c[31]
                    || c[27]
                    || c[22]
                    || c[16])
                    && c[26];
                v11 | v10
            }
            35 => {
                let v10 = nc[4] && c[26]
                    || nc[3] && c[21]
                    || nc[2] && c[15]
                    || c[8] && nc[1]
                    || nc[5] && c[30];
                let v11 = nc[6] && c[33];
                v11 | v10
            }
            _ => panic!("invalid index: {}", i),
        }
    }

    pub fn set(&mut self, i: usize, direction: Direction, blocked: bool) {
        self.cones[direction][i] = blocked;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use enum_map::enum_map;

    #[test]
    fn light_cones() {
        let light_cones = LightCones::new(MAX_EMITTER_RADIUS);

        assert_eq!(light_cones.len(), LIGHT_CONE_LEN);
        assert_eq!(light_cones.radiuses(), &[
            1, 2, 3, 4, 5, 6, 7, 8,
            2, 3, 4, 5, 6, 7, 8,
            3, 4, 5, 6, 7, 8,
            4, 5, 6, 7, 8,
            5, 6, 7, 8,
            6, 7, 8,
            7, 8,
            8][..]);

        #[derive(Debug)]
        struct P {
            i: usize,
            p: (i32, i32),
        }

        let expected = vec![
            enum_map! {
                Direction::NE => vec![
                    P { i: 0,   p: (1, -1) },
                    P { i: 7,   p: (8, -4) },
                    P { i: 17,  p: (5, -1) },
                    P { i: 35,  p: (8, 3) },
                ],
                _ => vec![],
            },
            enum_map! {
                Direction::NE => vec![
                    P { i: 0,   p: (1, 0) },
                    P { i: 7,   p: (8, -4) },
                    P { i: 17,  p: (5, 0) },
                    P { i: 35,  p: (8, 3) },
                ],
                _ => vec![],
            },
        ];

        for odd in 0..=1 {
            for dir in Direction::iter() {
                for e in &expected[odd][dir] {
                    assert_eq!(light_cones.cones(odd == 1)[dir][e.i], e.p.into(),
                        "odd={} {:?} {:?}", odd, dir, e);
                }
            }
        }
    }

    mod light_grid {
        use byteorder::{ByteOrder, LittleEndian};
        use std::collections::HashMap;

        use super::*;
        use crate::graphics::geometry::hex::TileGrid;
        use crate::util::test::ungz;

        #[test]
        fn reference_no_light_test() {
            let expected = read_light_grid_dump(&include_bytes!("light_grid_expected.bin.gz")[..]);

            let mut actual = LightGrid::new(200, 200, 1);
            for (linp, radius, amount) in vec![
                (0x3898, 8, 0x10000),
                (0x3d49, 8, 0x10000),
                (0x41fa, 8, 0x10000),
                (0x4453, 5, 0x10000),
                (0x4e84, 4, 0x10000),
                (0x54f4, 8, 0x10000),
                (0x5b33, 8, 0x10000),
                (0x5c02, 8, 0x10000),
                (0x5d93, 8, 0x10000),
                (0x60b1, 8, 0x10000),
                (0x617b, 8, 0x10000),
                (0x6499, 8, 0x10000),
                (0x64a1, 8, 0x10000),
            ] {
                let point = TileGrid::default().from_linear_inv(linp);
                actual.update(EPoint { elevation: 0, point }, radius, amount,
                    |_| LightTestResult::default());
            }

            assert_eq!(&actual.grid()[0][..], &expected[0][..]);
        }

        #[test]
        fn reference_with_light_test() {
            let expected = read_light_grid_dump(&include_bytes!("light_grid_expected2.bin.gz")[..]);

            // This data was generated from arcaves.map.
            let input = include!("light_grid_input.in");
            let light_test = include!("light_grid_light_test.in");

            const ELEVATION: u32 = 1;
            let mut light_test_map = HashMap::new();
            for ((i, direction, (x, y)), (block, update)) in light_test {
                let lt = LightTest {
                    i,
                    direction,
                    point: EPoint {
                        elevation: ELEVATION,
                        point: (x, y).into(),
                    },
                };
                let r = LightTestResult {
                    block,
                    update,
                };
                light_test_map.insert(lt, r);
            }

            let mut actual = LightGrid::new(200, 200, ELEVATION + 1);
            for (point, radius, intensity) in input {
                actual.update(Point::from(point).elevated(ELEVATION),
                    radius, intensity as i32, |lt| light_test_map[&lt]);
            }

            assert_eq!(&actual.grid()[ELEVATION as usize][..], &expected[0][..]);
        }

        #[test]
        fn flip() {
            let mut lg = LightGrid::new(200, 200, 2);

            let expected = Vec::from(lg.grid().clone());

            lg.update(EPoint { elevation: 0, point: Point::new(31, 41) }, 8, 1234567,
                |_| LightTestResult::default());
            lg.update(EPoint { elevation: 0, point: Point::new(31, 41) }, 8, -1234567,
                |_| LightTestResult::default());

            assert_eq!(lg.grid(), &expected[..]);
        }

        fn read_light_grid_dump(bytes: &[u8]) -> Box<[Box<[i32]>]> {
            let mut expected: Vec<_> = ungz(bytes).chunks(4).map(LittleEndian::read_i32).collect();
            for c in expected.chunks_mut(200) {
                c.reverse();
            }
            let expected: Vec<_> = expected.chunks(200 * 200)
                .map(|v| Vec::from(v).into_boxed_slice()).collect();
            expected.into_boxed_slice()
        }
    }
}
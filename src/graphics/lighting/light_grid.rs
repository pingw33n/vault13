use enum_map::EnumMap;

use graphics::geometry::Direction;
use graphics::geometry::hex::TileGrid;
use graphics::{ElevatedPoint, Point};
use util::{EnumExt, vec_with_func};

const MAX_EMITTER_RADIUS: u32 = 8;
/// Number of points inside the light cone of MAX_EMITTER_RADIUS.
const LIGHT_CONE_LEN: usize = 36;
const DEFAULT_LIGHT_INTENSITY: i32 = 655;
const MAX_INTENSITY: u32 = 0x10000;

#[derive(Clone, Copy, Debug)]
pub struct LightTestResult {
    blocked: bool,
    updatable: bool,
}

impl Default for LightTestResult {
    fn default() -> Self {
        Self {
            blocked: false,
            updatable: true,
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
    pub fn new(tile_grid: &TileGrid, elevations: usize) -> Self {
        assert!(elevations > 0);
        let light_cones = LightCones::new(MAX_EMITTER_RADIUS, tile_grid);
        let grid = vec_with_func(elevations,
            |_| vec![DEFAULT_LIGHT_INTENSITY; tile_grid.len()].into_boxed_slice()).into_boxed_slice();

        Self {
            width: tile_grid.width(),
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

    pub fn update(&mut self, p: impl Into<ElevatedPoint>, radius: u32, delta: i32,
                  tester: impl Fn(usize, Point) -> LightTestResult) {
        assert!(radius <= MAX_EMITTER_RADIUS);

        let p = p.into();
        assert!(p.elevation < self.grid.len());
        assert!(p.point.x >= 0 && p.point.x < self.width);
        assert!(p.point.y >= 0 && p.point.y < self.grid[0].len() as i32 / self.width);

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
                    let LightTestResult { blocked, updatable } = tester(i, light_cone_point);
                    if updatable {
                        Self::update_at(&mut self.grid, self.width, p.elevation,
                            light_cone_point, amount);
                    }
                    blocked
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

    fn update_at(grid: &mut Box<[Box<[i32]>]>, width: i32, elevation: usize, p: Point, delta: i32) {
        let i = (width * p.y + p.x) as usize;
        grid[elevation][i] += delta;
    }
}

#[derive(Debug)]
struct LightCones {
    cones: [EnumMap<Direction, Box<[Point]>>; 2],
    radiuses: Box<[u32]>,
}

impl LightCones {
    pub fn new(radius: u32, tile_grid: &TileGrid) -> Self {
        let mut radiuses = Vec::with_capacity(radius as usize);
        let cones = [
            EnumMap::from(|dir| Self::make(false, dir, radius, tile_grid,
                |r| if dir == Direction::NE { radiuses.push(r) })),
            EnumMap::from(|dir| Self::make(true, dir, radius, tile_grid, |_| {})),
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

    fn make(odd: bool, direction: Direction, radius: u32, tile_grid: &TileGrid,
            mut radius_out: impl FnMut(u32)) -> Box<[Point]> {
        let mut r = Vec::new();
        let origin = Point::new(odd as i32, 0);
        let next_dir = direction.rotate_cw();
        for start_dist in 0..radius {
            let start = tile_grid.go(origin, next_dir, start_dist, false);
            for dist in 1..=(radius - start_dist) {
                let point = tile_grid.go(start, direction, dist, false);
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

    #[test]
    fn light_cones() {
        let light_cones = LightCones::new(MAX_EMITTER_RADIUS, &TileGrid::default());

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
        use flate2::bufread::GzDecoder;
        use std::io::Read;
        use super::*;

        #[test]
        fn reference() {
            let mut expected = Vec::new();
            GzDecoder::new(&include_bytes!("light_grid_expected.bin.gz")[..])
                .read_to_end(&mut expected).unwrap();
            let mut expected: Vec<_> = expected.chunks(4).map(LittleEndian::read_i32).collect();
            for c in expected.chunks_mut(200) {
                c.reverse();
            }
            let expected: Vec<_> = expected.chunks(200 * 200).map(|v| Vec::from(v)).collect();

            let mut actual = LightGrid::new(&TileGrid::default(), 1);
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
                let point = TileGrid::default().from_linear(linp);
                actual.update(ElevatedPoint { elevation: 0, point }, radius, amount,
                    |_, _| LightTestResult::default());
            }

            assert_eq!(&actual.grid()[0][..], &expected[0][..]);
        }

        #[test]
        fn flip() {
            let mut lg = LightGrid::new(&TileGrid::default(), 2);

            let expected = Vec::from(lg.grid().clone());

            lg.update(ElevatedPoint { elevation: 0, point: Point::new(31, 41) }, 8, 1234567,
                |_, _| LightTestResult::default());
            lg.update(ElevatedPoint { elevation: 0, point: Point::new(31, 41) }, 8, -1234567,
                |_, _| LightTestResult::default());

            assert_eq!(lg.grid(), &expected[..]);
        }
    }
}
use super::{hex, sqr};
use crate::graphics::Point;
use crate::graphics::geometry::TileGridView;

pub const ELEVATION_COUNT: u32 = 3;

pub struct MapGrid {
    hex: hex::TileGrid,
    sqr: sqr::TileGrid,
    screen_width: i32,
    screen_height: i32,
}

impl MapGrid {
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        assert!(screen_width > 0);
        assert!(screen_height > 0);
        let mut r = Self {
            hex: hex::TileGrid::default(),
            sqr: sqr::TileGrid::default(),
            screen_width,
            screen_height,
        };
        r.sync_sqr();
        r
    }

    pub fn hex(&self) -> &hex::TileGrid {
        &self.hex
    }

    pub fn sqr(&self) -> &sqr::TileGrid {
        &self.sqr
    }

    pub fn screen_width(&self) -> i32 {
        self.screen_width
    }

    pub fn screen_height(&self) -> i32 {
        self.screen_height
    }

    pub fn center_hex(&self) -> Point {
        self.hex.from_screen((self.screen_width / 2, self.screen_height / 2))
    }

    pub fn center2(&mut self, hex_pos: impl Into<Point>) {
        self.hex.set_screen_pos((0, 0));
        let hex_screen_pos = -self.hex.to_screen(hex_pos.into()) +
            Point::new(self.screen_width / 2 - 16, self.screen_height / 2 - 8);
        self.hex.set_screen_pos(hex_screen_pos);
        self.sync_sqr();
    }

    pub fn scroll(&mut self, screen_offset: impl Into<Point>) {
        *self.hex.screen_pos_mut() -= screen_offset.into();
        self.sync_sqr();
    }

    fn sync_sqr(&mut self) {
        self.sqr.set_screen_pos(self.hex.screen_pos() - Point::new(16, 2));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn center2() {
        let mut m = MapGrid::new(640, 380);
        const W: i32 = 640;
        const H: i32 = 380;
        let expected_hex = Point::new(m.screen_width() / 2 - 16, m.screen_height() / 2 - 8);
        let expected_sqr = [
            [expected_hex - Point::new(16, 2), expected_hex - Point::new(48, 2)],
            [expected_hex - Point::new(32, 12 + 2), expected_hex - Point::new(64, 12 + 2)]];

        for (x, y) in vec![
                    (0, 0),
                    (1, 0),
                    (0, 1),
                    (123, 123),
                    (124, 124),
                ] {
            m.center2((x, y));
            assert_eq!(m.hex().to_screen((x, y)), expected_hex);
            let expected_sqr =  expected_sqr[y as usize % 2][x as usize % 2];
            assert_eq!(m.sqr().to_screen((x / 2, y / 2)), expected_sqr);
        }
    }
}
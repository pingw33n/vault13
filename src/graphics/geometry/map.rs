use graphics::Point;
use super::{hex, sqr};

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
        Self {
            hex: hex::TileGrid::default(),
            sqr: sqr::TileGrid::default(),
            screen_width,
            screen_height,
        }
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

    // tile_set_center()
    pub fn center(&mut self, hex_pos: impl Into<Point>) {
        let mut hex_pos = hex_pos.into();
        let mut hex_screen_pos = Point::new(self.screen_width / 2 - 16, self.screen_height / 2 - 8);
        if hex_pos.x % 2 != 0 {
            hex_pos.x -= 1;
            hex_screen_pos.x -= 32;
        }
        self.hex.set_pos(hex_pos);
        self.hex.set_screen_pos(hex_screen_pos);

        let sqr_pos = Point::new(hex_pos.x / 2, hex_pos.y / 2);
        let mut sqr_screen_pos = Point::new(hex_screen_pos.x - 16, hex_screen_pos.y - 2);
        if hex_pos.y % 2 != 0 {
            sqr_screen_pos.x -= 16;
            sqr_screen_pos.y -= 12;
        }
        self.sqr.set_pos(sqr_pos);
        self.sqr.set_screen_pos(sqr_screen_pos);

        trace!("MapGrid::centered: hex::pos={:?}, {:?} hex::screen_pos={:?} sqr::pos={:?}, {:?} sqr::screen_pos={:?}",
            hex_pos.tuple(), self.hex.to_linear_inv(hex_pos), hex_screen_pos.tuple(),
            sqr_pos.tuple(), self.sqr.to_linear_inv(sqr_pos), sqr_screen_pos.tuple());
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
    fn center() {
        let mut m = MapGrid::new(640, 380);

        let pos = m.hex().from_linear_inv(0x4e85);
        m.center(pos);
        assert_eq!(m.hex().pos(), Point::new(0x62, 0x64));
        assert_eq!(m.hex().screen_pos(), Point::new(0x130, 0xb6));
        assert_eq!(m.sqr().pos(), Point::new(0x31, 0x32));
        assert_eq!(m.sqr().screen_pos(), Point::new(0x120, 0xb4));

        let pos = m.hex().from_linear_inv(0x4e86);
        m.center(pos);
        assert_eq!(m.hex().pos(), Point::new(0x60, 0x64));
        assert_eq!(m.hex().screen_pos(), Point::new(0x110, 0xb6));
        assert_eq!(m.sqr().pos(), Point::new(0x30, 0x32));
        assert_eq!(m.sqr().screen_pos(), Point::new(0x100, 0xb4));

        let pos = m.hex().from_linear_inv(0x623a);
        m.center(pos);
        assert_eq!(m.hex().pos(), Point::new(0x34, 0x7d));
        assert_eq!(m.hex().screen_pos(), Point::new(0x110, 0xb6));
        assert_eq!(m.sqr().pos(), Point::new(0x1a, 0x3e));
        assert_eq!(m.sqr().screen_pos(), Point::new(0xf0, 0xa8));
    }

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
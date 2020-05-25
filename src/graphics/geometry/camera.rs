use crate::graphics::{Point, Rect};
use super::hex;
use super::sqr;

/// Defines part of tile grid to be drawn to screen viewport rect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Camera {
    /// Position of the top left tile (0, 0) on screen.
    pub origin: Point,

    /// Screen rect where the tile grid is visible.
    pub viewport: Rect,
}

impl Camera {
    pub fn hex(&self) -> hex::View {
        hex::View::new(self.origin)
    }

    pub fn sqr(&self) -> sqr::View {
        sqr::View::new(self.origin - Point::new(16, 2))
    }

    /// Adjusts the `origin` so the center of tile at `hex_pos` is positioned in the center of viewport.
    pub fn look_at(&mut self, hex_pos: Point) {
        self.align(hex_pos, self.viewport.center())
    }

    /// Adjusts the `origin` so the center of tile at `hex_pos` is positioned at the `screen_pos`.
    pub fn align(&mut self, hex_pos: Point, screen_pos: Point) {
        self.origin = screen_pos - hex::center_to_screen(hex_pos);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::TileGridView;

    #[test]
    fn look_at() {
        let mut c = Camera {
            origin: Point::new(0, 0),
            viewport: Rect::with_size(0, 0, 640, 380),
        };
        let expected_hex = Point::new(c.viewport.width() / 2 - 16, c.viewport.height() / 2 - 8);
        let expected_sqr = [
            [expected_hex - Point::new(16, 2), expected_hex - Point::new(48, 2)],
            [expected_hex - Point::new(32, 12 + 2), expected_hex - Point::new(64, 12 + 2)]];

        for &(x, y) in &[
                    (0, 0),
                    (1, 0),
                    (0, 1),
                    (123, 123),
                    (124, 124),
                ] {
            let p = Point::new(x, y);
            c.look_at(p);
            assert_eq!(c.hex().to_screen(p), expected_hex);
            let expected_sqr =  expected_sqr[y as usize % 2][x as usize % 2];
            assert_eq!(c.sqr().to_screen(p / 2), expected_sqr);
        }
    }
}
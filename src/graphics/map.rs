use graphics::geometry::{hex, sqr};
use graphics::{Point, Rect};
use graphics::render::Render;
use graphics::render::TextureHandle;

const ROOF_HEIGHT: i32 = 96;

pub fn render_floor<'a>(render: &mut Render, stg: &sqr::TileGrid, rect: &Rect,
        num_to_tex: impl FnMut(i32) -> Option<TextureHandle>) {
    render_square_tiles(render, stg, rect, 0, num_to_tex);
}

pub fn render_roof<'a>(render: &mut Render, stg: &sqr::TileGrid, rect: &Rect,
        num_to_tex: impl FnMut(i32) -> Option<TextureHandle>) {
    render_square_tiles(render, stg, rect, ROOF_HEIGHT, num_to_tex);
}

fn render_square_tiles<'a>(render: &mut Render, stg: &sqr::TileGrid, rect: &Rect,
        y_offset: i32,
        mut num_to_tex: impl FnMut(i32) -> Option<TextureHandle>) {
    let screen_y = rect.top + y_offset;
    let y = stg.from_screen((rect.left, screen_y)).y;
    let x = stg.from_screen((rect.right - 1, screen_y)).x;
    let start = stg.clip((x, y));

    let screen_y = rect.bottom - 1 + y_offset;
    let x = stg.from_screen((rect.left, screen_y)).x;
    let y = stg.from_screen((rect.right - 1, screen_y)).y;
    let end = stg.clip((x, y));

    // TODO apply per-hex lighting.

    for y in start.y..=end.y {
        for x in (end.x..=start.x).rev() {
            let num = stg.to_linear((x, y)).unwrap();
            if let Some(tex) = num_to_tex(num) {
                let scr_pt = stg.to_screen((x, y));
                render.draw(&tex, scr_pt.x, scr_pt.y - y_offset, 0x10000);
            }
        }
    }
}

// Whether scroll is restricted based on horz/vert distance from `dude_pos` to the new `pos`.
pub fn is_scroll_limited(htg: &hex::TileGrid,
                         pos: impl Into<Point>, dude_pos: impl Into<Point>) -> bool {
    let dist = htg.to_screen(dude_pos) - htg.to_screen(pos);
    dist.x >= 480 || dist.y >= 400

    // There's also:
//         v8 = abs(dude_tile_screen_y - g_map_win_center_y),
//         v4 > abs(dude_tile_screen_x - g_map_win_center_x))
//     || v6 > v8)
}

//TODO
//
//    || (unsigned __int8)g_tile_scroll_blocking_enabled & ((flags & TSCF_IGNORE_SCROLL_RESTRICTIONS) == 0)
//    && !obj_scroll_blocking_at_(tile_num, elevation)
//
//    || (tile_x = g_map_width_tiles - 1 - tile_num % g_map_width_tiles,
//        tile_y = tile_num / g_map_width_tiles,
//        g_map_border_set)
//    && (tile_x <= g_map_border_tile_x_min
//     || tile_x >= g_map_border_tile_x_max
//     || tile_y <= g_map_border_tile_y_min
//     || tile_y >= g_map_border_tile_y_max)
//}

// tile_set_center()
pub fn center(hex: &mut hex::TileGrid, sqr: &mut sqr::TileGrid, hex_pos: impl Into<Point>,
        screen_width: i32, screen_height: i32) {
    let mut hex_pos = hex_pos.into();

    let mut hex_screen_pos = Point::new((screen_width - 32) / 2, (screen_height - 16) / 2);
    if hex_pos.x % 2 != 0 {
        hex_pos.x -= 1;
        hex_screen_pos.x -= 32;
    }
    hex.set_pos(hex_pos);
    hex.set_screen_pos(hex_screen_pos);

    let sqr_pos = Point::new(hex_pos.x / 2, hex_pos.y / 2);
    let mut sqr_screen_pos = Point::new(hex_screen_pos.x - 16, hex_screen_pos.y - 2);
    if hex_pos.y % 2 != 0 {
        sqr_screen_pos.x -= 16;
        sqr_screen_pos.y -= 12;
    }
    sqr.set_pos(sqr_pos);
    sqr.set_screen_pos(sqr_screen_pos);

    trace!("centered: hex::pos={:?}, {:?} hex::screen_pos={:?} sqr::pos={:?}, {:?} sqr::screen_pos={:?}",
        hex_pos.tuple(), hex.to_linear(hex_pos), hex_screen_pos.tuple(),
        sqr_pos.tuple(), sqr.to_linear(sqr_pos), sqr_screen_pos.tuple());
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn center_() {
        let ref mut h = hex::TileGrid::default();
        let ref mut s = sqr::TileGrid::default();

        let pos = h.from_linear(0x4e85);
        center(h, s, pos, 640, 380);
        assert_eq!(h.pos(), Point::new(0x62, 0x64));
        assert_eq!(h.screen_pos(), Point::new(0x130, 0xb6));
        assert_eq!(s.pos(), Point::new(0x31, 0x32));
        assert_eq!(s.screen_pos(), Point::new(0x120, 0xb4));

        let pos = h.from_linear(0x4e86);
        center(h, s, pos, 640, 380);
        assert_eq!(h.pos(), Point::new(0x60, 0x64));
        assert_eq!(h.screen_pos(), Point::new(0x110, 0xb6));
        assert_eq!(s.pos(), Point::new(0x30, 0x32));
        assert_eq!(s.screen_pos(), Point::new(0x100, 0xb4));

        let pos = h.from_linear(0x623a);
        center(h, s, pos, 640, 380);
        assert_eq!(h.pos(), Point::new(0x34, 0x7d));
        assert_eq!(h.screen_pos(), Point::new(0x110, 0xb6));
        assert_eq!(s.pos(), Point::new(0x1a, 0x3e));
        assert_eq!(s.screen_pos(), Point::new(0xf0, 0xa8));
    }
}
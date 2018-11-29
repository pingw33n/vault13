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
    let sqr_rect = stg.from_screen_rect(rect, true);

    // TODO apply per-hex lighting.

    for y in sqr_rect.top..sqr_rect.bottom {
        for x in (sqr_rect.left..=sqr_rect.right).rev() {
            let num = stg.to_linear_inv((x, y)).unwrap();
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





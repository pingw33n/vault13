use crate::graphics::geometry::{hex, sqr};
use crate::graphics::lighting::light_map::{VERTEX_COUNT, VERTEX_HEXES};
use crate::graphics::{Point, Rect};
use crate::graphics::render::{Canvas, TextureHandle};

const ROOF_HEIGHT: i32 = 96;

pub fn render_floor<'a>(canvas: &mut Canvas, stg: &sqr::TileGrid, rect: &Rect,
        get_tex: impl FnMut(Point) -> Option<TextureHandle>,
        get_light: impl Fn(Point) -> u32) {
    render_square_tiles(canvas, stg, rect, 0, get_tex, get_light);
}

pub fn render_roof<'a>(canvas: &mut Canvas, stg: &sqr::TileGrid, rect: &Rect,
        get_tex: impl FnMut(Point) -> Option<TextureHandle>) {
    let rect = Rect::with_size(rect.left, rect.top + ROOF_HEIGHT, rect.width(), rect.height());
    render_square_tiles(canvas, stg, &rect, ROOF_HEIGHT, get_tex, |_| 0x10000);
}

fn render_square_tiles(canvas: &mut Canvas, stg: &sqr::TileGrid, rect: &Rect,
        y_offset: i32,
        mut get_tex: impl FnMut(Point) -> Option<TextureHandle>,
        get_light: impl Fn(Point) -> u32) {
    let sqr_rect = stg.from_screen_rect(rect, true);

    let mut vertex_lights = [0; VERTEX_COUNT];
    for y in sqr_rect.top..sqr_rect.bottom {
        for x in (sqr_rect.left..sqr_rect.right).rev() {
            if let Some(tex) = get_tex(Point::new(x, y)) {
                let scr_pt = stg.to_screen((x, y)) - Point::new(0, y_offset);

                let hex_pos = Point::new(x * 2, y * 2);
                for i in 0..VERTEX_COUNT {
                    let l = get_light(hex_pos + VERTEX_HEXES[i]);
                    vertex_lights[i] = l;
                }

                canvas.draw_multi_light(&tex, scr_pt.x, scr_pt.y, &vertex_lights[..]);
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





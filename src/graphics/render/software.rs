use sdl2::render::{Canvas, Texture as SdlTexture};
use sdl2::video::Window;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::rc::Rc;
use std::cell::RefCell;

use super::*;
use graphics::color::{Color8, Palette, PaletteOverlay};
use graphics::lightmap::LightMap;
use graphics::Rect;
use sdl2::pixels::PixelFormatEnum;
use std::cmp;

struct Texture {
    width: i32,
    height: i32,
    data: Box<[u8]>,
}

impl Texture {
    pub fn new(width: i32, height: i32, data: Box<[u8]>) -> Self {
        assert_eq!(data.len(), (width * height) as usize);
        Self {
            data,
            width,
            height,
        }
    }

    pub fn new_empty(width: i32, height: i32, fill_color: u8) -> Self {
        assert!(width > 0 && height > 0);
        let len = (width * height) as usize;
        let data = vec![fill_color; len].into_boxed_slice();

        Self::new(width, height, data)
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}

pub struct SoftwareRender {
    textures: SlotMap<DefaultKey, ()>,
    textures_sm: SecondaryMap<DefaultKey, Texture>,
    texture_drop_list: Rc<RefCell<Vec<DefaultKey>>>,
    light_map: LightMap,
    back_buf: Texture,
    palette: Box<Palette>,
    palette_overlay: PaletteOverlay,
    canvas: Canvas<Window>,
    canvas_texture: SdlTexture,
    clip_rect: Rect,
}

impl SoftwareRender {
    pub fn new(canvas: Canvas<Window>, palette: Box<Palette>, palette_overlay: PaletteOverlay) -> Self {
        let (w, h) = canvas.window().size();
        println!("{} {} ", w, h);
        let canvas_texture = canvas
            .texture_creator()
            .create_texture_streaming(PixelFormatEnum::RGB24, w, h)
            .unwrap();
        Self {
            textures: SlotMap::new(),
            textures_sm: SecondaryMap::new(),
            texture_drop_list: Rc::new(RefCell::new(Vec::new())),
            light_map: LightMap::new(),
            back_buf: Texture::new_empty(w as i32, h as i32, 0),
            palette,
            palette_overlay,
            canvas,
            canvas_texture,
            clip_rect: Rect::with_size(0, 0, w as i32, h as i32),
        }
    }

    fn do_draw_translucent(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32,
            grayscale_func: impl Fn(Rgb15) -> u8) {
        let pal = &self.palette;
        let tex = &self.textures_sm[tex.0.key];
        let light = (light >> 9) as u8;

        let color = pal.color_idx(color);

        Self::do_draw(&mut self.back_buf, x, y, tex, &self.clip_rect,
            |dst, _, _, _, _, src| {
                let alpha = grayscale_func(pal.rgb15(src)) / 4;
                let color = pal.alpha_blend(color, *dst, alpha);
                *dst = pal.darken(color, light);
            }
        );
    }

    fn do_draw(
            dst: &mut Texture,
            dst_x: i32, dst_y: i32,
            src: &Texture,
            clip_rect: &Rect,
            f: impl Fn(&mut u8, i32, i32, i32, i32, u8)) {
        let rect = Rect::with_size(dst_x, dst_y, src.width, src.height)
            .intersect(&Rect::with_size(0, 0, dst.width, dst.height))
            .intersect(&clip_rect);
        let src_rect = rect.translate(-dst_x, -dst_y);
        let dst_x = rect.left;
        let mut dst_y = rect.top;

        for src_y in src_rect.top..src_rect.bottom {
            let src = &src.data[(src_y * src.width) as usize..];
            let dst = &mut dst.data[(dst_y * dst.width) as usize..];
            let mut dst_x_i = dst_x;
            for src_x in src_rect.left..src_rect.right {
                let src_color_idx = src[src_x as usize];
                if src_color_idx != 0 {
                    let dst_color_idx = &mut dst[dst_x_i as usize];
                    f(dst_color_idx, dst_x_i, dst_y, src_x, src_y, src_color_idx);
                }
                dst_x_i += 1;
            }
            dst_y += 1;
        }
    }
}

impl Render for SoftwareRender {
    fn new_texture(&mut self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        let key = self.textures.insert(());
        self.textures_sm.insert(key, Texture::new(width, height, data));
        TextureHandle(Rc::new(TextureHandleInner {
            key,
            drop_list: self.texture_drop_list.clone(),
        }))
    }

    fn cleanup(&mut self) {
        let mut l = self.texture_drop_list.borrow_mut();
        for key in l.drain(..) {
            self.textures.remove(key);
            self.textures_sm.remove(key);
        }
    }

    fn present(&mut self) {
        let pal = &self.palette;
        let pal_overlay = &self.palette_overlay;
        let src = &self.back_buf.data;
        let src_width = self.back_buf.width;
        self.canvas_texture.with_lock(None, |dst, stride| {
            for (src_row, dst_row) in src.chunks(src_width as usize).zip(dst.chunks_mut(stride)) {
                for (&src_pixel, dst_pixel) in src_row.iter().zip(dst_row.chunks_mut(3)) {
                    let rgb = pal_overlay.get(src_pixel)
                        .unwrap_or_else(|| pal.rgb18(src_pixel))
                        .scale::<Color8>();
                    dst_pixel[0] = rgb.r();
                    dst_pixel[1] = rgb.g();
                    dst_pixel[2] = rgb.b();
                }
            }
        }).unwrap();
        self.canvas.copy(&self.canvas_texture, None, None).unwrap();
        self.canvas.present();
    }

    fn update(&mut self, time: Instant) {
        self.palette_overlay.rotate(time);
    }

    fn draw(&mut self, tex: &TextureHandle, x: i32, y: i32, light: u32) {
        let pal = &self.palette;
        let tex = &self.textures_sm[tex.0.key];
        let light = (light >> 9) as u8;

        Self::do_draw(&mut self.back_buf, x, y, tex, &self.clip_rect,
            |dst, _, _, _, _, src| {
                *dst = pal.darken(src, light);
            }
        );
    }

    fn draw_multi_light(&mut self, tex: &TextureHandle, x: i32, y: i32, lights: &[u32]) {
        self.light_map.build(lights);

        let pal = &self.palette;
        let light_map = &self.light_map;

        let tex = &self.textures_sm[tex.0.key];

        Self::do_draw(&mut self.back_buf, x, y, tex, &self.clip_rect,
            |dst, _, _, src_x, src_y, src| {
                let light = light_map.get(src_x, src_y + 2 /* as in original */);
                *dst = pal.darken(src, (light >> 9) as u8);
            }
        );
    }

    fn draw_masked(&mut self, tex: &TextureHandle, x: i32, y: i32,
                   mask: &TextureHandle, mask_x: i32, mask_y: i32,
                   light: u32) {
        let tex = &self.textures_sm[tex.0.key];
        let mask = &self.textures_sm[mask.0.key];

        let mask_rect = &Rect::with_size(mask_x, mask_y, mask.width, mask.height);
        let light = (light >> 9) as u8;

        let pal = &self.palette;

        Self::do_draw(&mut self.back_buf, x, y, tex, &self.clip_rect,
            |dst, dst_x, dst_y, _, _, src| {
                let src = pal.darken(src, light);
                let in_mask_x = dst_x - mask_x;
                let in_mask_y = dst_y - mask_y;
                let mask_v = if mask_rect.contains(dst_x, dst_y) {
                    let i = (dst_y - mask_y) * mask.width + dst_x - mask_x;
                    cmp::min(mask.data[i as usize], 128)
                } else {
                    0
                };
                *dst = if mask_v > 0 {
                    let masked_src = pal.darken(src, mask_v);
                    let masked_dst = pal.darken(*dst, 128 - mask_v);
                    pal.blend(masked_src, masked_dst)
                } else {
                    src
                };
            }
        );
    }

    fn draw_translucent(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32) {
        self.do_draw_translucent(tex, x, y, color, light, Rgb15::grayscale)
    }

    fn draw_translucent_dark(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32) {
        self.do_draw_translucent(tex, x, y, color, light, Rgb15::grayscale_dark)
    }
}

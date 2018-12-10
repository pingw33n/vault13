use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture as SdlTexture};
use sdl2::video::Window;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::cmp;
use std::rc::Rc;
use std::cell::{Ref, RefCell};

use super::*;
use graphics::color::{Color8, Palette, PaletteOverlay};
use graphics::lighting::light_map::{self, LightMap};
use graphics::Rect;

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

struct TexturesInner {
    handles: SlotMap<DefaultKey, ()>,
    textures: SecondaryMap<DefaultKey, Texture>,
    drop_list: Rc<RefCell<Vec<DefaultKey>>>,
}

impl TexturesInner {
    fn new() -> Self {
        Self {
            handles: SlotMap::new(),
            textures: SecondaryMap::new(),
            drop_list: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn new_texture(&mut self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        let key = self.handles.insert(());
        self.textures.insert(key, Texture::new(width, height, data));
        TextureHandle(Rc::new(TextureHandleInner {
            key,
            drop_list: self.drop_list.clone(),
        }))
    }

    fn cleanup(&mut self) {
        let mut l = self.drop_list.borrow_mut();
        for key in l.drain(..) {
            self.handles.remove(key);
            self.textures.remove(key);
        }
    }
}

#[derive(Clone)]
pub(in super) struct Textures(Rc<RefCell<TexturesInner>>);

impl Textures {
    fn new() -> Self {
        Textures(Rc::new(RefCell::new(TexturesInner::new())))
    }

    fn cleanup(&self) {
        self.0.borrow_mut().cleanup();
    }
}

impl Textures {
    pub fn new_texture(&self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        self.0.borrow_mut().new_texture(width, height, data)
    }

    fn get(&self, h: &TextureHandle) -> Ref<Texture> {
        let t = self.0.borrow();
        Ref::map(t, |t| &t.textures[h.0.key])
    }
}

pub struct SoftwareRenderer {
    textures: Textures,
    light_map: LightMap,
    back_buf: Texture,
    palette: Box<Palette>,
    palette_overlay: PaletteOverlay,
    canvas: Canvas<Window>,
    canvas_texture: SdlTexture,
    clip_rect: Rect,
}

impl SoftwareRenderer {
    pub fn new(canvas: Canvas<Window>, palette: Box<Palette>, palette_overlay: PaletteOverlay) -> Self {
        let (w, h) = canvas.window().size();
        println!("{} {} ", w, h);
        let canvas_texture = canvas
            .texture_creator()
            .create_texture_streaming(PixelFormatEnum::RGB24, w, h)
            .unwrap();
        Self {
            textures: Textures::new(),
            light_map: LightMap::new(),
            back_buf: Texture::new_empty(w as i32, h as i32, 0),
            palette,
            palette_overlay,
            canvas,
            canvas_texture,
            clip_rect: Rect::with_size(0, 0, w as i32, h as i32),
        }
    }

    pub fn canvas(&self) -> &Canvas<Window> {
        &self.canvas
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas<Window> {
        &mut self.canvas
    }

    fn do_draw_translucent(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32,
            grayscale_func: impl Fn(Rgb15) -> u8) {
        let pal = &self.palette;
        let tex = self.textures.get(tex);
        let light = (light >> 9) as u8;

        let color = pal.color_idx(color);

        Self::do_draw(&mut self.back_buf, x, y, &tex, &self.clip_rect,
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

impl Renderer for SoftwareRenderer {
    fn new_texture_factory(&self) -> TextureFactory {
        TextureFactory(TextureFactoryInner::Software(self.textures.clone()))
    }

    fn cleanup(&mut self) {
        self.textures.cleanup();
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
        let tex = self.textures.get(tex);
        let light = (light >> 9) as u8;

        Self::do_draw(&mut self.back_buf, x, y, &tex, &self.clip_rect,
            |dst, _, _, _, _, src| {
                *dst = pal.darken(src, light);
            }
        );
    }

    fn draw_multi_light(&mut self, tex: &TextureHandle, x: i32, y: i32, lights: &[u32]) {
        let mut uniform = true;
        for i in 1..light_map::VERTEX_COUNT {
            if lights[i] != lights[i - 1] {
                uniform = false;
                break;
            }
        }
        if uniform {
            self.draw(tex, x, y, lights[0]);
            return;
        }

        self.light_map.build(lights);

        let pal = &self.palette;
        let light_map = &self.light_map;

        let tex = self.textures.get(tex);

        Self::do_draw(&mut self.back_buf, x, y, &tex, &self.clip_rect,
            |dst, _, _, src_x, src_y, src| {
                let light = light_map.get(src_x, src_y + 2 /* as in original */);
                *dst = pal.darken(src, (light >> 9) as u8);
            }
        );
    }

    fn draw_masked(&mut self, tex: &TextureHandle, x: i32, y: i32,
                   mask: &TextureHandle, mask_x: i32, mask_y: i32,
                   light: u32) {
        let tex = self.textures.get(tex);
        let mask = self.textures.get(mask);

        let mask_rect = &Rect::with_size(mask_x, mask_y, mask.width, mask.height);
        let light = (light >> 9) as u8;

        let pal = &self.palette;

        Self::do_draw(&mut self.back_buf, x, y, &tex, &self.clip_rect,
            |dst, dst_x, dst_y, _, _, src| {
                let src = pal.darken(src, light);
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

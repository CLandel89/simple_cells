use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::Sdl;
use sdl2::surface::Surface;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::image::Sdl2ImageContext;
use sdl2::image::SaveSurface;

use automata;

pub struct Window {
    sdl_context: Sdl,
    sdl_img_context: Sdl2ImageContext,
    sdl_canvas: Canvas<sdl2::video::Window>,
    pub exit_issued: bool,
}

impl Window {
    pub fn new (prefs_json: &json::JsonValue) -> Window {
        let sdl_context = sdl2::init().unwrap();
        let sdl_img_context = sdl2::image::init(sdl2::image::InitFlag::PNG).unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let w: u32 = prefs_json["window_w"].as_u32().unwrap();
        let h: u32 = prefs_json["window_h"].as_u32().unwrap();
        let window = video_subsystem.window("simple_cells", w, h)
            .position_centered()
            .build()
            .unwrap();
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.clear();
        canvas.present();
        Window {
            sdl_context: sdl_context,
            sdl_img_context: sdl_img_context,
            sdl_canvas: canvas,
            exit_issued: false,
        }
    }
    pub fn fill (&mut self, r:u8, g:u8, b:u8) {
        self.set_draw_color(r, g, b);
        let (w,h) = self.sdl_canvas.output_size().unwrap();
        self.sdl_canvas.fill_rect(Rect::new(0,0,w,h));
    }
    pub fn set_draw_color (&mut self, r:u8, g:u8, b:u8) {
        self.sdl_canvas.set_draw_color(Color::RGB(r,g,b));
    }
    pub fn draw_point (&mut self, x:usize, y:usize) {
        self.sdl_canvas.draw_point(Point::new(x as i32, y as i32));
    }
    pub fn present (&mut self) {
        self.sdl_canvas.present();
        // Exit issued? This is a variant of the SDL2 Rust binding example: https://docs.rs/sdl2/0.34.5/sdl2/index.html
        let mut event_pump = self.sdl_context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    self.exit_issued = true;
                },
                _ => {}
            }
        }
    }
    pub fn seed_png (&self) -> ((usize,usize), Vec<Vec<u8>>) {
        let surf: Surface = sdl2::image::LoadSurface::from_file("seed.png").unwrap();
        let w = surf.width() as usize;
        let h = surf.height() as usize;
        let mut rows = Vec::<Vec<u8>>::with_capacity(h);
        unsafe {
            let pitch = (*surf.raw()).pitch as usize;
            let pixels = (*surf.raw()).pixels as *const u8;
            for y in 0..h {
                let mut row = vec![0; ((w as f64) / 8f64).ceil() as usize];
                for x in 0..w {
                    let v = *pixels.offset((y*pitch + x) as isize) == 0;
                    row[x/8] |= (v as u8) << (x%8);
                }
                rows.push(row);
            }
        }
        ((w,h), rows)
    }
    fn exit_issued (&self) -> bool {
        return self.exit_issued;
    }
    pub fn snapshot_png (&self, automata: &automata::Automata, path: &str) {
        let (w, h) = (automata.w, automata.h);
        let surf = Surface::new(
            w as u32,
            h as u32,
            sdl2::pixels::PixelFormatEnum::RGB332 //1 byte per pixel
        ).unwrap();
        let pitch = surf.pitch();
        unsafe {
            let pixels = (*surf.raw()).pixels as *mut u8;
            for y in 0..h {
                for x in 0..w {
                    let pixel_i = (y*(pitch as usize) + x) as isize;
                    if automata.get(x,y) {
                        *pixels.offset(pixel_i) = 0;
                    } else {
                        *pixels.offset(pixel_i) = 255;
                    }
                }
            }
        }
        surf.save(path).unwrap();
    }
}
extern crate sdl2;

mod automata;
mod window;

use std::time::Duration;
use std::time::Instant;


fn main () {
    let prefs_json = json::parse(
            & std::fs::read_to_string("prefs.json")
                .expect("Please ChDir to the path with the seed files and prefs.json.")
        ).unwrap();
    let win_w = prefs_json["window_w"].as_usize().unwrap();
    let win_h = prefs_json["window_h"].as_usize().unwrap();
    let mut window = window::Window::new(&prefs_json);
    let (win_w_by_x, win_h_by_y);
    let mut automata;
    {
        let ((w,h), seed) = window.seed_png();
        win_w_by_x = (w as f64) / (win_w as f64);
        win_h_by_y = (h as f64) / (win_h as f64);
        automata = automata::Automata::new(w, h);
        for y in 0..h {
            let row = &seed[y];
            for x in 0..w {
                let v = (row[x/8] >> (x%8)) & 1 != 0;
                automata.set(x, y, v);
            }
        }
    }
    let mut n = 0usize;
    let mut rpf = 10isize; //playing rounds per frame
    let mut t_counter = Instant::now();
    let mut f_counter = 0usize;
    let /*const*/ SECOND: Duration = Duration::new(1, 0);
    let fps = prefs_json["fps"].as_usize().unwrap();
    loop {
        window.fill(0,0,0);
        window.set_draw_color(255,255,255);
        for wy in 0..win_h {
            for wx in 0..win_w {
                //TODO: spread the "get"s over the field or something
                let x = (wx as f64 * win_w_by_x).round() as usize;
                let y = (wy as f64 * win_h_by_y).round() as usize;
                if automata.get(x, y) {
                    window.draw_point(wx, wy);
                }
            }
        }
        window.present();
        f_counter += 1;
        if window.exit_issued {
            break;
        }
        for _ in 0..rpf {
            automata.play();
            n += 1;
        }
        if t_counter.elapsed() >= SECOND {
            t_counter += SECOND;
            if f_counter < fps*9/10 {
                rpf -= 1 + rpf/10;
                if rpf <= 0 {
                    rpf = 1;
                }
            }
            if f_counter > fps*11/10 {
                rpf += 1 + rpf/10;
            }
            f_counter = 0;
        }
    }
}

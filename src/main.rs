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
        let n_threads = prefs_json["n_threads"].as_usize().unwrap();
        automata = automata::Automata::new(w, h, n_threads);
        for y in 0..h {
            let row = &seed[y];
            for x in 0..w {
                let v = (row[x/8] >> (x%8)) & 1 != 0;
                automata.set(x, y, v);
            }
        }
    }
    let mut n = 0_usize;
    let mut rpf = 1_f64; //playing rounds per frame
    let mut t_counter = Instant::now();
    let mut f_counter = 0_usize;
    let mut r_counter = 0_isize;
    let /*const*/ SECOND: Duration = Duration::new(1, 0);
    let fps = prefs_json["fps"].as_f64().unwrap();
    let spf = 1_f64 / fps;
    loop {
        window.fill(0,0,0);
        window.set_draw_color(255,255,255);
        for wy in 0..win_h {
            for wx in 0..win_w {
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
        automata.play(rpf as usize);
        n += rpf as usize;
        r_counter += rpf as isize;
        let elapsed = t_counter.elapsed();
        if f_counter == 16 || elapsed >= SECOND {
            t_counter = Instant::now();
            let dur = (elapsed.as_millis() as f64) / 1000.0;
            let rps = (r_counter as f64) / dur;
            let rpf_new = rps / fps;
            rpf = (3.0*rpf + rpf_new) / 4.0;
            if rpf < 1.0 {
                rpf = 1.0;
            }
            f_counter = 0;
            r_counter = 0;
        }
    }
}

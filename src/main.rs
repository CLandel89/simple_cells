extern crate chrono;
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
    let (w, h);
    {
        let seed;
        let seed_tuple = window.seed_png(); // ((x,y),seed)
        w = seed_tuple.0.0;
        h = seed_tuple.0.1;
        seed = seed_tuple.1;
        win_w_by_x = (w as f64) / (win_w as f64);
        win_h_by_y = (h as f64) / (win_h as f64);
        let gpu_i = prefs_json["gpu_i"].as_usize().unwrap();
        automata = automata::Automata::new(w, h, gpu_i).unwrap();
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
    let snapshots = prefs_json["snapshots"].as_isize().unwrap();
    let snapshots_dir = chrono::Local::now().format("%y%m%d.%H%M%S");
    let snapshots_dir = format!("{}", snapshots_dir);
    if snapshots > 0 {
        std::fs::create_dir(&snapshots_dir).unwrap();
        std::fs::copy(
            "seed.json",
            &format!("{}/seed.json", &snapshots_dir)
        );
        std::fs::copy(
            "seed.png",
            &format!("{}/00000000000000000000.png", &snapshots_dir)
        ).unwrap();
    }
    let mut snapshot_counter = 0_f64;
    let mut snapshot_trigger = false;
    loop {
        window.fill(0,0,0);
        window.set_draw_color(255,255,255);
        for wy in 0..win_h {
            for wx in 0..win_w {
                let x = (wx as f64 * win_w_by_x).round() as usize;
                let y = (wy as f64 * win_h_by_y).round() as usize;
                if (x >= automata.field0.w) {
                    continue;
                }
                if (y >= automata.field0.h) {
                    continue;
                }
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
        if snapshots > 0 {
            if snapshot_counter+rpf >= snapshots as f64 {
                snapshot_trigger = true;
                rpf = snapshots as f64 - snapshot_counter;
            }
        }
        automata.play(rpf as usize);
        n += rpf as usize;
        snapshot_counter += rpf as usize as f64;
        if snapshot_trigger {
            window.snapshot_png(
                &automata,
                &format!("{}/{:020}.png", &snapshots_dir, n)
            );
            snapshot_trigger = false;
            snapshot_counter = 0.0;
        }
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

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
    let gpu_i = prefs_json["gpu_i"].as_usize().unwrap();
    let mut automata = automata::Automata::new(&window, gpu_i).unwrap();
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
        window.present(&automata);
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

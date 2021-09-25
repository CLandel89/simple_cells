extern crate chrono;
extern crate sdl2;

mod automata;
mod utils;
mod window;

use std::time::Duration;
use std::time::Instant;


fn main ()
{
    let prefs_json = json::parse(
            & std::fs::read_to_string("prefs.json")
                .expect("Please ChDir to the path with the seed files and prefs.json.")
        ).unwrap();
    let seed_json = json::parse(
            & std::fs::read_to_string("seed.json")
                .expect("Please ChDir to the path with the seed files and prefs.json.")
        ).unwrap();
    let mut window = window::Window::new(&prefs_json);
    let gpu_i = prefs_json["gpu_i"].as_usize().unwrap();
    let mut automata = automata::Automata::new(&window, gpu_i, &seed_json).unwrap();
    let (w, h) = (automata.w, automata.h);
    let mut n = seed_json["n"].as_usize().unwrap();
    let mut rpf = 1_f64; //playing rounds per frame
    let mut t_counter = Instant::now();
    let mut f_counter = 0_usize;
    let mut r_counter = 0_isize;
    let /*const*/ second: Duration = Duration::new(1, 0);
    let fps = prefs_json["fps"].as_f64().unwrap();
    let snapshots = prefs_json["snapshots"].as_isize().unwrap();
    let snapshots_dir = chrono::Local::now().format("%y%m%d.%H%M%S");
    let snapshots_dir = format!("{}", snapshots_dir);
    if snapshots > 0 {
        std::fs::create_dir(&snapshots_dir).unwrap();
        std::fs::copy(
            "seed.json",
            &format!("{}/seed.json", &snapshots_dir)
        ).unwrap();
        std::fs::copy(
            "seed.png",
            &format!("{}/{:020}.png", &snapshots_dir, n)
        ).unwrap();
    }
    let mut snapshot_counter = 0_f64;
    let mut snapshot_trigger = false;
    let mut snapshot_restore_rpf = 1_f64;
    let benchmark_print = prefs_json["benchmark_print"].as_f64().unwrap();
    let mut benchmark_counter = 0;
    let mut benchmark_t = Instant::now();

    loop
    {
        window.present(&automata);
        f_counter += 1;
        if window.exit_issued {
            break;
        }

        if snapshots > 0 {
            if snapshot_counter+rpf >= snapshots as f64 {
                snapshot_trigger = true;
                snapshot_restore_rpf = rpf;
                rpf = snapshots as f64 - snapshot_counter;
            }
        }

        automata.play(rpf as usize);
        n += rpf as usize;
        snapshot_counter += rpf as usize as f64;
        r_counter += rpf as isize;
        benchmark_counter += rpf as usize;

        if snapshot_trigger {
            window.snapshot_png(
                &automata,
                &format!("{}/{:020}.png", &snapshots_dir, n)
            );
            snapshot_trigger = false;
            snapshot_counter = 0.0;
            rpf = snapshot_restore_rpf;
        }

        let elapsed = t_counter.elapsed();
        if f_counter == 16 || elapsed >= second {
            t_counter = Instant::now();
            let dur = (elapsed.as_millis() as f64) / 1000.0;
            let rps = (r_counter as f64) / dur;
            let rpf_new = rps / fps;
            rpf = (7.0*rpf + rpf_new) / 8.0;
            if rpf < 1.0 {
                rpf = 1.0;
            }
            f_counter = 0;
            r_counter = 0;
        }

        if benchmark_print > 0.0 {
            let elapsed = benchmark_t.elapsed().as_millis() as f64 / 1000.0;
            if elapsed >= benchmark_print {
                utils::benchmark_print(
                    (benchmark_counter*w*h) as f64,
                    elapsed
                );
                benchmark_t = Instant::now();
                benchmark_counter = 0;
            }
        }

    }

}

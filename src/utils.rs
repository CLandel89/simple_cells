pub fn benchmark_print (n: f64, s: f64) {
    //prefixes:
    //1 to 3 decimal places => no prefix
    //4 to 6 => k
    //7 to 9 => M
    //(1+3*i) up to incl. (3+3*i) => PREF[i]
    //https://en.wikipedia.org/wiki/Unit_prefix
    let /*const*/ pref: Vec<_> = " kMGTPEZY".chars().collect();
    //nps:
    //e.g. 2000 in 2s => 1000 per 1s
    let nps = n / s;
    let i = nps.log(1000.0).floor() as usize;
    let disp = nps / 1000_f64.powf(i as f64);
    let prec = 3 - (disp.log(10.0).floor() as usize + 1);
    println!(
        "n calculated cells: {0:.1$} {2} / s",
        disp,
        prec,
        pref[i]
    );
}
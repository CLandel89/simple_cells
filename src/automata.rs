extern crate crossbeam;

pub struct Field {
    data: Vec<Vec<u8>>,
    pub w: usize,
    pub h: usize,
}
impl Field {
    pub fn new (w:usize, h:usize) -> Field {
        let pitch = ((w as f64) / 8_f64).ceil() as usize;
        let mut data: Vec<Vec<u8>> = Vec::with_capacity(h);
        for _ in 0 .. h {
            let mut row = Vec::<u8>::with_capacity(pitch);
            row.resize(pitch, 0);
            data.push(row);
        }
        Field {
            data: data,
            w: w,
            h: h,
        }
    }
    pub fn get (&self, x:usize, y:usize) -> bool {
        ((self.data[y][x/8] >> (x%8)) & 1) != 0
    }
    pub fn set (&mut self, x:usize, y:usize, v:bool) {
        let v8 = (v as u8) << (x%8);
        let mut d = self.data[y][x/8];
        d &= !(1 << (x%8));
        d |= v8;
        self.data[y][x/8] = d;
    }
}

// These indices are for 3×3 surrounding environments.
const TL:usize=0; const TM:usize=1; const TR:usize=2;
const ML:usize=3; const MM:usize=4; const MR:usize=5;
const BL:usize=6; const BM:usize=7; const BR:usize=8;

/*
The "table" is the storage for all 2×1 results of 4×3 field slices.
There are 2^(4×­3)=4096 2-bit lookup values. A u8 can hold 4 such values.
TODO: Document how and why.
*/
pub struct Table {
    values: [u8; 4096/4],
}
impl Table {
    pub fn new (borns: u16, survives: u16) -> Self {
        let mut new = Self {
            values: [0; 4096/4],
        };
        for env in 0..4096 {
            let mut value = 0u8;
            //the counting in this closure needs to be done for both result bits
            let check_accountable_bits = |accountable_bits: [usize; 8], mid: usize| {
                let mut count = 0;
                for accountable_bit in &accountable_bits {
                    if (env >> accountable_bit) & 1 != 0 {
                        count += 1;
                    }
                }
                if (env >> mid) & 1 != 0 {
                    if (survives >> count) & 1 != 0 {
                        return true;
                    }
                }
                else {
                    if (borns >> count) & 1 != 0 {
                        return true;
                    }
                }
                return false;
            };
            // env shifting numbers:
            //  0  1  2  3
            //  4  5  6  7
            //  8  9 10 11
            //left result bit
            if check_accountable_bits([0,1,2, 4,6, 8,9,10], 5) {
                value |= 1 << 0;
            }
            //right result bit
            if check_accountable_bits([1,2,3, 5,7, 9,10,11], 6) {
                value |= 1 << 1;
            }
            //0 to 2 bits are set in "value", now store
            new.set(env, value);
        }
        new
    }
    fn get (&self, env: u16) -> u8 {
        //get the u8 with the entry and 1 other entry
        let d = self.values[env as usize / 4];
        //shift the entry to the LSB and clear any MSB past 2 bits
        ((d >> ((env%4)*2)) & 3)
    }
    fn set (&mut self, env: u16, value: u8) {
        //get the u8 with the entry and 1 other entry
        let mut d = self.values[env as usize / 4];
        //clear the 4 bits of the entry
        d &= !(3 << ((env%4)*2));
        //set the entry
        d |= (value as u8) << ((env%4)*2);
        //store
        self.values[env as usize / 4] = d;
    }
    /*
    Calculates a new byte from the environment of 9 bytes.
    In other words: New 8×1 slice from an 8×3 slice.
    */
    pub fn work_u8 (&self, env: &[u8;9]) -> u8 {
        let mut result: u8 = 0;
        let collect_env12 = |spot: &[(usize,usize); 12]| {
            let mut env12 = 0_u16;
            for si in 0..12 {
                let u8_i = spot[si].0;
                let u8_shift = spot[si].1;
                let bit = (env[u8_i] >> u8_shift) & 1;
                env12 |= (bit as u16) << si;
            }
            env12
        };
        let env_0 = collect_env12(&[
            (TL,7), (TM,0), (TM,1), (TM,2),
            (ML,7), (MM,0), (MM,1), (MM,2),
            (BL,7), (BM,0), (BM,1), (BM,2)
        ]);
        result |= self.get(env_0) << 0;
        let env_2 = collect_env12(&[
            (TM,1), (TM,2), (TM,3), (TM,4),
            (MM,1), (MM,2), (MM,3), (MM,4),
            (BM,1), (BM,2), (BM,3), (BM,4)
        ]);
        result |= self.get(env_2) << 2;
        let env_4 = collect_env12(&[
            (TM,3), (TM,4), (TM,5), (TM,6),
            (MM,3), (MM,4), (MM,5), (MM,6),
            (BM,3), (BM,4), (BM,5), (BM,6)
        ]);
        result |= self.get(env_4) << 4;
        let env_6 = collect_env12(&[
            (TM,5), (TM,6), (TM,7), (TR,0),
            (MM,5), (MM,6), (MM,7), (MR,0),
            (BM,5), (BM,6), (BM,7), (BR,0)
        ]);
        result |= self.get(env_6) << 6;
        result
    }
}

pub struct Worker<'a> {
    source: &'a Field,
    tx: crossbeam::channel::Sender <(usize,Vec<u8>)>,
    table: &'a Table,
    n_threads: usize,
    thread_i: usize,
}
impl<'a> Worker<'a> {
    pub fn play (&mut self) {
        let source = &self.source;
        let table = &self.table;
        let w = source.w;
        let h = source.h;
        let w8 = source.data[0].len();
        let mut data = Vec::<u8>::with_capacity(w8);
        data.resize(w8, 0);
        // in case there are bits in each last byte that need to be cleared
        let mut cutoff = 0_u8;
        if w%8 != 0 {
            for ci in w%8 .. 8 {
                cutoff |= 1 << ci;
            }
        }
        // source field rows
        let (mut trow, mut mrow, mut brow);
        // top row
        let y = 0;
        if y % self.n_threads == self.thread_i {
            mrow = &source.data[y];
            brow = &source.data[y+1];
            // top left corner
            let x8 = 0;
            data[x8] = table.work_u8(&[
                0, 0, 0,
                0, mrow[0], mrow[1],
                0, brow[0], brow[1]
            ]);
            // top stripe
            for x8 in 1 .. w8-1 {
                data[x8] = table.work_u8(&[
                    0, 0, 0,
                    mrow[x8-1], mrow[x8], mrow[x8+1],
                    brow[x8-1], brow[x8], brow[x8+1]
                ]);
            }
            // top right corner
            let x8 = mrow.len()-1;
            data[x8] = table.work_u8(&[
                0, 0, 0,
                mrow[w8-2], mrow[w8-1], 0,
                brow[w8-2], brow[w8-1], 0
            ]);
            // top cutoff
            data[x8] &= !cutoff;
            // top tx
            self.tx.send((y,data.clone())).unwrap();
        }
        // mid rows
        for y in 1..h-1 {
            if y % self.n_threads != self.thread_i {
                continue;
            }
            trow = &source.data[y-1];
            mrow = &source.data[y];
            brow = &source.data[y+1];
            // left edge
            let x8 = 0;
            data[x8] = table.work_u8(&[
                0, trow[0], trow[1],
                0, mrow[0], mrow[1],
                0, brow[0], brow[1]
            ]);
            // mid
            for x8 in 1 .. w8-1 {
                data[x8] = table.work_u8(&[
                    trow[x8-1], trow[x8], trow[x8+1],
                    mrow[x8-1], mrow[x8], mrow[x8+1],
                    brow[x8-1], brow[x8], brow[x8+1]
                ]);
            }
            // right edge
            let x8 = w8-1;
            data[x8] = table.work_u8(&[
                trow[x8-1], trow[x8], 0,
                mrow[x8-1], mrow[x8], 0,
                brow[x8-1], brow[x8], 0
            ]);
            // mid cutoff
            data[x8] &= !cutoff;
            // mid tx
            self.tx.send((y,data.clone())).unwrap();
        }
        // bottom row
        let y = h-1;
        if y % self.n_threads == self.thread_i {
            trow = &source.data[y-1];
            mrow = &source.data[y];
            // bottom left corner
            let x8 = 0;
            data[x8] = table.work_u8(&[
                0, trow[x8], trow[x8+1],
                0, mrow[x8], mrow[x8+1],
                0, 0, 0
            ]);
            // bottom stripe
            for x8 in 1 .. w8-1 {
                data[x8] = table.work_u8(&[
                    trow[x8-1], trow[x8], trow[x8+1],
                    mrow[x8-1], mrow[x8], mrow[x8+1],
                    0, 0, 0
                ]);
            }
            // bottom right corner
            let x8 = w8-1;
            data[x8] = table.work_u8(&[
                trow[x8-1], trow[x8], 0,
                mrow[x8-1], trow[x8], 0,
                0, 0, 0
            ]);
            // bottom cutoff
            data[x8] &= !cutoff;
            // bottom tx
            self.tx.send((y,data.clone())).unwrap();
        }
    }
}

pub struct Automata {
    seed_json: json::JsonValue,
    w: usize,
    h: usize,
    //these work like a double buffer
    field0: Field,
    field1: Field,
    fields_swapped: bool,
    //optimization
    table: Table,
    n_threads: usize,
}

impl Automata {
    pub fn new (w:usize, h:usize, n_threads:usize) -> Automata {
        let seed_json = json::parse(
                & std::fs::read_to_string("seed.json")
                    .expect("Please ChDir to the path with the seed files and prefs.json.")
            ).unwrap();
        let mut borns = 0u16;
        let mut survives = 0u16;
        {
            let mut bs_str = seed_json["rulestring"].as_str().unwrap().split("/");
            let mut b_str = bs_str.next().unwrap().chars();
            assert_eq!('B', b_str.next().unwrap());
            for born_c in b_str {
                let born_i = born_c.to_digit(9).unwrap();
                borns |= 1 << born_i;
            }
            let mut s_str = bs_str.next().unwrap().chars();
            assert_eq!('S', s_str.next().unwrap());
            for survive_c in s_str {
                let survive_i = survive_c.to_digit(9).unwrap();
                survives |= 1 << survive_i;
            }
        }
        Automata {
            w: w,
            h: h,
            field0: Field::new(w,h),
            field1: Field::new(w,h),
            fields_swapped: false,
            seed_json: seed_json,
            table: Table::new(borns, survives),
            n_threads: n_threads,
        }
    }
    // Plays n rounds of Game Of Life or so.
    pub fn play (&mut self, n_rounds: usize) {
        for _ in 0..n_rounds {
            let (source, target) =
                if self.fields_swapped {
                    (&self.field1, &mut self.field0)
                } else {
                    (&self.field0, &mut self.field1)
                };
            let h = source.h;
            let table = &self.table;
            let n_threads = self.n_threads;
            let (tx, rx):
                (
                    crossbeam::channel::Sender <(usize,Vec<u8>)>,
                    crossbeam::channel::Receiver <(usize,Vec<u8>)>
                )
                =
                    crossbeam::channel::bounded(0);
            crossbeam::scope(|s| {
                for thread_i in 0..n_threads {
                    let tx_clone = tx.clone();
                    s.spawn(move |_| {
                        let mut worker = Worker {
                            source: source,
                            tx: tx_clone,
                            table: table,
                            n_threads: n_threads,
                            thread_i: thread_i,
                        };
                        worker.play();
                    });
                }
                let mut n_collected = 0_usize;
                for (y, data) in rx {
                    n_collected += 1;
                    target.data[y] = data;
                    if n_collected == h {
                        break;
                    }
                }
            });
            self.fields_swapped = !self.fields_swapped;
        }
    }
    pub fn get (&self, x:usize, y:usize) -> bool {
        let field = if self.fields_swapped { &self.field1 } else { &self.field0 };
        field.get(x,y)
    }
    pub fn set (&mut self, x:usize, y:usize, v:bool) {
        let field = if self.fields_swapped { &mut self.field1 } else { &mut self.field0 };
        field.set(x,y,v);
    }
}
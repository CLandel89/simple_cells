const CHUNK_W: usize = 64;

/*
A "chunk" is a CHUNK_W × CHUNK_W piece of the field.
*/
pub struct Chunk {
    data: [u8; CHUNK_W*CHUNK_W/8],
}
impl Chunk {
    pub fn new () -> Chunk {
        Chunk {
            data: [0; CHUNK_W*CHUNK_W/8],
        }
    }
    pub fn get (&self, x:usize, y:usize) -> bool {
        let di = y * CHUNK_W / 8 + x / 8;
        let d = self.data[di];
        (d >> (x%8)) & 1 != 0
    }
    pub fn set (&mut self, x:usize, y:usize, v:bool) {
        let di = y * CHUNK_W / 8 + x / 8;
        let mut d = self.data[di];
        let v8 = (v as u8) << (x%8);
        let clear = (1 << (x%8)) ^ v8;
        d |= v8; //sets the bit if v was set
        d &= !clear; //clears the bit if v was not set
        self.data[di] = d;
    }
    // Sets a 2×2 piece of the chunk, assuming x%2==0 and y%2==0.
    pub fn set4 (&mut self, x:usize, y:usize, v:u8) {
        self.set(x,y, v&0b0001 != 0);
        self.set(x+1,y, v&0b0010 != 0);
        self.set(x,y+1, v&0b0100 != 0);
        self.set(x+1,y+1, v&0b1000 != 0);
    }
}

pub struct Field {
    rows: Vec<Vec<Chunk>>,
}
impl Field {
    pub fn new (w:usize, h:usize) -> Field {
        let n_chunks_per_row = 2 + (w as f64 / (CHUNK_W as f64)).ceil() as usize;
        let n_rows = 2 + (h as f64 / (CHUNK_W as f64)).ceil() as usize;
        let mut rows: Vec<Vec<Chunk>> = Vec::with_capacity(n_rows);
        for _ in 0..n_rows {
            let mut row: Vec<Chunk> = Vec::with_capacity(n_chunks_per_row);
            for _ in 0..n_chunks_per_row {
                row.push(Chunk::new());
            }
            rows.push(row);
        }
        Field {
            rows: rows,
        }
    }
}

/*
The "table" is the storage for all 2×2 results of 4×4 field slices.
There are 2^(4×­4)=65536 4-bit lookup values. A u8 can hold 2 such values.
TODO: Document how and why.
*/
pub struct Table {
    values: [u8; 65536/2],
}
impl Table {
    pub fn new (borns: u16, survives: u16) -> Table {
        let mut table = Table {
            values: [0; 65536/2],
        };
        for env in 0..=65535 {
            let mut value = 0u8;
            //the counting in this closure needs to be done for all 4 result bits
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
            //  0  1  2  3
            //  4  5  6  7
            //  8  9 10 11
            // 12 13 14 15
            //upper left result bit
            if check_accountable_bits([0,1,2, 4,6, 8,9,10], 5) {
                value |= 1 << 0;
            }
            //upper right result bit
            if check_accountable_bits([1,2,3, 5,7, 9,10,11], 6) {
                value |= 1 << 1;
            }
            //lower left result bit
            if check_accountable_bits([4,5,6, 8,10, 12,13,14], 9) {
                value |= 1 << 2;
            }
            //lower right result bit
            if check_accountable_bits([5,6,7, 9,11, 13,14,15], 10) {
                value |= 1 << 3;
            }
            //0 to 4 bits are set in "value", now store
            table.set(env, value);
        }
        table
    }
    pub fn get (&self, env: u16) -> u8 {
        //get the u8 with the entry and 1 other entry
        let d = self.values[env as usize / 2];
        //shift the entry to the LSB and clear any MSB past 4 bits
        ((d >> ((env%2)*4)) & 0xf)
    }
    pub fn set (&mut self, env: u16, value: u8) {
        //get the u8 with the entry and 1 other entry
        let mut d = self.values[env as usize / 2];
        //clear the 4 bits of the entry
        d &= !(0xf << ((env%2)*4));
        //set the entry
        d |= (value as u8) << ((env%2)*4);
        //store
        self.values[env as usize / 2] = d;
    }
}

//designating indices for positions in a 3×3 arrangement
const TL:usize=0; const TM:usize=1; const TR:usize=2;
const ML:usize=3; const MM:usize=4; const MR:usize=5;
const BL:usize=6; const BM:usize=7; const BR:usize=8;

pub struct Worker<'a> {
    env: [&'a Chunk; 9], //a 3×3 arrangement
    target: &'a mut Chunk,
    table: &'a Table,
}
impl<'a> Worker<'a> {
    pub fn play (&mut self) {
        let env = &self.env;
        let target = &mut self.target;
        let table = &self.table;
        let mut src: u16;
        let collect_src = |bits: [(usize,usize,usize); 16]| {
            let mut src = 0u16;
            for i in 0..16 {
                let env_i = bits[i].0;
                let x = bits[i].1;
                let y = bits[i].2;
                src |= (env[env_i].get(x,y) as u16) << i;
            }
            src
        };
        // some magic numbers we'll use more often
        const CW1: usize = CHUNK_W - 1;
        const CW2: usize = CHUNK_W - 2;
        const CW3: usize = CHUNK_W - 3;
        // top left corner
        src = collect_src([
            (TL,CW1,CW1), (TM,0,CW1), (TM,1,CW1), (TM,2,CW1),
            (ML,CW1,0), (MM,0,0), (MM,1,0), (MM,2,0),
            (ML,CW1,1), (MM,0,1), (MM,1,1), (MM,2,1),
            (ML,CW1,2), (MM,0,2), (MM,1,2), (MM,2,2)
        ]);
        target.set4(0,0, table.get(src));
        // top stripe
        for x in (2..CW2).step_by(2) {
            src = collect_src([
                (TM,x-1,CW1), (TM,x,CW1), (TM,x+1,CW1), (TM,x+2,CW1),
                (MM,x-1,0), (MM,x,0), (MM,x+1,0), (MM,x+2,0),
                (MM,x-1,1), (MM,x,1), (MM,x+1,1), (MM,x+2,1),
                (MM,x-1,2), (MM,x,2), (MM,x+1,2), (MM,x+2,2)
            ]);
            target.set4(x,0, table.get(src));
        }
        // top right corner
        src = collect_src([
            (TM,CW3,CW1), (TM,CW2,CW1), (TM,CW1,CW1), (TR,0,CW1),
            (MM,CW3,0), (MM,CW2,0), (MM,CW1,0), (MR,0,0),
            (MM,CW3,1), (MM,CW2,1), (MM,CW1,1), (MR,0,1),
            (MM,CW3,2), (MM,CW2,2), (MM,CW1,2), (MR,0,2)
        ]);
        target.set4(CW2,0, table.get(src));
        // left stripe
        for y in (2..CW2).step_by(2) {
            src = collect_src([
                (ML,CW1,y-1), (MM,0,y-1), (MM,1,y-1), (MM,2,y-1),
                (ML,CW1,y), (MM,0,y), (MM,1,y), (MM,2,y),
                (ML,CW1,y+1), (MM,0,y+1), (MM,1,y+1), (MM,2,y+1),
                (ML,CW1,y+2), (MM,0,y+2), (MM,1,y+2), (MM,2,y+2)
            ]);
            target.set4(0,y, table.get(src));
        }
        // mid block
        for y in (2..CW2).step_by(2) {
            for x in (2..CW2).step_by(2) {
                src = collect_src([
                    (MM,x-1,y-1), (MM,x,y-1), (MM,x+1,y-1), (MM,x+2,y-1),
                    (MM,x-1,y), (MM,x,y), (MM,x+1,y), (MM,x+2,y),
                    (MM,x-1,y+1), (MM,x,y+1), (MM,x+1,y+1), (MM,x+2,y+1),
                    (MM,x-1,y+2), (MM,x,y+2), (MM,x+1,y+2), (MM,x+2,y+2)
                ]);
                target.set4(x,y, table.get(src));
            }
        }
        // right stripe
        for y in (2..CW2).step_by(2) {
            src = collect_src([
                (MM,CW3,y-1), (MM,CW2,y-1), (MM,CW1,y-1), (MR,0,y-1),
                (MM,CW3,y), (MM,CW2,y), (MM,CW1,y), (MR,0,y),
                (MM,CW3,y+1), (MM,CW2,y+1), (MM,CW1,y+1), (MR,0,y+1),
                (MM,CW3,y+2), (MM,CW2,y+2), (MM,CW1,y+2), (MR,0,y+2)
            ]);
            target.set4(CW2,y, table.get(src));
        }
        // bottom left corner
        src = collect_src([
            (ML,CW1,CW3), (MM,0,CW3), (MM,1,CW3), (MM,2,CW3),
            (ML,CW1,CW2), (MM,0,CW2), (MM,1,CW2), (MM,2,CW2),
            (ML,CW1,CW1), (MM,0,CW1), (MM,1,CW1), (MM,2,CW1),
            (BL,CW1,0), (BM,0,0), (BM,1,0), (BM,2,0)
        ]);
        target.set4(0,CW2, table.get(src));
        // bottom stripe
        for x in (2..CW2).step_by(2) {
            src = collect_src([
                (MM,x-1,CW3), (MM,x,CW3), (MM,x+1,CW3), (MM,x+2,CW3),
                (MM,x-1,CW2), (MM,x,CW2), (MM,x+1,CW2), (MM,x+2,CW2),
                (MM,x-1,CW1), (MM,x,CW1), (MM,x+1,CW1), (MM,x+2,CW1),
                (BM,x-1,0), (BM,x,0), (BM,x+1,0), (BM,x+2,0)
            ]);
            target.set4(x,CW2, table.get(src));
        }
        // bottom right corner
        src = collect_src([
            (MM,CW3,CW3), (MM,CW2,CW3), (MM,CW1,CW3), (MR,0,CW3),
            (MM,CW3,CW3), (MM,CW2,CW3), (MM,CW1,CW3), (MR,0,CW3),
            (MM,CW3,CW1), (MM,CW2,CW1), (MM,CW1,CW1), (MR,0,CW1),
            (BM,CW3,0), (BM,CW2,0), (BM,CW1,0), (BR,0,0)
        ]);
        target.set4(CW2,CW2, table.get(src));
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
    //the rules
    borns: u16,
    survives: u16,
    //optimization
    table: Table,
}

impl Automata {
    pub fn new (w:usize, h:usize) -> Automata {
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
            borns: borns,
            survives: survives,
            table: Table::new(borns, survives),
        }
    }
    // Plays a round of Game Of Life or so.
    pub fn play (&mut self) {
        let (source, target) = 
            if self.fields_swapped {
                (&self.field1, &mut self.field0)
            } else {
                (&self.field0, &mut self.field1)
            };
        // play on all chunks except for the margin
        for cyi in 0 .. source.rows.len()-2 {
            let cy = cyi + 1;
            for cxi in 0 .. source.rows[0].len()-2 {
                //snake movements so the cache can recycle more chunks
                let cx = if cy%2 != 0 {
                        source.rows[0].len()-2 - cxi
                    } else {
                        cxi + 1
                    };
                let mut worker = Worker {
                    env: [
                        &source.rows[cy-1][cx-1], &source.rows[cy-1][cx], &source.rows[cy-1][cx+1],
                        &source.rows[cy][cx-1], &source.rows[cy][cx], &source.rows[cy][cx+1],
                        &source.rows[cy+1][cx-1], &source.rows[cy+1][cx], &source.rows[cy+1][cx+1]
                    ],
                    target: &mut target.rows[cy][cx],
                    table: &self.table,
                };
                worker.play();
            }
        }
        // clear smaller margins, if necessairy
        if (self.borns >> 0) & 1 == 0 {
            //TODO: Check if rules like B0 work correctly
            if self.w%CHUNK_W != 0 {
                for y in 0 .. self.h+1 {
                    self.set(self.w, y, false);
                }
            }
            if self.h%CHUNK_W != 0 {
                for x in 0 .. self.w+1 {
                    self.set(x, self.h, false);
                }
            }
        }
        // the field is all set, now swap source and target
        self.fields_swapped = !self.fields_swapped;
    }
    pub fn get (&self, x:usize, y:usize) -> bool {
        let field = if self.fields_swapped { &self.field1 } else { &self.field0 };
        let rowi = 1 + y / CHUNK_W;
        let ci = 1 + x / CHUNK_W;
        let xo = x % CHUNK_W;
        let yo = y % CHUNK_W;
        field.rows[rowi][ci].get(xo, yo)
    }
    pub fn set (&mut self, x:usize, y:usize, v:bool) {
        let field = if self.fields_swapped { &mut self.field1 } else { &mut self.field0 };
        let rowi = 1 + y / CHUNK_W;
        let ci = 1 + x / CHUNK_W;
        let xo = x % CHUNK_W;
        let yo = y % CHUNK_W;
        field.rows[rowi][ci].set(xo, yo, v);
    }
}
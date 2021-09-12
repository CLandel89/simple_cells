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
            for _ in 0 .. pitch {
                row.push(0);
            }
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
    // Sets a 2×1 piece of the field, assuming x%2==0 and y%2==0, and v&(!3) == 0.
    pub fn set2 (&mut self, x:usize, y:usize, v:u8) {
        let v8 = v << (x%8);
        let mut d = self.data[y][x/8];
        d &= !(3 << (x%8));
        d |= v8;
        self.data[y][x/8] = d;
    }
}

/*
The "table" is the storage for all 2×1 results of 4×3 field slices.
There are 2^(4×­3)=4096 2-bit lookup values. A u8 can hold 4 such values.
TODO: Document how and why.
*/
pub struct Table {
    values: [u8; 4096/4],
}
impl Table {
    pub fn new (borns: u16, survives: u16) -> Table {
        let mut table = Table {
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
            table.set(env, value);
        }
        table
    }
    pub fn get (&self, env: u16) -> u8 {
        //get the u8 with the entry and 1 other entry
        let d = self.values[env as usize / 4];
        //shift the entry to the LSB and clear any MSB past 2 bits
        ((d >> ((env%4)*2)) & 3)
    }
    pub fn set (&mut self, env: u16, value: u8) {
        //get the u8 with the entry and 1 other entry
        let mut d = self.values[env as usize / 4];
        //clear the 4 bits of the entry
        d &= !(3 << ((env%4)*2));
        //set the entry
        d |= (value as u8) << ((env%4)*2);
        //store
        self.values[env as usize / 4] = d;
    }
}

pub struct Worker<'a> {
    source: &'a Field,
    target: &'a mut Field,
    table: &'a Table,
}
impl<'a> Worker<'a> {
    pub fn play (&mut self) {
        let source = &self.source;
        let target = &mut self.target;
        let table = &self.table;
        let w = source.w;
        let h = source.h;
        let mut src: u16;
        let collect_src = |bits: [Option<(usize,usize)>; 12]| {
            let mut src = 0u16;
            for i in 0..12 {
                match bits[i] {
                    Some(b) => {
                        let x = b.0;
                        let y = b.1;
                        src |= (source.get(x,y) as u16) << i;
                    },
                    None => {}
                }
            }
            src
        };
        let collect_src_noopt = |bits: [(usize,usize); 12]| {
            let mut src = 0u16;
            for i in 0..12 {
                let x = bits[i].0;
                let y = bits[i].1;
                src |= (source.get(x,y) as u16) << i;
            }
            src
        };
        // top left corner
        src = collect_src([
            None, None,        None,        None,
            None, Some((0,0)), Some((1,0)), Some((2,0)),
            None, Some((0,1)), Some((1,1)), Some((2,1))
        ]);
        target.set2(0,0, table.get(src));
        // top stripe
        for x in (2..w-2).step_by(2) {
            src = collect_src([
                None,          None,        None,          None,
                Some((x-1,0)), Some((x,0)), Some((x+1,0)), Some((x+2,0)),
                Some((x-1,1)), Some((x,1)), Some((x+1,1)), Some((x+2,1))
            ]);
            target.set2(x,0, table.get(src));
        }
        // top right corner
        src = collect_src([
            None,          None,          None,          None,
            Some((w-3,0)), Some((w-2,0)), Some((w-1,0)), None,
            Some((w-3,1)), Some((w-2,1)), Some((w-1,1)), None
        ]);
        target.set2(w-2,0, table.get(src));
        // left stripe
        for y in 1..h-1 {
            src = collect_src([
                None, Some((0,y-1)), Some((1,y-1)), Some((2,y-1)),
                None, Some((0,y)),   Some((1,y)),   Some((2,y)),
                None, Some((0,y+1)), Some((1,y+1)), Some((2,y+1))
            ]);
            target.set2(0,y, table.get(src));
        }
        // mid block
        for y in 1..h-1 {
            for x in (2..w-2).step_by(2) {
                src = collect_src_noopt([
                    (x-1,y-1), (x,y-1), (x+1,y-1), (x+2,y-1),
                    (x-1,y),   (x,y),   (x+1,y),   (x+2,y),
                    (x-1,y+1), (x,y+1), (x+1,y+1), (x+2,y+1)
                ]);
                target.set2(x,y, table.get(src));
            }
        }
        // right stripe
        for y in 1..h-1 {
            src = collect_src([
                Some((w-3,y-1)), Some((w-2,y-1)), Some((w-1,y-1)), None,
                Some((w-3,y)),   Some((w-2,y)),   Some((w-1,y)),   None,
                Some((w-3,y+1)), Some((w-2,y+1)), Some((w-1,y+1)), None
            ]);
            target.set2(w-2,y, table.get(src));
        }
        // bottom left corner
        src = collect_src([
            None, Some((0,h-2)), Some((1,h-2)), Some((2,h-2)),
            None, Some((0,h-1)), Some((1,h-1)), Some((2,h-1)),
            None, None,          None,          None
        ]);
        target.set2(0,h-1, table.get(src));
        // bottom stripe
        for x in (2..w-2).step_by(2) {
            src = collect_src([
                Some((x-1,h-2)), Some((x,h-2)), Some((x+1,h-2)), Some((x+2,h-2)),
                Some((x-1,h-1)), Some((x,h-1)), Some((x+1,h-1)), Some((x+2,h-1)),
                None,            None,          None,            None
            ]);
            target.set2(x,h-1, table.get(src));
        }
        // bottom right corner
        src = collect_src([
            Some((w-3,h-2)), Some((w-2,h-2)), Some((w-1,h-2)), None,
            Some((w-3,h-1)), Some((w-2,h-1)), Some((w-1,h-1)), None,
            None,            None,            None,            None
        ]);
        target.set2(w-2,h-1, table.get(src));
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
        let mut worker = Worker {
            source: source,
            target: target,
            table: &self.table,
        };
        worker.play();
        self.fields_swapped = !self.fields_swapped;
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
extern crate opencl3 as cl;
use automata::cl::memory::ClMem;

pub struct Field {
    data: Vec<u8>,
    pub w: usize,
    pub h: usize,
    pub w8: usize,
}
impl Field {
    pub fn new (w:usize, h:usize) -> Field {
        let w8 = ((w as f64) / 8_f64).ceil() as usize;
        let mut data: Vec<u8> = Vec::with_capacity(h*w8);
        data.resize(h*w8, 0);
        Field {
            data: data,
            w: w,
            h: h,
            w8: w8,
        }
    }
    pub fn get (&self, x:usize, y:usize) -> bool {
        let di = y*self.w8 + x/8;
        let d = self.data[di];
        ((d >> (x%8)) & 1) != 0
    }
    pub fn set (&mut self, x:usize, y:usize, v:bool) {
        let v8 = (v as u8) << (x%8);
        let di = y*self.w8 + x/8;
        let mut d = self.data[di];
        d &= !(1 << (x%8));
        d |= v8;
        self.data[di] = d;
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
}

pub struct Automata {
    seed_json: json::JsonValue,
    pub w: usize,
    pub h: usize,
    //these work like a double buffer
    pub field0: Field,
    field1: Field,
    fields_swapped: bool,
    //optimization
    table: Table,
    clb_field0: cl::memory::Buffer<u8>,
    clb_field1: cl::memory::Buffer<u8>,
    clb_table: cl::memory::Buffer<u8>,
    cl_command_queue: cl::command_queue::CommandQueue,
    clk_play: cl::kernel::Kernel,
}

#[derive(Debug, Clone)]
pub struct AutomataError {
    msg: String,
}
impl std::fmt::Display for AutomataError {
    fn fmt (&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for AutomataError {}

impl Automata {
    pub fn new (w:usize, h:usize, gpu_i:usize) -> Result<Automata, Box<dyn std::error::Error>> {
        // apply seed.json
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
        // table
        let mut table = Table::new(borns, survives);
        // (host) fields
        let (field0, field1) = (Field::new(w,h), Field::new(w,h));
        // integrate OpenCL
        let cl_context;
        let cl_command_queue;
        let clk_play;
        let clb_field0: cl::memory::Buffer<u8>;
        let clb_field1: cl::memory::Buffer<u8>;
        let mut clb_table: cl::memory::Buffer<u8>;
        {
            let mut devices = Vec::<cl::types::cl_device_id>::new();
            for platform in cl::platform::get_platforms().unwrap() {
                for device in platform.get_devices(cl::device::CL_DEVICE_TYPE_GPU).unwrap() {
                    devices.push(device);
                }
            }
            if gpu_i >= devices.len() {
                return Err(Box::new(AutomataError{
                    msg: "Cannot find a suitable OpenCL device for gpu_i.".to_string()
                }));
            }
            let device = cl::device::Device::new(devices[gpu_i]);
            cl_context = cl::context::Context::from_device(&device).unwrap();
            cl_command_queue = cl::command_queue::CommandQueue::create_with_properties(
                &cl_context,
                device.id(),
                0, //properties
                0 //queue_size
            ).unwrap();
            let program = cl::program::Program::create_and_build_from_source(
                &cl_context,
                &include_str!("kernels.cl"),
                "" //options
            ).unwrap();
            clk_play = cl::kernel::Kernel::new(
                cl::kernel::create_kernel(
                    program.get(),
                    &std::ffi::CString::new("play").unwrap()
                ).unwrap()
            );
            clb_field0 = cl::memory::Buffer::create(
                &cl_context,
                cl::memory::CL_MEM_READ_WRITE,
                h * field0.w8,
                std::ptr::null_mut()
            ).unwrap();
            clb_field1 = cl::memory::Buffer::create(
                &cl_context,
                cl::memory::CL_MEM_READ_WRITE,
                h * field0.w8,
                std::ptr::null_mut()
            ).unwrap();
            clb_table = cl::memory::Buffer::create(
                &cl_context,
                cl::memory::CL_MEM_READ_WRITE,
                table.values.len(),
                std::ptr::null_mut()
            ).unwrap();
            cl_command_queue.enqueue_write_buffer(
                &mut clb_table,
                1, //blocking_write
                0, //offset
                &table.values,
                &[] //event_wait_list
            ).unwrap();
            clk_play.set_arg(0, &(w as u32)).unwrap();
            clk_play.set_arg(1, &(h as u32)).unwrap();
            // 2 (source) set in loop
            // 3 (target) set in loop
            clk_play.set_arg(4, &clb_table.get()).unwrap();
        }
        Ok(Automata {
            w: w,
            h: h,
            field0: field0,
            field1: field1,
            fields_swapped: false,
            seed_json: seed_json,
            table: table,
            clb_field0: clb_field0,
            clb_field1: clb_field1,
            clb_table: clb_table,
            cl_command_queue: cl_command_queue,
            clk_play: clk_play,
        })
    }
    // Plays n rounds of Game Of Life or so.
    pub fn play (&mut self, n_rounds: usize) {
        let (source, target) =
            if self.fields_swapped {
                (&self.field1, &mut self.field0)
            } else {
                (&self.field0, &mut self.field1)
            };
        let w = source.w;
        let h = source.h;
        let w8 = source.w8;
        let table = &self.table;
        // prepare OpenCL
        let cl_command_queue = &self.cl_command_queue;
        let (mut clb_source, mut clb_target) =
            if self.fields_swapped {
                (&mut self.clb_field1, &mut self.clb_field0)
            } else {
                (&mut self.clb_field0, &mut self.clb_field1)
            };
        let mut clb_table = &mut self.clb_table;
        let clk_play = &self.clk_play;
        cl_command_queue.enqueue_write_buffer(
            &mut clb_source,
            1, //blocking_write
            0, //offset
            &source.data,
            &[] //event_wait_list
        ).unwrap();
        for _ in 0..n_rounds {
            if self.fields_swapped {
                clb_source = &mut self.clb_field1;
                clb_target = &mut self.clb_field0;
            } else {
                clb_source = &mut self.clb_field0;
                clb_target = &mut self.clb_field1;
            }
            clk_play.set_arg(2, &clb_source.get()).unwrap();
            clk_play.set_arg(3, &clb_target.get()).unwrap();
            // go, using OpenCL
            cl_command_queue.enqueue_nd_range_kernel(
                clk_play.get(),
                1, //work_dim; for: y=0, y=1, y=2, ... y=h-1
                [0].as_ptr(), //global_work_offsets
                [h].as_ptr(), //global_work_sizes
                [1].as_ptr(), //local_work_sizes
                &[] //event_wait_list
            ).unwrap();
            // clean up
            self.cl_command_queue.finish().unwrap();
            self.fields_swapped = !self.fields_swapped;
        }
        // declare source again, the "swapped" property might have changed
        let source =
            if self.fields_swapped {
                &mut self.field1
            } else {
                &mut self.field0
            };
        cl_command_queue.enqueue_read_buffer(
            &clb_target,
            1, //blocking_read
            0, //offset
            &mut source.data,
            &[] //event_wait_list
        ).unwrap();
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
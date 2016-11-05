use std::collections::BTreeMap;

#[macro_use] use log;
use bit_vec::BitVec;

use mem::GBMem;
use regset::GBRegisterSet;

pub struct GBCpu {
    sp  : u16, // stack pointer
    pc  : u16, // program counter
    mem : GBMem, // ram
    registers: GBRegisterSet,
    stop_flag: bool, // stop flag used by the stop instruction
    last_op_cycles: usize,
    instruction_cycle_map: BTreeMap<u16, usize>,
}

enum GBData {
    D16(u16),
    D8(u8),
    R8(i8),
    ADDRESS{ addr: usize, size: u16 },
    SP, PC,
    REG{name: String, inc: bool, dec: bool, addr: bool},
}

impl GBCpu {

    pub fn new(mem: GBMem) -> GBCpu {
        let mut cpu = GBCpu {
            sp: 0x0,
            pc: 0x0,
            mem: mem,
            registers: GBRegisterSet::new(vec!["AF", "BC", "DE", "HL"]),
            stop_flag: false,
            last_op_cycles: 0,
            instruction_cycle_map: BTreeMap::new(),
        };

        cpu.init_cycle_map();

        cpu
    }

    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn get_mem_ref<'a> (&'a self) -> &'a GBMem {
        &self.mem
    }

    pub fn get_mem_mut<'a> (&'a mut self) -> &'a mut GBMem {
        &mut self.mem
    }

    pub fn get_regset_ref<'a> (&'a self) -> &'a GBRegisterSet {
        &self.registers
    }

    pub fn get_last_op_cycles(&self) -> usize {
        self.last_op_cycles
    }

    pub fn step(&mut self) {
        self.exec_next_op();
    }

    pub fn is_interrupt_enabled(&self, ipos: usize) -> bool {
        let flags = BitVec::from_bytes(&[ self.mem.get(0xffff as usize) ]);
        flags.get(7-ipos).unwrap()
    }

    pub fn interrupt_enable(&mut self, ipos: usize, enable: bool) -> bool {
        let mut flags = BitVec::from_bytes(&[ self.mem.get(0xffff as usize) ]);
        let prev = flags.get(7-ipos).unwrap();

        flags.set(7-ipos, enable);
        self.mem.put(0xffff as usize, flags.to_bytes()[0] as u8);

        prev
    }

    pub fn set_interrupt_request(&mut self, ipos: usize, request: bool) -> bool {
        let mut requests = BitVec::from_bytes(&[ self.mem.get(0xff0f as usize) ]);
        let prev = requests.get(7-ipos).unwrap();

        requests.set(7-ipos, request);
        self.mem.put(0xff0f as usize, requests.to_bytes()[0] as u8);

        prev
    }

    fn stack_push(&mut self, value: u16) {

        // most significant part
        self.sp -= 1;
        self.mem.put(self.sp as usize, (value >> 8) as u8 );

        // least significant part
        self.sp -= 1;
        self.mem.put(self.sp as usize, value as u8 );

    }

    fn stack_pop(&mut self) -> u16 {
        // least significant part
        let mut value = self.mem.get(self.sp as usize) as u16;
        self.sp += 1;

        // most significant part
        value |= (self.mem.get(self.sp as usize) as u16) << 8;
        self.sp += 1;

        value
    }

    fn arg_parse(&self, arg_in: String) -> GBData {

        let mut arg = arg_in;

        let is_address = if arg.contains("(") {
            arg = arg.replace("(", "").replace(")", "");
            true
        } else {
            false
        };

        let has_inc = if arg.contains("+") {
            arg = arg.replace("+", "");
            true
        } else {
            false
        };

        let has_dec = if arg.contains("-") {
            arg = arg.replace("-", "");
            true
        } else {
            false
        };

        if arg == "SP" {
            GBData::SP
        } else if arg == "PC" {
            GBData::PC
        } else if arg == "r8" {
            let mut byte = self.mem.get(self.pc as usize) as i8;
            GBData::R8(byte)
        } else if arg == "d8" {
            let mut byte = self.mem.get(self.pc as usize) as u8;
            if is_address {
                byte += 0xFF00;
                GBData::ADDRESS{ addr: byte as usize, size: 1 }
            } else {
                GBData::D8(byte)
            }
        } else if arg == "d16" {
            let mut byte = self.mem.get(self.pc as usize) as u16;
            byte |= (self.mem.get((self.pc+1) as usize) as u16) << 8;
            if is_address {
                byte += 0xFF00;
                GBData::ADDRESS{ addr: byte as usize, size: 2 }
            } else {
                GBData::D16(byte)
            }
        } else if arg == "a8" {
            let mut byte = self.mem.get(self.pc as usize) as u16;
            if is_address {
                byte += 0xFF00;
                GBData::ADDRESS{ addr: byte as usize, size: 1 }
            } else {
                GBData::D8(byte as u8)
            }
        } else if arg == "a16" {
            let mut byte = self.mem.get(self.pc as usize) as u16;
            byte |= (self.mem.get((self.pc+1) as usize) as u16) << 8;
            if is_address {
                // byte += 0xFF00;
                GBData::ADDRESS{ addr: byte as usize, size: 2 }
            } else {
                GBData::D16(byte)
            }

        } else {
            // Register fallback
            GBData::REG{name: arg, inc: has_inc, dec: has_dec, addr: is_address}
        }

    }

    fn data_parse(&mut self, arg: &GBData) -> u16 {

        match arg {
            &GBData::R8(v) => {
                self.pc += 1;
                v as u16
            },
            &GBData::D8(v) => {
                self.pc += 1;
                v as u16
            },
            &GBData::D16(v) => {
                self.pc += 2;
                v
            },
            &GBData::REG{ref name, inc, dec, addr} => {
                let mut value = self.registers.get(&name);

                if addr {
                    value = self.mem.get(value as usize) as u16;
                }
                if inc {
                    self.registers.inc(&name);
                }
                if dec {
                    self.registers.dec(&name);
                }

                value
            },
            &GBData::ADDRESS{ addr, size } => {
                self.pc += size;
                self.mem.get(addr as usize) as u16
            },
            &GBData::SP => {
                self.sp
            },
            &GBData::PC => {
                self.pc
            },
        }

    }

    fn op_adc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_add<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_and<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_call<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("CALL {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        // 0	1	2	3
        // Z	N	H	C
        let flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);

        let condition = match args[0] {
            "NZ" => { // Z = 0
                !flags.get(0).unwrap()
            },
            "Z" => { // Z != 0
                flags.get(0).unwrap()
            },
            "NC" => { // C = 0
                !flags.get(3).unwrap()
            },
            "C" => { // C != 0
                flags.get(3).unwrap()
            },
            _ => true,
        };

        let destination = {
            let argp = self.arg_parse(args.last().unwrap().to_string());
            self.data_parse(&argp)
        };

        if condition {
            let v = self.pc;
            self.stack_push(v);
            self.pc = destination;
        }

    }

    fn op_ccf(&mut self) {

    }

    fn op_cpl(&mut self) {

    }

    fn op_cp<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if result is zero. (Set if A = n.) (0)
        // N - Set. (1)
        // H - Set if no borrow from bit 4. (2)
        // C - Set for no borrow. (Set if A < n.) (3)
        println!("CP {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut reg_a = self.registers.get(&"A".to_string());

        let argp = self.arg_parse(args[0].to_string());
        let data = self.data_parse(&argp);

        let mut flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);

        // set N
        flags.set(1, true);

        // Zero flag
        flags.set(0, reg_a == data);

        let half_carry = ( reg_a - (data & 0x0F) ) & 0xF0 != 0x00; // no borrow
        // Half carry flag
        flags.set(2, half_carry);

        // Half carry flag
        flags.set(3, reg_a > data);

        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);

    }

    fn op_daa(&mut self) {

    }

    fn op_dec<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if reselt is zero. (0)
        // N - Set. (1)
        // H - Set if no borrow from bit 4. (2)
        // C - Not affected. (3)
        println!("DEC {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);
        let change_flags = args[0].len() == 1;

        // reset N
        if change_flags {
            flags.set(1, true);
        }

        match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, inc, dec, addr} => {

                if addr {

                    let reg_value = self.registers.get(&name);
                    let mut mem_value = self.mem.get(reg_value as usize) as u16;

                    let half_carry = ( mem_value - 0x01 ) & 0xF0 != 0x00; // no borrow

                    mem_value -= 0x1;

                    if change_flags {
                        // Zero flag
                        flags.set(0, mem_value == 0x0);
                        // half carry
                        flags.set(2, half_carry);
                        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);
                    }

                    self.mem.put(reg_value as usize, mem_value as u8);

                } else {
                    let mut reg_value = self.registers.get(&name);
                    let half_carry = ( reg_value - 0x01 ) & 0xF0 != 0x00; // no borrow

                    reg_value -= 0x1;

                    self.registers.put(&name, reg_value);

                    if change_flags {
                        // Zero flag
                        flags.set(0, reg_value == 0x0);
                        // half carry
                        flags.set(2, half_carry);
                        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);
                    }
                }

            },
            GBData::SP => self.sp -= 0x1,
            _ => {},
        }

    }

    fn op_di(&mut self) {

    }

    fn op_ei(&mut self) {

    }

    fn op_halt(&mut self) {

    }

    fn op_inc<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if result is zero. (0)
        // N - Reset. (1)
        // H - Set if carry from bit 3. (2)
        // C - Not affected. (3)
        println!("INC {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);
        let change_flags = args[0].len() == 1;

        // reset N
        if change_flags {
            flags.set(1, false);
        }

        match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, inc, dec, addr} => {

                if addr {

                    let reg_value = self.registers.get(&name);
                    let mut mem_value = self.mem.get(reg_value as usize) as u16;

                    let half_carry = ( (mem_value & 0x0F) + 0x01 ) & 0x10 == 0x10;

                    mem_value += 0x1;

                    if change_flags {
                        // Zero flag
                        flags.set(0, mem_value == 0x0);
                        // half carry
                        flags.set(2, half_carry);
                        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);
                    }

                    self.mem.put(reg_value as usize, mem_value as u8);

                } else {
                    let mut reg_value = self.registers.get(&name);
                    let half_carry = ( (reg_value & 0x0F) + 0x01 ) & 0x10 == 0x10;

                    reg_value += 0x1;

                    self.registers.put(&name, reg_value);

                    if change_flags {
                        // Zero flag
                        flags.set(0, reg_value == 0x0);
                        // half carry
                        flags.set(2, half_carry);
                        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);
                    }
                }

            },
            GBData::SP => self.sp += 0x1,
            _ => {},
        }

    }

    fn op_jp<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_jr<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("JR {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        // 0	1	2	3
        // Z	N	H	C
        let flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);

        let condition = match args[0] {
            "NZ" => { // Z = 0
                !flags.get(0).unwrap()
            },
            "Z" => { // Z != 0
                flags.get(0).unwrap()
            },
            "NC" => { // C = 0
                !flags.get(3).unwrap()
            },
            "C" => { // C != 0
                flags.get(3).unwrap()
            },
            _ => true,
        };

        let destination = {
            let argp = self.arg_parse(args.last().unwrap().to_string());
            let data = self.data_parse(&argp) as i16; // r8
            ((self.pc as i16) + data) as u16
        };

        if condition {
            self.pc = destination;
        }

    }

    fn op_ldh<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("LDH {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        // match destination
        match self.arg_parse(args[0].to_string()) {
            GBData::ADDRESS{ addr, size } => {
                self.pc += size;

                let argp = self.arg_parse(args[1].to_string());
                let data = self.data_parse(&argp);
                self.mem.put(addr as usize, data as u8);
            },
            GBData::REG{name, inc, dec, addr} => {

                let argp = self.arg_parse(args[1].to_string());
                let data = self.data_parse(&argp);
                self.registers.put(&name, data);
            },
            _ => {},
        };

    }

    fn op_ld<'a> (&mut self, args: &'a Vec<&'a str>) {

        // TODO:0 check affected flags when op (0xF8) LD HL,SP+r8 id:0

        println!("LD {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;


        // match destination
        match self.arg_parse(args[0].to_string()) {
            GBData::SP => {

                let argp = self.arg_parse(args[1].to_string());
                self.sp = self.data_parse(&argp);

            },
            GBData::REG{name, inc, dec, addr} => {
                let argp = self.arg_parse(args[1].to_string());
                let data = self.data_parse(&argp);

                // NOTE: copy to an address? id:6
                if addr {
                    self.mem.put(self.registers.get(&name) as usize, data as u8);
                } else {
                    self.registers.put(&name, data);
                }

                if inc {
                    self.registers.inc(&name);
                }
                if dec {
                    self.registers.dec(&name);
                }

            },
            GBData::ADDRESS{ addr, size } => {
                // move the program counter over the address size that we just read
                self.pc += size;
                let argp = self.arg_parse(args[1].to_string());
                let mut data = self.data_parse(&argp);

                self.mem.put(addr, data as u8);
            },
            _ => {},
        };

    }

    fn op_none(&mut self) {

    }

    fn op_nop(&mut self) {

    }

    fn op_or<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_pop<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("POP {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, inc, dec, addr} => {

                let value = self.stack_pop();
                self.registers.put(&name, value);

            },
            _ => {},
        };

    }

    fn op_prefix<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("CB {}", args.join(","));
        self.pc += 1;

        self.exec_next_op_cb();

    }

    fn op_push<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("PUSH {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, inc, dec, addr} => {

                let value = self.registers.get(&name);
                self.stack_push(value);

            },
            _ => {},
        };

    }

    fn op_reti(&mut self) {

    }

    fn op_ret<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("RET {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        // 0	1	2	3
        // Z	N	H	C
        let flags = BitVec::from_bytes(&[ self.registers.get(&"F".to_string()) as u8 ]);

        let condition = {

            if args.len() == 0 {
                true
            } else {
                match args[0] {
                    "NZ" => { // Z = 0
                        !flags.get(0).unwrap()
                    },
                    "Z" => { // Z != 0
                        flags.get(0).unwrap()
                    },
                    "NC" => { // C = 0
                        !flags.get(3).unwrap()
                    },
                    "C" => { // C != 0
                        flags.get(3).unwrap()
                    },
                    _ => true,
                }
            }

        };

        let destination = self.stack_pop();

        if condition {
            self.pc = destination;
        }

    }

    fn op_rla(&mut self) {

        // Flags affected:
        // Z - Set if result is zero. (0)
        // N - Reset. (1)
        // H - Reset. (2)
        // C - Contains old bit 7 (0 in BitVec) data. (3)
        println!("RLA");
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut flags = self.registers.get(&"F".to_string()) as u8;
        let mut flags_bits = BitVec::from_bytes(&[flags]);

        // reset H
        flags_bits.set(2, false);
        // reset N
        flags_bits.set(1, false);

        let data = self.registers.get(&"A".to_string()) as u8;

        let mut bits = BitVec::from_bytes(&[data]);
        flags_bits.set( 3, bits.get(0).unwrap() );
        let data_rotated = data.rotate_left(1);

        // zero flag
        flags_bits.set(0, data_rotated == 0x0);

        self.registers.put(&"F".to_string(), flags_bits.to_bytes()[0] as u16);
        self.registers.put(&"A".to_string(), data_rotated as u16);

    }

    fn op_rlca(&mut self) {

    }

    fn op_rra(&mut self) {

    }

    fn op_rrca(&mut self) {

    }

    fn op_rst<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sbc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_scf(&mut self) {

    }

    fn op_stop<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sub<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_xor<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if result is zero.
        // N - Reset.
        // H - Reset.
        // C - Reset.
        println!("XOR {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut reg_a = self.registers.get(&"A".to_string());
        // match destination
        let data = match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, addr, ..} => {
                let value = self.registers.get(&name);
                if addr {
                    self.mem.get(value as usize) as u16
                } else {
                    value
                }
            },
            GBData::D8(v) => {
                self.pc += 1;
                v as u16
            },
            _ => 0x0,
        };

        // THE XOR
        reg_a ^= data;

        let mut flags = BitVec::from_elem(8, false);
        if (reg_a & 0x00FF) == 0 {
            flags.set(0, true); // set Z flag
        }

        self.registers.put(&"A".to_string(), reg_a);
        self.registers.put(&"F".to_string(), flags.to_bytes()[0] as u16);

    }

    fn op_bit<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if bit b of register r is 0. (0)
        // N - Reset. (1)
        // H - Set. (2)
        // C - Not affected.(3)
        println!("BIT {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        // first arg is an integer
        let bit = args[0].parse::<usize>().unwrap();
        let argp = self.arg_parse(args[1].to_string());
        let data = self.data_parse(&argp) as u8;
        let mut flags = self.registers.get(&"F".to_string()) as u8;

        let mut bits = BitVec::from_bytes(&[data]);

        let mut flags_bits = BitVec::from_bytes(&[flags]);
        // set H
        flags_bits.set(2, true);
        // reset N
        flags_bits.set(1, false);

        // set Z if bit b is 0
        flags_bits.set(0, !bits.get(7-bit).unwrap() );

        flags = flags_bits.to_bytes()[0];

        self.registers.put(&"F".to_string(), flags as u16);

    }

    fn op_res<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_rlc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_rl<'a> (&mut self, args: &'a Vec<&'a str>) {

        // Flags affected:
        // Z - Set if result is zero. (0)
        // N - Reset. (1)
        // H - Reset. (2)
        // C - Contains old bit 7 (0 in BitVec) data. (3)
        println!("RL {}", args.join(","));
        let cycles = self.instruction_cycle_map.get(&(self.mem.get(self.pc as usize) as u16)).unwrap().clone();
        self.last_op_cycles = cycles;
        self.pc += 1;

        let mut flags = self.registers.get(&"F".to_string()) as u8;
        let mut flags_bits = BitVec::from_bytes(&[flags]);

        // reset H
        flags_bits.set(2, false);
        // reset N
        flags_bits.set(1, false);

        match self.arg_parse(args[0].to_string()) {
            GBData::REG{name, addr, ..} => {
                let data = {
                    if addr {
                        self.mem.get(self.registers.get(&name) as usize)
                    } else {
                        self.registers.get(&name) as u8
                    }
                };

                let mut bits = BitVec::from_bytes(&[data]);
                flags_bits.set( 3, bits.get(0).unwrap() );
                let data_rotated = data.rotate_left(1);

                // zero flag
                flags_bits.set(0, data_rotated == 0x0);

                self.registers.put(&"F".to_string(), flags_bits.to_bytes()[0] as u16);
                self.registers.put(&name, data_rotated as u16);

            },
            _ => {},
        };

    }

    fn op_rrc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_rr<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_set<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sla<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sra<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_srl<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_swap<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn exec_next_op(&mut self) {

        let byte = self.mem.get(self.pc as usize);

        match byte & 0xFF {

            0x0 => self.op_nop(),
            0x1 => self.op_ld(&vec!("BC","d16")),
            0x2 => self.op_ld(&vec!("(BC)","A")),
            0x3 => self.op_inc(&vec!("BC")),
            0x4 => self.op_inc(&vec!("B")),
            0x5 => self.op_dec(&vec!("B")),
            0x6 => self.op_ld(&vec!("B","d8")),
            0x7 => self.op_rlca(),
            0x8 => self.op_ld(&vec!("(a16)","SP")),
            0x9 => self.op_add(&vec!("HL","BC")),
            0xa => self.op_ld(&vec!("A","(BC)")),
            0xb => self.op_dec(&vec!("BC")),
            0xc => self.op_inc(&vec!("C")),
            0xd => self.op_dec(&vec!("C")),
            0xe => self.op_ld(&vec!("C","d8")),
            0xf => self.op_rrca(),
            0x10 => self.op_stop(&vec!("0")),
            0x11 => self.op_ld(&vec!("DE","d16")),
            0x12 => self.op_ld(&vec!("(DE)","A")),
            0x13 => self.op_inc(&vec!("DE")),
            0x14 => self.op_inc(&vec!("D")),
            0x15 => self.op_dec(&vec!("D")),
            0x16 => self.op_ld(&vec!("D","d8")),
            0x17 => self.op_rla(),
            0x18 => self.op_jr(&vec!("r8")),
            0x19 => self.op_add(&vec!("HL","DE")),
            0x1a => self.op_ld(&vec!("A","(DE)")),
            0x1b => self.op_dec(&vec!("DE")),
            0x1c => self.op_inc(&vec!("E")),
            0x1d => self.op_dec(&vec!("E")),
            0x1e => self.op_ld(&vec!("E","d8")),
            0x1f => self.op_rra(),
            0x20 => self.op_jr(&vec!("NZ","r8")),
            0x21 => self.op_ld(&vec!("HL","d16")),
            0x22 => self.op_ld(&vec!("(HL+)","A")),
            0x23 => self.op_inc(&vec!("HL")),
            0x24 => self.op_inc(&vec!("H")),
            0x25 => self.op_dec(&vec!("H")),
            0x26 => self.op_ld(&vec!("H","d8")),
            0x27 => self.op_daa(),
            0x28 => self.op_jr(&vec!("Z","r8")),
            0x29 => self.op_add(&vec!("HL","HL")),
            0x2a => self.op_ld(&vec!("A","(HL+)")),
            0x2b => self.op_dec(&vec!("HL")),
            0x2c => self.op_inc(&vec!("L")),
            0x2d => self.op_dec(&vec!("L")),
            0x2e => self.op_ld(&vec!("L","d8")),
            0x2f => self.op_cpl(),
            0x30 => self.op_jr(&vec!("NC","r8")),
            0x31 => self.op_ld(&vec!("SP","d16")),
            0x32 => self.op_ld(&vec!("(HL-)","A")),
            0x33 => self.op_inc(&vec!("SP")),
            0x34 => self.op_inc(&vec!("(HL)")),
            0x35 => self.op_dec(&vec!("(HL)")),
            0x36 => self.op_ld(&vec!("(HL)","d8")),
            0x37 => self.op_scf(),
            0x38 => self.op_jr(&vec!("C","r8")),
            0x39 => self.op_add(&vec!("HL","SP")),
            0x3a => self.op_ld(&vec!("A","(HL-)")),
            0x3b => self.op_dec(&vec!("SP")),
            0x3c => self.op_inc(&vec!("A")),
            0x3d => self.op_dec(&vec!("A")),
            0x3e => self.op_ld(&vec!("A","d8")),
            0x3f => self.op_ccf(),
            0x40 => self.op_ld(&vec!("B","B")),
            0x41 => self.op_ld(&vec!("B","C")),
            0x42 => self.op_ld(&vec!("B","D")),
            0x43 => self.op_ld(&vec!("B","E")),
            0x44 => self.op_ld(&vec!("B","H")),
            0x45 => self.op_ld(&vec!("B","L")),
            0x46 => self.op_ld(&vec!("B","(HL)")),
            0x47 => self.op_ld(&vec!("B","A")),
            0x48 => self.op_ld(&vec!("C","B")),
            0x49 => self.op_ld(&vec!("C","C")),
            0x4a => self.op_ld(&vec!("C","D")),
            0x4b => self.op_ld(&vec!("C","E")),
            0x4c => self.op_ld(&vec!("C","H")),
            0x4d => self.op_ld(&vec!("C","L")),
            0x4e => self.op_ld(&vec!("C","(HL)")),
            0x4f => self.op_ld(&vec!("C","A")),
            0x50 => self.op_ld(&vec!("D","B")),
            0x51 => self.op_ld(&vec!("D","C")),
            0x52 => self.op_ld(&vec!("D","D")),
            0x53 => self.op_ld(&vec!("D","E")),
            0x54 => self.op_ld(&vec!("D","H")),
            0x55 => self.op_ld(&vec!("D","L")),
            0x56 => self.op_ld(&vec!("D","(HL)")),
            0x57 => self.op_ld(&vec!("D","A")),
            0x58 => self.op_ld(&vec!("E","B")),
            0x59 => self.op_ld(&vec!("E","C")),
            0x5a => self.op_ld(&vec!("E","D")),
            0x5b => self.op_ld(&vec!("E","E")),
            0x5c => self.op_ld(&vec!("E","H")),
            0x5d => self.op_ld(&vec!("E","L")),
            0x5e => self.op_ld(&vec!("E","(HL)")),
            0x5f => self.op_ld(&vec!("E","A")),
            0x60 => self.op_ld(&vec!("H","B")),
            0x61 => self.op_ld(&vec!("H","C")),
            0x62 => self.op_ld(&vec!("H","D")),
            0x63 => self.op_ld(&vec!("H","E")),
            0x64 => self.op_ld(&vec!("H","H")),
            0x65 => self.op_ld(&vec!("H","L")),
            0x66 => self.op_ld(&vec!("H","(HL)")),
            0x67 => self.op_ld(&vec!("H","A")),
            0x68 => self.op_ld(&vec!("L","B")),
            0x69 => self.op_ld(&vec!("L","C")),
            0x6a => self.op_ld(&vec!("L","D")),
            0x6b => self.op_ld(&vec!("L","E")),
            0x6c => self.op_ld(&vec!("L","H")),
            0x6d => self.op_ld(&vec!("L","L")),
            0x6e => self.op_ld(&vec!("L","(HL)")),
            0x6f => self.op_ld(&vec!("L","A")),
            0x70 => self.op_ld(&vec!("(HL)","B")),
            0x71 => self.op_ld(&vec!("(HL)","C")),
            0x72 => self.op_ld(&vec!("(HL)","D")),
            0x73 => self.op_ld(&vec!("(HL)","E")),
            0x74 => self.op_ld(&vec!("(HL)","H")),
            0x75 => self.op_ld(&vec!("(HL)","L")),
            0x76 => self.op_halt(),
            0x77 => self.op_ld(&vec!("(HL)","A")),
            0x78 => self.op_ld(&vec!("A","B")),
            0x79 => self.op_ld(&vec!("A","C")),
            0x7a => self.op_ld(&vec!("A","D")),
            0x7b => self.op_ld(&vec!("A","E")),
            0x7c => self.op_ld(&vec!("A","H")),
            0x7d => self.op_ld(&vec!("A","L")),
            0x7e => self.op_ld(&vec!("A","(HL)")),
            0x7f => self.op_ld(&vec!("A","A")),
            0x80 => self.op_add(&vec!("A","B")),
            0x81 => self.op_add(&vec!("A","C")),
            0x82 => self.op_add(&vec!("A","D")),
            0x83 => self.op_add(&vec!("A","E")),
            0x84 => self.op_add(&vec!("A","H")),
            0x85 => self.op_add(&vec!("A","L")),
            0x86 => self.op_add(&vec!("A","(HL)")),
            0x87 => self.op_add(&vec!("A","A")),
            0x88 => self.op_adc(&vec!("A","B")),
            0x89 => self.op_adc(&vec!("A","C")),
            0x8a => self.op_adc(&vec!("A","D")),
            0x8b => self.op_adc(&vec!("A","E")),
            0x8c => self.op_adc(&vec!("A","H")),
            0x8d => self.op_adc(&vec!("A","L")),
            0x8e => self.op_adc(&vec!("A","(HL)")),
            0x8f => self.op_adc(&vec!("A","A")),
            0x90 => self.op_sub(&vec!("B")),
            0x91 => self.op_sub(&vec!("C")),
            0x92 => self.op_sub(&vec!("D")),
            0x93 => self.op_sub(&vec!("E")),
            0x94 => self.op_sub(&vec!("H")),
            0x95 => self.op_sub(&vec!("L")),
            0x96 => self.op_sub(&vec!("(HL)")),
            0x97 => self.op_sub(&vec!("A")),
            0x98 => self.op_sbc(&vec!("A","B")),
            0x99 => self.op_sbc(&vec!("A","C")),
            0x9a => self.op_sbc(&vec!("A","D")),
            0x9b => self.op_sbc(&vec!("A","E")),
            0x9c => self.op_sbc(&vec!("A","H")),
            0x9d => self.op_sbc(&vec!("A","L")),
            0x9e => self.op_sbc(&vec!("A","(HL)")),
            0x9f => self.op_sbc(&vec!("A","A")),
            0xa0 => self.op_and(&vec!("B")),
            0xa1 => self.op_and(&vec!("C")),
            0xa2 => self.op_and(&vec!("D")),
            0xa3 => self.op_and(&vec!("E")),
            0xa4 => self.op_and(&vec!("H")),
            0xa5 => self.op_and(&vec!("L")),
            0xa6 => self.op_and(&vec!("(HL)")),
            0xa7 => self.op_and(&vec!("A")),
            0xa8 => self.op_xor(&vec!("B")),
            0xa9 => self.op_xor(&vec!("C")),
            0xaa => self.op_xor(&vec!("D")),
            0xab => self.op_xor(&vec!("E")),
            0xac => self.op_xor(&vec!("H")),
            0xad => self.op_xor(&vec!("L")),
            0xae => self.op_xor(&vec!("(HL)")),
            0xaf => self.op_xor(&vec!("A")),
            0xb0 => self.op_or(&vec!("B")),
            0xb1 => self.op_or(&vec!("C")),
            0xb2 => self.op_or(&vec!("D")),
            0xb3 => self.op_or(&vec!("E")),
            0xb4 => self.op_or(&vec!("H")),
            0xb5 => self.op_or(&vec!("L")),
            0xb6 => self.op_or(&vec!("(HL)")),
            0xb7 => self.op_or(&vec!("A")),
            0xb8 => self.op_cp(&vec!("B")),
            0xb9 => self.op_cp(&vec!("C")),
            0xba => self.op_cp(&vec!("D")),
            0xbb => self.op_cp(&vec!("E")),
            0xbc => self.op_cp(&vec!("H")),
            0xbd => self.op_cp(&vec!("L")),
            0xbe => self.op_cp(&vec!("(HL)")),
            0xbf => self.op_cp(&vec!("A")),
            0xc0 => self.op_ret(&vec!("NZ")),
            0xc1 => self.op_pop(&vec!("BC")),
            0xc2 => self.op_jp(&vec!("NZ","a16")),
            0xc3 => self.op_jp(&vec!("a16")),
            0xc4 => self.op_call(&vec!("NZ","a16")),
            0xc5 => self.op_push(&vec!("BC")),
            0xc6 => self.op_add(&vec!("A","d8")),
            0xc7 => self.op_rst(&vec!("00H")),
            0xc8 => self.op_ret(&vec!("Z")),
            0xc9 => self.op_ret(&vec!()),
            0xca => self.op_jp(&vec!("Z","a16")),
            0xcb => self.op_prefix(&vec!("CB")),
            0xcc => self.op_call(&vec!("Z","a16")),
            0xcd => self.op_call(&vec!("a16")),
            0xce => self.op_adc(&vec!("A","d8")),
            0xcf => self.op_rst(&vec!("08H")),
            0xd0 => self.op_ret(&vec!("NC")),
            0xd1 => self.op_pop(&vec!("DE")),
            0xd2 => self.op_jp(&vec!("NC","a16")),
            0xd3 => self.op_none(),
            0xd4 => self.op_call(&vec!("NC","a16")),
            0xd5 => self.op_push(&vec!("DE")),
            0xd6 => self.op_sub(&vec!("d8")),
            0xd7 => self.op_rst(&vec!("10H")),
            0xd8 => self.op_ret(&vec!("C")),
            0xd9 => self.op_reti(),
            0xda => self.op_jp(&vec!("C","a16")),
            0xdb => self.op_none(),
            0xdc => self.op_call(&vec!("C","a16")),
            0xdd => self.op_none(),
            0xde => self.op_sbc(&vec!("A","d8")),
            0xdf => self.op_rst(&vec!("18H")),
            0xe0 => self.op_ldh(&vec!("(a8)","A")),
            0xe1 => self.op_pop(&vec!("HL")),
            0xe2 => self.op_ld(&vec!("(C)","A")),
            0xe3 => self.op_none(),
            0xe4 => self.op_none(),
            0xe5 => self.op_push(&vec!("HL")),
            0xe6 => self.op_and(&vec!("d8")),
            0xe7 => self.op_rst(&vec!("20H")),
            0xe8 => self.op_add(&vec!("SP","r8")),
            0xe9 => self.op_jp(&vec!("(HL)")),
            0xea => self.op_ld(&vec!("(a16)","A")),
            0xeb => self.op_none(),
            0xec => self.op_none(),
            0xed => self.op_none(),
            0xee => self.op_xor(&vec!("d8")),
            0xef => self.op_rst(&vec!("28H")),
            0xf0 => self.op_ldh(&vec!("A","(a8)")),
            0xf1 => self.op_pop(&vec!("AF")),
            0xf2 => self.op_ld(&vec!("A","(C)")),
            0xf3 => self.op_di(),
            0xf4 => self.op_none(),
            0xf5 => self.op_push(&vec!("AF")),
            0xf6 => self.op_or(&vec!("d8")),
            0xf7 => self.op_rst(&vec!("30H")),
            0xf8 => self.op_ld(&vec!("HL","SP+r8")),
            0xf9 => self.op_ld(&vec!("SP","HL")),
            0xfa => self.op_ld(&vec!("A","(a16)")),
            0xfb => self.op_ei(),
            0xfc => self.op_none(),
            0xfd => self.op_none(),
            0xfe => self.op_cp(&vec!("d8")),
            0xff => self.op_rst(&vec!("38H")),
            _ => panic!("Unknown OP"),

        }

    }

    fn exec_next_op_cb(&mut self) {

        let byte = self.mem.get(self.pc as usize);

        match byte & 0xFF {
            0x0 => self.op_rlc(&vec!("B")),
            0x1 => self.op_rlc(&vec!("C")),
            0x2 => self.op_rlc(&vec!("D")),
            0x3 => self.op_rlc(&vec!("E")),
            0x4 => self.op_rlc(&vec!("H")),
            0x5 => self.op_rlc(&vec!("L")),
            0x6 => self.op_rlc(&vec!("(HL)")),
            0x7 => self.op_rlc(&vec!("A")),
            0x8 => self.op_rrc(&vec!("B")),
            0x9 => self.op_rrc(&vec!("C")),
            0xa => self.op_rrc(&vec!("D")),
            0xb => self.op_rrc(&vec!("E")),
            0xc => self.op_rrc(&vec!("H")),
            0xd => self.op_rrc(&vec!("L")),
            0xe => self.op_rrc(&vec!("(HL)")),
            0xf => self.op_rrc(&vec!("A")),
            0x10 => self.op_rl(&vec!("B")),
            0x11 => self.op_rl(&vec!("C")),
            0x12 => self.op_rl(&vec!("D")),
            0x13 => self.op_rl(&vec!("E")),
            0x14 => self.op_rl(&vec!("H")),
            0x15 => self.op_rl(&vec!("L")),
            0x16 => self.op_rl(&vec!("(HL)")),
            0x17 => self.op_rl(&vec!("A")),
            0x18 => self.op_rr(&vec!("B")),
            0x19 => self.op_rr(&vec!("C")),
            0x1a => self.op_rr(&vec!("D")),
            0x1b => self.op_rr(&vec!("E")),
            0x1c => self.op_rr(&vec!("H")),
            0x1d => self.op_rr(&vec!("L")),
            0x1e => self.op_rr(&vec!("(HL)")),
            0x1f => self.op_rr(&vec!("A")),
            0x20 => self.op_sla(&vec!("B")),
            0x21 => self.op_sla(&vec!("C")),
            0x22 => self.op_sla(&vec!("D")),
            0x23 => self.op_sla(&vec!("E")),
            0x24 => self.op_sla(&vec!("H")),
            0x25 => self.op_sla(&vec!("L")),
            0x26 => self.op_sla(&vec!("(HL)")),
            0x27 => self.op_sla(&vec!("A")),
            0x28 => self.op_sra(&vec!("B")),
            0x29 => self.op_sra(&vec!("C")),
            0x2a => self.op_sra(&vec!("D")),
            0x2b => self.op_sra(&vec!("E")),
            0x2c => self.op_sra(&vec!("H")),
            0x2d => self.op_sra(&vec!("L")),
            0x2e => self.op_sra(&vec!("(HL)")),
            0x2f => self.op_sra(&vec!("A")),
            0x30 => self.op_swap(&vec!("B")),
            0x31 => self.op_swap(&vec!("C")),
            0x32 => self.op_swap(&vec!("D")),
            0x33 => self.op_swap(&vec!("E")),
            0x34 => self.op_swap(&vec!("H")),
            0x35 => self.op_swap(&vec!("L")),
            0x36 => self.op_swap(&vec!("(HL)")),
            0x37 => self.op_swap(&vec!("A")),
            0x38 => self.op_srl(&vec!("B")),
            0x39 => self.op_srl(&vec!("C")),
            0x3a => self.op_srl(&vec!("D")),
            0x3b => self.op_srl(&vec!("E")),
            0x3c => self.op_srl(&vec!("H")),
            0x3d => self.op_srl(&vec!("L")),
            0x3e => self.op_srl(&vec!("(HL)")),
            0x3f => self.op_srl(&vec!("A")),
            0x40 => self.op_bit(&vec!("0","B")),
            0x41 => self.op_bit(&vec!("0","C")),
            0x42 => self.op_bit(&vec!("0","D")),
            0x43 => self.op_bit(&vec!("0","E")),
            0x44 => self.op_bit(&vec!("0","H")),
            0x45 => self.op_bit(&vec!("0","L")),
            0x46 => self.op_bit(&vec!("0","(HL)")),
            0x47 => self.op_bit(&vec!("0","A")),
            0x48 => self.op_bit(&vec!("1","B")),
            0x49 => self.op_bit(&vec!("1","C")),
            0x4a => self.op_bit(&vec!("1","D")),
            0x4b => self.op_bit(&vec!("1","E")),
            0x4c => self.op_bit(&vec!("1","H")),
            0x4d => self.op_bit(&vec!("1","L")),
            0x4e => self.op_bit(&vec!("1","(HL)")),
            0x4f => self.op_bit(&vec!("1","A")),
            0x50 => self.op_bit(&vec!("2","B")),
            0x51 => self.op_bit(&vec!("2","C")),
            0x52 => self.op_bit(&vec!("2","D")),
            0x53 => self.op_bit(&vec!("2","E")),
            0x54 => self.op_bit(&vec!("2","H")),
            0x55 => self.op_bit(&vec!("2","L")),
            0x56 => self.op_bit(&vec!("2","(HL)")),
            0x57 => self.op_bit(&vec!("2","A")),
            0x58 => self.op_bit(&vec!("3","B")),
            0x59 => self.op_bit(&vec!("3","C")),
            0x5a => self.op_bit(&vec!("3","D")),
            0x5b => self.op_bit(&vec!("3","E")),
            0x5c => self.op_bit(&vec!("3","H")),
            0x5d => self.op_bit(&vec!("3","L")),
            0x5e => self.op_bit(&vec!("3","(HL)")),
            0x5f => self.op_bit(&vec!("3","A")),
            0x60 => self.op_bit(&vec!("4","B")),
            0x61 => self.op_bit(&vec!("4","C")),
            0x62 => self.op_bit(&vec!("4","D")),
            0x63 => self.op_bit(&vec!("4","E")),
            0x64 => self.op_bit(&vec!("4","H")),
            0x65 => self.op_bit(&vec!("4","L")),
            0x66 => self.op_bit(&vec!("4","(HL)")),
            0x67 => self.op_bit(&vec!("4","A")),
            0x68 => self.op_bit(&vec!("5","B")),
            0x69 => self.op_bit(&vec!("5","C")),
            0x6a => self.op_bit(&vec!("5","D")),
            0x6b => self.op_bit(&vec!("5","E")),
            0x6c => self.op_bit(&vec!("5","H")),
            0x6d => self.op_bit(&vec!("5","L")),
            0x6e => self.op_bit(&vec!("5","(HL)")),
            0x6f => self.op_bit(&vec!("5","A")),
            0x70 => self.op_bit(&vec!("6","B")),
            0x71 => self.op_bit(&vec!("6","C")),
            0x72 => self.op_bit(&vec!("6","D")),
            0x73 => self.op_bit(&vec!("6","E")),
            0x74 => self.op_bit(&vec!("6","H")),
            0x75 => self.op_bit(&vec!("6","L")),
            0x76 => self.op_bit(&vec!("6","(HL)")),
            0x77 => self.op_bit(&vec!("6","A")),
            0x78 => self.op_bit(&vec!("7","B")),
            0x79 => self.op_bit(&vec!("7","C")),
            0x7a => self.op_bit(&vec!("7","D")),
            0x7b => self.op_bit(&vec!("7","E")),
            0x7c => self.op_bit(&vec!("7","H")),
            0x7d => self.op_bit(&vec!("7","L")),
            0x7e => self.op_bit(&vec!("7","(HL)")),
            0x7f => self.op_bit(&vec!("7","A")),
            0x80 => self.op_res(&vec!("0","B")),
            0x81 => self.op_res(&vec!("0","C")),
            0x82 => self.op_res(&vec!("0","D")),
            0x83 => self.op_res(&vec!("0","E")),
            0x84 => self.op_res(&vec!("0","H")),
            0x85 => self.op_res(&vec!("0","L")),
            0x86 => self.op_res(&vec!("0","(HL)")),
            0x87 => self.op_res(&vec!("0","A")),
            0x88 => self.op_res(&vec!("1","B")),
            0x89 => self.op_res(&vec!("1","C")),
            0x8a => self.op_res(&vec!("1","D")),
            0x8b => self.op_res(&vec!("1","E")),
            0x8c => self.op_res(&vec!("1","H")),
            0x8d => self.op_res(&vec!("1","L")),
            0x8e => self.op_res(&vec!("1","(HL)")),
            0x8f => self.op_res(&vec!("1","A")),
            0x90 => self.op_res(&vec!("2","B")),
            0x91 => self.op_res(&vec!("2","C")),
            0x92 => self.op_res(&vec!("2","D")),
            0x93 => self.op_res(&vec!("2","E")),
            0x94 => self.op_res(&vec!("2","H")),
            0x95 => self.op_res(&vec!("2","L")),
            0x96 => self.op_res(&vec!("2","(HL)")),
            0x97 => self.op_res(&vec!("2","A")),
            0x98 => self.op_res(&vec!("3","B")),
            0x99 => self.op_res(&vec!("3","C")),
            0x9a => self.op_res(&vec!("3","D")),
            0x9b => self.op_res(&vec!("3","E")),
            0x9c => self.op_res(&vec!("3","H")),
            0x9d => self.op_res(&vec!("3","L")),
            0x9e => self.op_res(&vec!("3","(HL)")),
            0x9f => self.op_res(&vec!("3","A")),
            0xa0 => self.op_res(&vec!("4","B")),
            0xa1 => self.op_res(&vec!("4","C")),
            0xa2 => self.op_res(&vec!("4","D")),
            0xa3 => self.op_res(&vec!("4","E")),
            0xa4 => self.op_res(&vec!("4","H")),
            0xa5 => self.op_res(&vec!("4","L")),
            0xa6 => self.op_res(&vec!("4","(HL)")),
            0xa7 => self.op_res(&vec!("4","A")),
            0xa8 => self.op_res(&vec!("5","B")),
            0xa9 => self.op_res(&vec!("5","C")),
            0xaa => self.op_res(&vec!("5","D")),
            0xab => self.op_res(&vec!("5","E")),
            0xac => self.op_res(&vec!("5","H")),
            0xad => self.op_res(&vec!("5","L")),
            0xae => self.op_res(&vec!("5","(HL)")),
            0xaf => self.op_res(&vec!("5","A")),
            0xb0 => self.op_res(&vec!("6","B")),
            0xb1 => self.op_res(&vec!("6","C")),
            0xb2 => self.op_res(&vec!("6","D")),
            0xb3 => self.op_res(&vec!("6","E")),
            0xb4 => self.op_res(&vec!("6","H")),
            0xb5 => self.op_res(&vec!("6","L")),
            0xb6 => self.op_res(&vec!("6","(HL)")),
            0xb7 => self.op_res(&vec!("6","A")),
            0xb8 => self.op_res(&vec!("7","B")),
            0xb9 => self.op_res(&vec!("7","C")),
            0xba => self.op_res(&vec!("7","D")),
            0xbb => self.op_res(&vec!("7","E")),
            0xbc => self.op_res(&vec!("7","H")),
            0xbd => self.op_res(&vec!("7","L")),
            0xbe => self.op_res(&vec!("7","(HL)")),
            0xbf => self.op_res(&vec!("7","A")),
            0xc0 => self.op_set(&vec!("0","B")),
            0xc1 => self.op_set(&vec!("0","C")),
            0xc2 => self.op_set(&vec!("0","D")),
            0xc3 => self.op_set(&vec!("0","E")),
            0xc4 => self.op_set(&vec!("0","H")),
            0xc5 => self.op_set(&vec!("0","L")),
            0xc6 => self.op_set(&vec!("0","(HL)")),
            0xc7 => self.op_set(&vec!("0","A")),
            0xc8 => self.op_set(&vec!("1","B")),
            0xc9 => self.op_set(&vec!("1","C")),
            0xca => self.op_set(&vec!("1","D")),
            0xcb => self.op_set(&vec!("1","E")),
            0xcc => self.op_set(&vec!("1","H")),
            0xcd => self.op_set(&vec!("1","L")),
            0xce => self.op_set(&vec!("1","(HL)")),
            0xcf => self.op_set(&vec!("1","A")),
            0xd0 => self.op_set(&vec!("2","B")),
            0xd1 => self.op_set(&vec!("2","C")),
            0xd2 => self.op_set(&vec!("2","D")),
            0xd3 => self.op_set(&vec!("2","E")),
            0xd4 => self.op_set(&vec!("2","H")),
            0xd5 => self.op_set(&vec!("2","L")),
            0xd6 => self.op_set(&vec!("2","(HL)")),
            0xd7 => self.op_set(&vec!("2","A")),
            0xd8 => self.op_set(&vec!("3","B")),
            0xd9 => self.op_set(&vec!("3","C")),
            0xda => self.op_set(&vec!("3","D")),
            0xdb => self.op_set(&vec!("3","E")),
            0xdc => self.op_set(&vec!("3","H")),
            0xdd => self.op_set(&vec!("3","L")),
            0xde => self.op_set(&vec!("3","(HL)")),
            0xdf => self.op_set(&vec!("3","A")),
            0xe0 => self.op_set(&vec!("4","B")),
            0xe1 => self.op_set(&vec!("4","C")),
            0xe2 => self.op_set(&vec!("4","D")),
            0xe3 => self.op_set(&vec!("4","E")),
            0xe4 => self.op_set(&vec!("4","H")),
            0xe5 => self.op_set(&vec!("4","L")),
            0xe6 => self.op_set(&vec!("4","(HL)")),
            0xe7 => self.op_set(&vec!("4","A")),
            0xe8 => self.op_set(&vec!("5","B")),
            0xe9 => self.op_set(&vec!("5","C")),
            0xea => self.op_set(&vec!("5","D")),
            0xeb => self.op_set(&vec!("5","E")),
            0xec => self.op_set(&vec!("5","H")),
            0xed => self.op_set(&vec!("5","L")),
            0xee => self.op_set(&vec!("5","(HL)")),
            0xef => self.op_set(&vec!("5","A")),
            0xf0 => self.op_set(&vec!("6","B")),
            0xf1 => self.op_set(&vec!("6","C")),
            0xf2 => self.op_set(&vec!("6","D")),
            0xf3 => self.op_set(&vec!("6","E")),
            0xf4 => self.op_set(&vec!("6","H")),
            0xf5 => self.op_set(&vec!("6","L")),
            0xf6 => self.op_set(&vec!("6","(HL)")),
            0xf7 => self.op_set(&vec!("6","A")),
            0xf8 => self.op_set(&vec!("7","B")),
            0xf9 => self.op_set(&vec!("7","C")),
            0xfa => self.op_set(&vec!("7","D")),
            0xfb => self.op_set(&vec!("7","E")),
            0xfc => self.op_set(&vec!("7","H")),
            0xfd => self.op_set(&vec!("7","L")),
            0xfe => self.op_set(&vec!("7","(HL)")),
            0xff => self.op_set(&vec!("7","A")),
            _ => panic!("Unknown OP"),
        }
    }

    fn init_cycle_map(&mut self) {
        self.instruction_cycle_map.insert(0x0, 4);
        self.instruction_cycle_map.insert(0x1, 12);
        self.instruction_cycle_map.insert(0x2, 8);
        self.instruction_cycle_map.insert(0x3, 8);
        self.instruction_cycle_map.insert(0x4, 4);
        self.instruction_cycle_map.insert(0x5, 4);
        self.instruction_cycle_map.insert(0x6, 8);
        self.instruction_cycle_map.insert(0x7, 4);
        self.instruction_cycle_map.insert(0x8, 20);
        self.instruction_cycle_map.insert(0x9, 8);
        self.instruction_cycle_map.insert(0xa, 8);
        self.instruction_cycle_map.insert(0xb, 8);
        self.instruction_cycle_map.insert(0xc, 4);
        self.instruction_cycle_map.insert(0xd, 4);
        self.instruction_cycle_map.insert(0xe, 8);
        self.instruction_cycle_map.insert(0xf, 4);
        self.instruction_cycle_map.insert(0x10, 4);
        self.instruction_cycle_map.insert(0x11, 12);
        self.instruction_cycle_map.insert(0x12, 8);
        self.instruction_cycle_map.insert(0x13, 8);
        self.instruction_cycle_map.insert(0x14, 4);
        self.instruction_cycle_map.insert(0x15, 4);
        self.instruction_cycle_map.insert(0x16, 8);
        self.instruction_cycle_map.insert(0x17, 4);
        self.instruction_cycle_map.insert(0x18, 12);
        self.instruction_cycle_map.insert(0x19, 8);
        self.instruction_cycle_map.insert(0x1a, 8);
        self.instruction_cycle_map.insert(0x1b, 8);
        self.instruction_cycle_map.insert(0x1c, 4);
        self.instruction_cycle_map.insert(0x1d, 4);
        self.instruction_cycle_map.insert(0x1e, 8);
        self.instruction_cycle_map.insert(0x1f, 4);
        self.instruction_cycle_map.insert(0x20, 8);
        self.instruction_cycle_map.insert(0x21, 12);
        self.instruction_cycle_map.insert(0x22, 8);
        self.instruction_cycle_map.insert(0x23, 8);
        self.instruction_cycle_map.insert(0x24, 4);
        self.instruction_cycle_map.insert(0x25, 4);
        self.instruction_cycle_map.insert(0x26, 8);
        self.instruction_cycle_map.insert(0x27, 4);
        self.instruction_cycle_map.insert(0x28, 8);
        self.instruction_cycle_map.insert(0x29, 8);
        self.instruction_cycle_map.insert(0x2a, 8);
        self.instruction_cycle_map.insert(0x2b, 8);
        self.instruction_cycle_map.insert(0x2c, 4);
        self.instruction_cycle_map.insert(0x2d, 4);
        self.instruction_cycle_map.insert(0x2e, 8);
        self.instruction_cycle_map.insert(0x2f, 4);
        self.instruction_cycle_map.insert(0x30, 8);
        self.instruction_cycle_map.insert(0x31, 12);
        self.instruction_cycle_map.insert(0x32, 8);
        self.instruction_cycle_map.insert(0x33, 8);
        self.instruction_cycle_map.insert(0x34, 12);
        self.instruction_cycle_map.insert(0x35, 12);
        self.instruction_cycle_map.insert(0x36, 12);
        self.instruction_cycle_map.insert(0x37, 4);
        self.instruction_cycle_map.insert(0x38, 8);
        self.instruction_cycle_map.insert(0x39, 8);
        self.instruction_cycle_map.insert(0x3a, 8);
        self.instruction_cycle_map.insert(0x3b, 8);
        self.instruction_cycle_map.insert(0x3c, 4);
        self.instruction_cycle_map.insert(0x3d, 4);
        self.instruction_cycle_map.insert(0x3e, 8);
        self.instruction_cycle_map.insert(0x3f, 4);
        self.instruction_cycle_map.insert(0x40, 4);
        self.instruction_cycle_map.insert(0x41, 4);
        self.instruction_cycle_map.insert(0x42, 4);
        self.instruction_cycle_map.insert(0x43, 4);
        self.instruction_cycle_map.insert(0x44, 4);
        self.instruction_cycle_map.insert(0x45, 4);
        self.instruction_cycle_map.insert(0x46, 8);
        self.instruction_cycle_map.insert(0x47, 4);
        self.instruction_cycle_map.insert(0x48, 4);
        self.instruction_cycle_map.insert(0x49, 4);
        self.instruction_cycle_map.insert(0x4a, 4);
        self.instruction_cycle_map.insert(0x4b, 4);
        self.instruction_cycle_map.insert(0x4c, 4);
        self.instruction_cycle_map.insert(0x4d, 4);
        self.instruction_cycle_map.insert(0x4e, 8);
        self.instruction_cycle_map.insert(0x4f, 4);
        self.instruction_cycle_map.insert(0x50, 4);
        self.instruction_cycle_map.insert(0x51, 4);
        self.instruction_cycle_map.insert(0x52, 4);
        self.instruction_cycle_map.insert(0x53, 4);
        self.instruction_cycle_map.insert(0x54, 4);
        self.instruction_cycle_map.insert(0x55, 4);
        self.instruction_cycle_map.insert(0x56, 8);
        self.instruction_cycle_map.insert(0x57, 4);
        self.instruction_cycle_map.insert(0x58, 4);
        self.instruction_cycle_map.insert(0x59, 4);
        self.instruction_cycle_map.insert(0x5a, 4);
        self.instruction_cycle_map.insert(0x5b, 4);
        self.instruction_cycle_map.insert(0x5c, 4);
        self.instruction_cycle_map.insert(0x5d, 4);
        self.instruction_cycle_map.insert(0x5e, 8);
        self.instruction_cycle_map.insert(0x5f, 4);
        self.instruction_cycle_map.insert(0x60, 4);
        self.instruction_cycle_map.insert(0x61, 4);
        self.instruction_cycle_map.insert(0x62, 4);
        self.instruction_cycle_map.insert(0x63, 4);
        self.instruction_cycle_map.insert(0x64, 4);
        self.instruction_cycle_map.insert(0x65, 4);
        self.instruction_cycle_map.insert(0x66, 8);
        self.instruction_cycle_map.insert(0x67, 4);
        self.instruction_cycle_map.insert(0x68, 4);
        self.instruction_cycle_map.insert(0x69, 4);
        self.instruction_cycle_map.insert(0x6a, 4);
        self.instruction_cycle_map.insert(0x6b, 4);
        self.instruction_cycle_map.insert(0x6c, 4);
        self.instruction_cycle_map.insert(0x6d, 4);
        self.instruction_cycle_map.insert(0x6e, 8);
        self.instruction_cycle_map.insert(0x6f, 4);
        self.instruction_cycle_map.insert(0x70, 8);
        self.instruction_cycle_map.insert(0x71, 8);
        self.instruction_cycle_map.insert(0x72, 8);
        self.instruction_cycle_map.insert(0x73, 8);
        self.instruction_cycle_map.insert(0x74, 8);
        self.instruction_cycle_map.insert(0x75, 8);
        self.instruction_cycle_map.insert(0x76, 4);
        self.instruction_cycle_map.insert(0x77, 8);
        self.instruction_cycle_map.insert(0x78, 4);
        self.instruction_cycle_map.insert(0x79, 4);
        self.instruction_cycle_map.insert(0x7a, 4);
        self.instruction_cycle_map.insert(0x7b, 4);
        self.instruction_cycle_map.insert(0x7c, 4);
        self.instruction_cycle_map.insert(0x7d, 4);
        self.instruction_cycle_map.insert(0x7e, 8);
        self.instruction_cycle_map.insert(0x7f, 4);
        self.instruction_cycle_map.insert(0x80, 4);
        self.instruction_cycle_map.insert(0x81, 4);
        self.instruction_cycle_map.insert(0x82, 4);
        self.instruction_cycle_map.insert(0x83, 4);
        self.instruction_cycle_map.insert(0x84, 4);
        self.instruction_cycle_map.insert(0x85, 4);
        self.instruction_cycle_map.insert(0x86, 8);
        self.instruction_cycle_map.insert(0x87, 4);
        self.instruction_cycle_map.insert(0x88, 4);
        self.instruction_cycle_map.insert(0x89, 4);
        self.instruction_cycle_map.insert(0x8a, 4);
        self.instruction_cycle_map.insert(0x8b, 4);
        self.instruction_cycle_map.insert(0x8c, 4);
        self.instruction_cycle_map.insert(0x8d, 4);
        self.instruction_cycle_map.insert(0x8e, 8);
        self.instruction_cycle_map.insert(0x8f, 4);
        self.instruction_cycle_map.insert(0x90, 4);
        self.instruction_cycle_map.insert(0x91, 4);
        self.instruction_cycle_map.insert(0x92, 4);
        self.instruction_cycle_map.insert(0x93, 4);
        self.instruction_cycle_map.insert(0x94, 4);
        self.instruction_cycle_map.insert(0x95, 4);
        self.instruction_cycle_map.insert(0x96, 8);
        self.instruction_cycle_map.insert(0x97, 4);
        self.instruction_cycle_map.insert(0x98, 4);
        self.instruction_cycle_map.insert(0x99, 4);
        self.instruction_cycle_map.insert(0x9a, 4);
        self.instruction_cycle_map.insert(0x9b, 4);
        self.instruction_cycle_map.insert(0x9c, 4);
        self.instruction_cycle_map.insert(0x9d, 4);
        self.instruction_cycle_map.insert(0x9e, 8);
        self.instruction_cycle_map.insert(0x9f, 4);
        self.instruction_cycle_map.insert(0xa0, 4);
        self.instruction_cycle_map.insert(0xa1, 4);
        self.instruction_cycle_map.insert(0xa2, 4);
        self.instruction_cycle_map.insert(0xa3, 4);
        self.instruction_cycle_map.insert(0xa4, 4);
        self.instruction_cycle_map.insert(0xa5, 4);
        self.instruction_cycle_map.insert(0xa6, 8);
        self.instruction_cycle_map.insert(0xa7, 4);
        self.instruction_cycle_map.insert(0xa8, 4);
        self.instruction_cycle_map.insert(0xa9, 4);
        self.instruction_cycle_map.insert(0xaa, 4);
        self.instruction_cycle_map.insert(0xab, 4);
        self.instruction_cycle_map.insert(0xac, 4);
        self.instruction_cycle_map.insert(0xad, 4);
        self.instruction_cycle_map.insert(0xae, 8);
        self.instruction_cycle_map.insert(0xaf, 4);
        self.instruction_cycle_map.insert(0xb0, 4);
        self.instruction_cycle_map.insert(0xb1, 4);
        self.instruction_cycle_map.insert(0xb2, 4);
        self.instruction_cycle_map.insert(0xb3, 4);
        self.instruction_cycle_map.insert(0xb4, 4);
        self.instruction_cycle_map.insert(0xb5, 4);
        self.instruction_cycle_map.insert(0xb6, 8);
        self.instruction_cycle_map.insert(0xb7, 4);
        self.instruction_cycle_map.insert(0xb8, 4);
        self.instruction_cycle_map.insert(0xb9, 4);
        self.instruction_cycle_map.insert(0xba, 4);
        self.instruction_cycle_map.insert(0xbb, 4);
        self.instruction_cycle_map.insert(0xbc, 4);
        self.instruction_cycle_map.insert(0xbd, 4);
        self.instruction_cycle_map.insert(0xbe, 8);
        self.instruction_cycle_map.insert(0xbf, 4);
        self.instruction_cycle_map.insert(0xc0, 8);
        self.instruction_cycle_map.insert(0xc1, 12);
        self.instruction_cycle_map.insert(0xc2, 12);
        self.instruction_cycle_map.insert(0xc3, 16);
        self.instruction_cycle_map.insert(0xc4, 12);
        self.instruction_cycle_map.insert(0xc5, 16);
        self.instruction_cycle_map.insert(0xc6, 8);
        self.instruction_cycle_map.insert(0xc7, 16);
        self.instruction_cycle_map.insert(0xc8, 8);
        self.instruction_cycle_map.insert(0xc9, 16);
        self.instruction_cycle_map.insert(0xca, 12);
        self.instruction_cycle_map.insert(0xcb, 4);
        self.instruction_cycle_map.insert(0xcc, 12);
        self.instruction_cycle_map.insert(0xcd, 24);
        self.instruction_cycle_map.insert(0xce, 8);
        self.instruction_cycle_map.insert(0xcf, 16);
        self.instruction_cycle_map.insert(0xd0, 8);
        self.instruction_cycle_map.insert(0xd1, 12);
        self.instruction_cycle_map.insert(0xd2, 12);
        self.instruction_cycle_map.insert(0xd3, 0);
        self.instruction_cycle_map.insert(0xd4, 12);
        self.instruction_cycle_map.insert(0xd5, 16);
        self.instruction_cycle_map.insert(0xd6, 8);
        self.instruction_cycle_map.insert(0xd7, 16);
        self.instruction_cycle_map.insert(0xd8, 8);
        self.instruction_cycle_map.insert(0xd9, 16);
        self.instruction_cycle_map.insert(0xda, 12);
        self.instruction_cycle_map.insert(0xdb, 0);
        self.instruction_cycle_map.insert(0xdc, 12);
        self.instruction_cycle_map.insert(0xdd, 0);
        self.instruction_cycle_map.insert(0xde, 8);
        self.instruction_cycle_map.insert(0xdf, 16);
        self.instruction_cycle_map.insert(0xe0, 12);
        self.instruction_cycle_map.insert(0xe1, 12);
        self.instruction_cycle_map.insert(0xe2, 8);
        self.instruction_cycle_map.insert(0xe3, 0);
        self.instruction_cycle_map.insert(0xe4, 0);
        self.instruction_cycle_map.insert(0xe5, 16);
        self.instruction_cycle_map.insert(0xe6, 8);
        self.instruction_cycle_map.insert(0xe7, 16);
        self.instruction_cycle_map.insert(0xe8, 16);
        self.instruction_cycle_map.insert(0xe9, 4);
        self.instruction_cycle_map.insert(0xea, 16);
        self.instruction_cycle_map.insert(0xeb, 0);
        self.instruction_cycle_map.insert(0xec, 0);
        self.instruction_cycle_map.insert(0xed, 0);
        self.instruction_cycle_map.insert(0xee, 8);
        self.instruction_cycle_map.insert(0xef, 16);
        self.instruction_cycle_map.insert(0xf0, 12);
        self.instruction_cycle_map.insert(0xf1, 12);
        self.instruction_cycle_map.insert(0xf2, 8);
        self.instruction_cycle_map.insert(0xf3, 4);
        self.instruction_cycle_map.insert(0xf4, 0);
        self.instruction_cycle_map.insert(0xf5, 16);
        self.instruction_cycle_map.insert(0xf6, 8);
        self.instruction_cycle_map.insert(0xf7, 16);
        self.instruction_cycle_map.insert(0xf8, 12);
        self.instruction_cycle_map.insert(0xf9, 8);
        self.instruction_cycle_map.insert(0xfa, 16);
        self.instruction_cycle_map.insert(0xfb, 4);
        self.instruction_cycle_map.insert(0xfc, 0);
        self.instruction_cycle_map.insert(0xfd, 0);
        self.instruction_cycle_map.insert(0xfe, 8);
        self.instruction_cycle_map.insert(0xff, 16);
        // cb
        self.instruction_cycle_map.insert(0xcb00, 8);
        self.instruction_cycle_map.insert(0xcb01, 8);
        self.instruction_cycle_map.insert(0xcb02, 8);
        self.instruction_cycle_map.insert(0xcb03, 8);
        self.instruction_cycle_map.insert(0xcb04, 8);
        self.instruction_cycle_map.insert(0xcb05, 8);
        self.instruction_cycle_map.insert(0xcb06, 16);
        self.instruction_cycle_map.insert(0xcb07, 8);
        self.instruction_cycle_map.insert(0xcb08, 8);
        self.instruction_cycle_map.insert(0xcb09, 8);
        self.instruction_cycle_map.insert(0xcb0a, 8);
        self.instruction_cycle_map.insert(0xcb0b, 8);
        self.instruction_cycle_map.insert(0xcb0c, 8);
        self.instruction_cycle_map.insert(0xcb0d, 8);
        self.instruction_cycle_map.insert(0xcb0e, 16);
        self.instruction_cycle_map.insert(0xcb0f, 8);
        self.instruction_cycle_map.insert(0xcb10, 8);
        self.instruction_cycle_map.insert(0xcb11, 8);
        self.instruction_cycle_map.insert(0xcb12, 8);
        self.instruction_cycle_map.insert(0xcb13, 8);
        self.instruction_cycle_map.insert(0xcb14, 8);
        self.instruction_cycle_map.insert(0xcb15, 8);
        self.instruction_cycle_map.insert(0xcb16, 16);
        self.instruction_cycle_map.insert(0xcb17, 8);
        self.instruction_cycle_map.insert(0xcb18, 8);
        self.instruction_cycle_map.insert(0xcb19, 8);
        self.instruction_cycle_map.insert(0xcb1a, 8);
        self.instruction_cycle_map.insert(0xcb1b, 8);
        self.instruction_cycle_map.insert(0xcb1c, 8);
        self.instruction_cycle_map.insert(0xcb1d, 8);
        self.instruction_cycle_map.insert(0xcb1e, 16);
        self.instruction_cycle_map.insert(0xcb1f, 8);
        self.instruction_cycle_map.insert(0xcb20, 8);
        self.instruction_cycle_map.insert(0xcb21, 8);
        self.instruction_cycle_map.insert(0xcb22, 8);
        self.instruction_cycle_map.insert(0xcb23, 8);
        self.instruction_cycle_map.insert(0xcb24, 8);
        self.instruction_cycle_map.insert(0xcb25, 8);
        self.instruction_cycle_map.insert(0xcb26, 16);
        self.instruction_cycle_map.insert(0xcb27, 8);
        self.instruction_cycle_map.insert(0xcb28, 8);
        self.instruction_cycle_map.insert(0xcb29, 8);
        self.instruction_cycle_map.insert(0xcb2a, 8);
        self.instruction_cycle_map.insert(0xcb2b, 8);
        self.instruction_cycle_map.insert(0xcb2c, 8);
        self.instruction_cycle_map.insert(0xcb2d, 8);
        self.instruction_cycle_map.insert(0xcb2e, 16);
        self.instruction_cycle_map.insert(0xcb2f, 8);
        self.instruction_cycle_map.insert(0xcb30, 8);
        self.instruction_cycle_map.insert(0xcb31, 8);
        self.instruction_cycle_map.insert(0xcb32, 8);
        self.instruction_cycle_map.insert(0xcb33, 8);
        self.instruction_cycle_map.insert(0xcb34, 8);
        self.instruction_cycle_map.insert(0xcb35, 8);
        self.instruction_cycle_map.insert(0xcb36, 16);
        self.instruction_cycle_map.insert(0xcb37, 8);
        self.instruction_cycle_map.insert(0xcb38, 8);
        self.instruction_cycle_map.insert(0xcb39, 8);
        self.instruction_cycle_map.insert(0xcb3a, 8);
        self.instruction_cycle_map.insert(0xcb3b, 8);
        self.instruction_cycle_map.insert(0xcb3c, 8);
        self.instruction_cycle_map.insert(0xcb3d, 8);
        self.instruction_cycle_map.insert(0xcb3e, 16);
        self.instruction_cycle_map.insert(0xcb3f, 8);
        self.instruction_cycle_map.insert(0xcb40, 8);
        self.instruction_cycle_map.insert(0xcb41, 8);
        self.instruction_cycle_map.insert(0xcb42, 8);
        self.instruction_cycle_map.insert(0xcb43, 8);
        self.instruction_cycle_map.insert(0xcb44, 8);
        self.instruction_cycle_map.insert(0xcb45, 8);
        self.instruction_cycle_map.insert(0xcb46, 16);
        self.instruction_cycle_map.insert(0xcb47, 8);
        self.instruction_cycle_map.insert(0xcb48, 8);
        self.instruction_cycle_map.insert(0xcb49, 8);
        self.instruction_cycle_map.insert(0xcb4a, 8);
        self.instruction_cycle_map.insert(0xcb4b, 8);
        self.instruction_cycle_map.insert(0xcb4c, 8);
        self.instruction_cycle_map.insert(0xcb4d, 8);
        self.instruction_cycle_map.insert(0xcb4e, 16);
        self.instruction_cycle_map.insert(0xcb4f, 8);
        self.instruction_cycle_map.insert(0xcb50, 8);
        self.instruction_cycle_map.insert(0xcb51, 8);
        self.instruction_cycle_map.insert(0xcb52, 8);
        self.instruction_cycle_map.insert(0xcb53, 8);
        self.instruction_cycle_map.insert(0xcb54, 8);
        self.instruction_cycle_map.insert(0xcb55, 8);
        self.instruction_cycle_map.insert(0xcb56, 16);
        self.instruction_cycle_map.insert(0xcb57, 8);
        self.instruction_cycle_map.insert(0xcb58, 8);
        self.instruction_cycle_map.insert(0xcb59, 8);
        self.instruction_cycle_map.insert(0xcb5a, 8);
        self.instruction_cycle_map.insert(0xcb5b, 8);
        self.instruction_cycle_map.insert(0xcb5c, 8);
        self.instruction_cycle_map.insert(0xcb5d, 8);
        self.instruction_cycle_map.insert(0xcb5e, 16);
        self.instruction_cycle_map.insert(0xcb5f, 8);
        self.instruction_cycle_map.insert(0xcb60, 8);
        self.instruction_cycle_map.insert(0xcb61, 8);
        self.instruction_cycle_map.insert(0xcb62, 8);
        self.instruction_cycle_map.insert(0xcb63, 8);
        self.instruction_cycle_map.insert(0xcb64, 8);
        self.instruction_cycle_map.insert(0xcb65, 8);
        self.instruction_cycle_map.insert(0xcb66, 16);
        self.instruction_cycle_map.insert(0xcb67, 8);
        self.instruction_cycle_map.insert(0xcb68, 8);
        self.instruction_cycle_map.insert(0xcb69, 8);
        self.instruction_cycle_map.insert(0xcb6a, 8);
        self.instruction_cycle_map.insert(0xcb6b, 8);
        self.instruction_cycle_map.insert(0xcb6c, 8);
        self.instruction_cycle_map.insert(0xcb6d, 8);
        self.instruction_cycle_map.insert(0xcb6e, 16);
        self.instruction_cycle_map.insert(0xcb6f, 8);
        self.instruction_cycle_map.insert(0xcb70, 8);
        self.instruction_cycle_map.insert(0xcb71, 8);
        self.instruction_cycle_map.insert(0xcb72, 8);
        self.instruction_cycle_map.insert(0xcb73, 8);
        self.instruction_cycle_map.insert(0xcb74, 8);
        self.instruction_cycle_map.insert(0xcb75, 8);
        self.instruction_cycle_map.insert(0xcb76, 16);
        self.instruction_cycle_map.insert(0xcb77, 8);
        self.instruction_cycle_map.insert(0xcb78, 8);
        self.instruction_cycle_map.insert(0xcb79, 8);
        self.instruction_cycle_map.insert(0xcb7a, 8);
        self.instruction_cycle_map.insert(0xcb7b, 8);
        self.instruction_cycle_map.insert(0xcb7c, 8);
        self.instruction_cycle_map.insert(0xcb7d, 8);
        self.instruction_cycle_map.insert(0xcb7e, 16);
        self.instruction_cycle_map.insert(0xcb7f, 8);
        self.instruction_cycle_map.insert(0xcb80, 8);
        self.instruction_cycle_map.insert(0xcb81, 8);
        self.instruction_cycle_map.insert(0xcb82, 8);
        self.instruction_cycle_map.insert(0xcb83, 8);
        self.instruction_cycle_map.insert(0xcb84, 8);
        self.instruction_cycle_map.insert(0xcb85, 8);
        self.instruction_cycle_map.insert(0xcb86, 16);
        self.instruction_cycle_map.insert(0xcb87, 8);
        self.instruction_cycle_map.insert(0xcb88, 8);
        self.instruction_cycle_map.insert(0xcb89, 8);
        self.instruction_cycle_map.insert(0xcb8a, 8);
        self.instruction_cycle_map.insert(0xcb8b, 8);
        self.instruction_cycle_map.insert(0xcb8c, 8);
        self.instruction_cycle_map.insert(0xcb8d, 8);
        self.instruction_cycle_map.insert(0xcb8e, 16);
        self.instruction_cycle_map.insert(0xcb8f, 8);
        self.instruction_cycle_map.insert(0xcb90, 8);
        self.instruction_cycle_map.insert(0xcb91, 8);
        self.instruction_cycle_map.insert(0xcb92, 8);
        self.instruction_cycle_map.insert(0xcb93, 8);
        self.instruction_cycle_map.insert(0xcb94, 8);
        self.instruction_cycle_map.insert(0xcb95, 8);
        self.instruction_cycle_map.insert(0xcb96, 16);
        self.instruction_cycle_map.insert(0xcb97, 8);
        self.instruction_cycle_map.insert(0xcb98, 8);
        self.instruction_cycle_map.insert(0xcb99, 8);
        self.instruction_cycle_map.insert(0xcb9a, 8);
        self.instruction_cycle_map.insert(0xcb9b, 8);
        self.instruction_cycle_map.insert(0xcb9c, 8);
        self.instruction_cycle_map.insert(0xcb9d, 8);
        self.instruction_cycle_map.insert(0xcb9e, 16);
        self.instruction_cycle_map.insert(0xcb9f, 8);
        self.instruction_cycle_map.insert(0xcba0, 8);
        self.instruction_cycle_map.insert(0xcba1, 8);
        self.instruction_cycle_map.insert(0xcba2, 8);
        self.instruction_cycle_map.insert(0xcba3, 8);
        self.instruction_cycle_map.insert(0xcba4, 8);
        self.instruction_cycle_map.insert(0xcba5, 8);
        self.instruction_cycle_map.insert(0xcba6, 16);
        self.instruction_cycle_map.insert(0xcba7, 8);
        self.instruction_cycle_map.insert(0xcba8, 8);
        self.instruction_cycle_map.insert(0xcba9, 8);
        self.instruction_cycle_map.insert(0xcbaa, 8);
        self.instruction_cycle_map.insert(0xcbab, 8);
        self.instruction_cycle_map.insert(0xcbac, 8);
        self.instruction_cycle_map.insert(0xcbad, 8);
        self.instruction_cycle_map.insert(0xcbae, 16);
        self.instruction_cycle_map.insert(0xcbaf, 8);
        self.instruction_cycle_map.insert(0xcbb0, 8);
        self.instruction_cycle_map.insert(0xcbb1, 8);
        self.instruction_cycle_map.insert(0xcbb2, 8);
        self.instruction_cycle_map.insert(0xcbb3, 8);
        self.instruction_cycle_map.insert(0xcbb4, 8);
        self.instruction_cycle_map.insert(0xcbb5, 8);
        self.instruction_cycle_map.insert(0xcbb6, 16);
        self.instruction_cycle_map.insert(0xcbb7, 8);
        self.instruction_cycle_map.insert(0xcbb8, 8);
        self.instruction_cycle_map.insert(0xcbb9, 8);
        self.instruction_cycle_map.insert(0xcbba, 8);
        self.instruction_cycle_map.insert(0xcbbb, 8);
        self.instruction_cycle_map.insert(0xcbbc, 8);
        self.instruction_cycle_map.insert(0xcbbd, 8);
        self.instruction_cycle_map.insert(0xcbbe, 16);
        self.instruction_cycle_map.insert(0xcbbf, 8);
        self.instruction_cycle_map.insert(0xcbc0, 8);
        self.instruction_cycle_map.insert(0xcbc1, 8);
        self.instruction_cycle_map.insert(0xcbc2, 8);
        self.instruction_cycle_map.insert(0xcbc3, 8);
        self.instruction_cycle_map.insert(0xcbc4, 8);
        self.instruction_cycle_map.insert(0xcbc5, 8);
        self.instruction_cycle_map.insert(0xcbc6, 16);
        self.instruction_cycle_map.insert(0xcbc7, 8);
        self.instruction_cycle_map.insert(0xcbc8, 8);
        self.instruction_cycle_map.insert(0xcbc9, 8);
        self.instruction_cycle_map.insert(0xcbca, 8);
        self.instruction_cycle_map.insert(0xcbcb, 8);
        self.instruction_cycle_map.insert(0xcbcc, 8);
        self.instruction_cycle_map.insert(0xcbcd, 8);
        self.instruction_cycle_map.insert(0xcbce, 16);
        self.instruction_cycle_map.insert(0xcbcf, 8);
        self.instruction_cycle_map.insert(0xcbd0, 8);
        self.instruction_cycle_map.insert(0xcbd1, 8);
        self.instruction_cycle_map.insert(0xcbd2, 8);
        self.instruction_cycle_map.insert(0xcbd3, 8);
        self.instruction_cycle_map.insert(0xcbd4, 8);
        self.instruction_cycle_map.insert(0xcbd5, 8);
        self.instruction_cycle_map.insert(0xcbd6, 16);
        self.instruction_cycle_map.insert(0xcbd7, 8);
        self.instruction_cycle_map.insert(0xcbd8, 8);
        self.instruction_cycle_map.insert(0xcbd9, 8);
        self.instruction_cycle_map.insert(0xcbda, 8);
        self.instruction_cycle_map.insert(0xcbdb, 8);
        self.instruction_cycle_map.insert(0xcbdc, 8);
        self.instruction_cycle_map.insert(0xcbdd, 8);
        self.instruction_cycle_map.insert(0xcbde, 16);
        self.instruction_cycle_map.insert(0xcbdf, 8);
        self.instruction_cycle_map.insert(0xcbe0, 8);
        self.instruction_cycle_map.insert(0xcbe1, 8);
        self.instruction_cycle_map.insert(0xcbe2, 8);
        self.instruction_cycle_map.insert(0xcbe3, 8);
        self.instruction_cycle_map.insert(0xcbe4, 8);
        self.instruction_cycle_map.insert(0xcbe5, 8);
        self.instruction_cycle_map.insert(0xcbe6, 16);
        self.instruction_cycle_map.insert(0xcbe7, 8);
        self.instruction_cycle_map.insert(0xcbe8, 8);
        self.instruction_cycle_map.insert(0xcbe9, 8);
        self.instruction_cycle_map.insert(0xcbea, 8);
        self.instruction_cycle_map.insert(0xcbeb, 8);
        self.instruction_cycle_map.insert(0xcbec, 8);
        self.instruction_cycle_map.insert(0xcbed, 8);
        self.instruction_cycle_map.insert(0xcbee, 16);
        self.instruction_cycle_map.insert(0xcbef, 8);
        self.instruction_cycle_map.insert(0xcbf0, 8);
        self.instruction_cycle_map.insert(0xcbf1, 8);
        self.instruction_cycle_map.insert(0xcbf2, 8);
        self.instruction_cycle_map.insert(0xcbf3, 8);
        self.instruction_cycle_map.insert(0xcbf4, 8);
        self.instruction_cycle_map.insert(0xcbf5, 8);
        self.instruction_cycle_map.insert(0xcbf6, 16);
        self.instruction_cycle_map.insert(0xcbf7, 8);
        self.instruction_cycle_map.insert(0xcbf8, 8);
        self.instruction_cycle_map.insert(0xcbf9, 8);
        self.instruction_cycle_map.insert(0xcbfa, 8);
        self.instruction_cycle_map.insert(0xcbfb, 8);
        self.instruction_cycle_map.insert(0xcbfc, 8);
        self.instruction_cycle_map.insert(0xcbfd, 8);
        self.instruction_cycle_map.insert(0xcbfe, 16);
        self.instruction_cycle_map.insert(0xcbff, 8);
    }

}

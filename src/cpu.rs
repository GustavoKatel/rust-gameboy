// mod mem;

use mem::GBMem;

pub struct GBCpu {
    sp  : u16, // stack pointer
    pc  : u16, // program counter
    mem : GBMem, // ram
    AF: u16, // reg 16bits: A (8bits) and flags (F) b'Zero-N(subtract)-HalfCarry-CarryFlag-0000
    BC: u16, // reg 16bits
    DE: u16, // reg 16bits
    HL: u16, // reg 16bits
}

impl GBCpu {

    pub fn new(mem: GBMem) -> GBCpu {
        GBCpu {
            sp: 0x0,
            pc: 0x0,
            mem: mem,
            AF: 0x0,
            BC: 0x0,
            DE: 0x0,
            HL: 0x0,
        }
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

    pub fn tick(&mut self) {

        self.step();

    }

    fn step(&mut self) {

        let byte = self.mem.get(self.pc as usize);

        match byte & 0xFF {
            0x0 => self.op_nop(&byte),

            // LD
            0x01 | 0x11 | 0x21 | 0x31 |
            0x02 | 0x12 | 0x22 | 0x32 |
            0x06 | 0x16 | 0x26 | 0x36 |
            0x0A | 0x1A | 0x2A | 0x3A |
            0x0E | 0x1E | 0x2E | 0x3E |
            0x40 ... 0x4F |
            0x50 ... 0x5F |
            0x60 ... 0x6F |
            0x70 ... 0x75 | 0x77 ... 0x7F |
            0xE2 | 0xF2 |
            0xEA | 0xFA |
            0x08 => self.op_ld(&byte),

            0x76 => self.op_halt(&byte),

            op => panic!("Unknown OP 0x{:04X}", op),
        }

    }

    fn op_nop(&self, _: &u8) {
    }

    fn op_halt(&self, _: &u8) {
    }

    fn get_next_8(&self) -> u8 {
        self.mem.get((self.pc+1) as usize) as u8
    }

    fn get_next_16(&self) -> u16 {
        self.mem.get((self.pc+1) as usize) as u16 +
            ((self.mem.get((self.pc+2) as usize) as u16) << 8)
    }

    fn op_ld(&mut self, byte: &u8) {

        match byte & 0xFF {
            0x01 => {
                self.BC = self.get_next_16();

                self.pc += 3;
            }

            0x11 => {
                self.DE = self.get_next_16();

                self.pc += 3;
            }

            0x21 => {
                self.HL = self.get_next_16();

                self.pc += 3;
            }

            0x31 => {
                self.sp = self.get_next_16();

                self.pc += 3;
            }

            0x02 => {
                self.mem.put(self.BC as usize, (self.AF >> 8) as u8 );

                self.pc += 1;
            }

            0x12 => {
                self.mem.put(self.DE as usize, (self.AF >> 8) as u8 );

                self.pc += 1;
            }

            0x22 => {
                self.mem.put(self.HL as usize, (self.AF >> 8) as u8 );

                self.HL += 1;

                self.pc += 1;
            }

            0x32 => {
                self.mem.put(self.HL as usize, (self.AF >> 8) as u8 );

                self.HL -= 1;

                self.pc += 1;
            }

            0x06 => {
                self.BC &= 0x00FF; // clear B, preserve C
                self.BC |= (self.get_next_8() as u16) << 8;

                self.pc += 2;
            }

            0x16 => {
                self.DE &= 0x00FF;
                self.DE |= (self.get_next_8() as u16) << 8;

                self.pc += 2;
            }

            0x26 => {
                self.HL &= 0x00FF;
                self.HL |= (self.get_next_8() as u16) << 8;

                self.pc += 2;
            }

            0x36 => {
                let next8 = self.get_next_8();
                self.mem.put(self.HL as usize, next8);

                self.pc += 2;
            }

            0x08 => {
                let next16 = self.get_next_16();
                self.mem.put(next16 as usize, self.sp as u8);

                self.pc += 3;
            }

            0x0A => {
                self.AF &= 0x00FF; // clear A and preserve F
                self.AF |= (self.mem.get(self.BC as usize) as u16) << 8;

                self.pc += 1;
            }

            0x1A => {
                self.AF &= 0x00FF; // clear A and preserve F
                self.AF |= (self.mem.get(self.DE as usize) as u16) << 8;

                self.pc += 1;
            }

            0x2A => {
                self.AF &= 0x00FF; // clear A and preserve F
                self.AF |= (self.mem.get(self.HL as usize) as u16) << 8;
                self.HL += 1;

                self.pc += 1;
            }

            0x3A => {
                self.AF &= 0x00FF; // clear A and preserve F
                self.AF |= (self.mem.get(self.HL as usize) as u16) << 8;
                self.HL -= 1;

                self.pc += 1;
            }

            0x0E => {
                self.BC &= 0xFF00; // clear C and preserve B
                self.BC |= self.get_next_8() as u16;

                self.pc += 2;
            }

            0x1E => {
                self.DE &= 0xFF00; // clear E and preserve D
                self.DE |= self.get_next_8() as u16;

                self.pc += 2;
            }

            0x2E => {
                self.HL &= 0xFF00; // clear L and preserve H
                self.HL |= self.get_next_8() as u16;

                self.pc += 2;
            }

            0x3E => {
                self.AF &= 0x00FF; // clear A and preserve B
                self.AF |= (self.get_next_8() as u16) << 8;

                self.pc += 2;
            }

            // this pattern match the case LD 8bits,8bits
            0x40 ... 0x4F |
            0x50 ... 0x5F |
            0x60 ... 0x6F |
            0x70 ... 0x7F => {

                let o8 : u8 = match byte & 0x0F {
                    0x0 | 0x8 => (self.BC >> 8) as u8, // B
                    0x1 | 0x9 => self.BC as u8, // C
                    0x2 | 0xA => (self.DE >> 8) as u8, // D
                    0x3 | 0xB => self.DE as u8, // E
                    0x4 | 0xC => (self.HL >> 8) as u8, // H
                    0x5 | 0xD => self.HL as u8, // L
                    0x6 | 0xE => self.mem.get(self.HL as usize), // (HL)
                    0x7 | 0xF => (self.AF >> 8) as u8, // A
                    _ => 0x0,
                };

                match (byte & 0xF0) >> 4 {
                    0x4 => {
                        if byte & 0x0F < 0x8 {

                            self.BC &= 0x00FF;
                            self.BC |= (o8 as u16) << 8;

                        } else {
                            self.BC &= 0xFF00;
                            self.BC |= o8 as u16;
                        }
                    },
                    0x5 => {
                        if byte & 0x0F < 0x8 {

                            self.DE &= 0x00FF;
                            self.DE |= (o8 as u16) << 8;

                        } else {
                            self.DE &= 0xFF00;
                            self.DE |= o8 as u16;
                        }
                    },

                    0x6 => {
                        if byte & 0x0F < 0x8 {

                            self.HL &= 0x00FF;
                            self.HL |= (o8 as u16) << 8;

                        } else {
                            self.HL &= 0xFF00;
                            self.HL |= o8 as u16;
                        }
                    },

                    0x7 => {
                        if byte & 0x0F < 0x8 {

                            self.mem.put(self.HL as usize, o8);

                        } else {
                            self.AF &= 0x00FF;
                            self.AF |= (o8 as u16) << 8;
                        }
                    },

                    _ => {},

                };

                self.pc += 1;

            },

            // LDH (a8),A
            // direct address or use the offset 0xFF00?
            0xE0 => {
                let b8 = self.get_next_8();
                self.mem.put(b8 as usize, (self.AF >> 8) as u8 );
                self.pc += 2;
            },

            // LDH A,(a8)
            // direct address or use the offset 0xFF00?
            0xF0 => {
                self.AF &= 0x00FF;
                let b8 = self.get_next_8();
                self.AF |= (self.mem.get(b8 as usize) as u16) << 8;
                self.pc += 2;
            },

            0xE2 => {
                self.mem.put(( self.BC as u8 ) as usize, (self.AF >> 8) as u8 );
                self.pc += 2;
            },

            0xF2 => {
                self.AF &= 0x00FF;
                self.AF |= (self.mem.get((self.BC as u8) as usize) as u16) << 8;
                self.pc += 2;
            },

            0xEA => {
                let b16 = self.get_next_16();
                self.mem.put( b16 as usize, (self.AF >> 8) as u8 );
                self.pc += 3;
            },

            0xFA => {
                self.AF &= 0x00FF;
                let b16 = self.get_next_16();
                self.AF |= (self.mem.get( b16 as usize) as u16) << 8;
                self.pc += 3;
            },

            _ => panic!("Unknown LD variant"),
        }

    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use mem::GBMem;

    #[test]
    fn op_ld_0x01() {

        let mut mem = GBMem::new();
        mem.put(0, 0x01); // op
        mem.put(1, 1); // b7-0
        mem.put(2, 0); // b15-8

        let mut cpu = GBCpu::new(mem);
        // sp = 0x0
        // pc = 0x0

        cpu.op_ld(&(0x01 as u8));

        assert_eq!((cpu.BC, cpu.get_pc()), (1, 3));
    }

    #[test]
    fn op_ld_0x11() {

        let mut mem = GBMem::new();
        mem.put(0, 0x11); // op
        mem.put(1, 1); // b7-0
        mem.put(2, 0); // b15-8

        let mut cpu = GBCpu::new(mem);
        // sp = 0x0
        // pc = 0x0

        cpu.op_ld(&(0x11 as u8));

        assert_eq!((cpu.DE, cpu.get_pc()), (1, 3));
    }

    #[test]
    fn op_ld_0x21() {

        let mut mem = GBMem::new();
        mem.put(0, 0x21); // op
        mem.put(1, 1); // b7-0
        mem.put(2, 0); // b15-8

        let mut cpu = GBCpu::new(mem);
        // sp = 0x0
        // pc = 0x0

        cpu.op_ld(&(0x21 as u8));

        assert_eq!((cpu.HL, cpu.get_pc()), (1, 3));
    }

    #[test]
    fn op_ld_0x31() {

        let mut mem = GBMem::new();
        mem.put(0, 0x31); // op
        mem.put(1, 1); // b7-0
        mem.put(2, 0); // b15-8

        let mut cpu = GBCpu::new(mem);
        // sp = 0x0
        // pc = 0x0

        cpu.op_ld(&(0x31 as u8));

        assert_eq!((cpu.get_sp(), cpu.get_pc()), (1, 3));
    }

    #[test]
    fn op_ld_0x02() {

        let mut mem = GBMem::new();
        mem.put(0, 0x02); // op

        let mut cpu = GBCpu::new(mem);
        cpu.AF = 300 as u16;
        cpu.BC = 0;

        cpu.op_ld(&(0x02 as u8));

        assert_eq!(
            ( cpu.pc, cpu.get_mem_ref().get(cpu.BC as usize), ),
            ( 1, (cpu.AF >> 8) as u8, )
        );
    }

    #[test]
    fn op_ld_0x12() {

        let mut mem = GBMem::new();
        mem.put(0, 0x12); // op

        let mut cpu = GBCpu::new(mem);
        cpu.AF = 300 as u16;
        cpu.DE = 0;

        cpu.op_ld(&(0x12 as u8));

        assert_eq!(
            ( cpu.pc, cpu.get_mem_ref().get(cpu.DE as usize), ),
            ( 1, (cpu.AF >> 8) as u8, )
        );
    }

    #[test]
    fn op_ld_0x22() {

        let mut mem = GBMem::new();
        mem.put(0, 0x22); // op

        let mut cpu = GBCpu::new(mem);
        cpu.AF = 300 as u16;
        cpu.HL = 0;

        cpu.op_ld(&(0x22 as u8));

        assert_eq!(
            ( cpu.pc, cpu.get_mem_ref().get((cpu.HL-1) as usize), ),
            ( 1, (cpu.AF >> 8) as u8, )
        );
    }

    #[test]
    fn op_ld_0x32() {

        let mut mem = GBMem::new();
        mem.put(0, 0x32); // op

        let mut cpu = GBCpu::new(mem);
        cpu.AF = 300 as u16;
        cpu.HL = 500; // random memory addr

        cpu.op_ld(&(0x32 as u8));

        assert_eq!(
            ( cpu.pc, cpu.get_mem_ref().get((cpu.HL+1) as usize), ),
            ( 1, (cpu.AF >> 8) as u8, )
        );
    }

    #[test]
    fn op_ld_0x26() {

        let mut mem = GBMem::new();
        mem.put(0, 0x26); // op
        mem.put(1, 0x01); // op

        let mut cpu = GBCpu::new(mem);
        cpu.HL = 500;

        cpu.op_ld(&(0x26 as u8));

        assert_eq!(
            ( cpu.pc, (cpu.HL >> 8) as u8, ),
            ( 2, 1, )
        );
    }

    #[test]
    fn op_ld_0x36() {

        let mut mem = GBMem::new();
        mem.put(0, 0x36); // op
        mem.put(1, 0x01); // op

        let mut cpu = GBCpu::new(mem);
        cpu.HL = 500; // random memory addr

        cpu.op_ld(&(0x36 as u8));

        assert_eq!(
            ( cpu.pc, cpu.get_mem_ref().get((cpu.HL) as usize), ),
            ( 2, 1, )
        );
    }

}

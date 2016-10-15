mod cpu;
mod mem;

use std::io::prelude::*;
use std::fs::File;

use cpu::GBCpu;
use mem::GBMem;

fn main() {

    let mut mem = GBMem::new();

    {
        let rom_file = File::open("etc/boot.bin").unwrap();

        for (pos, byte) in rom_file.bytes().enumerate() {

            mem.put(pos, byte.unwrap());

        }
    }

    let mut cpu = GBCpu::new(mem);

    cpu.tick();

    println!("0x{:02X}", cpu.get_sp());
    println!("0x{:02X}", cpu.get_mem_ref().get(0x07 as usize));


}

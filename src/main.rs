mod cpu;
mod mem;

use std::io::prelude::*;
use std::fs::File;
use std::{thread, time};

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

    let timeout = time::Duration::from_millis(16);

    'main_loop: loop {
        cpu.tick();

        println!("SP: 0x{:04X}", cpu.get_sp());
        println!("PC: 0x{:04X}", cpu.get_pc());

        thread::sleep(timeout);

    }

}

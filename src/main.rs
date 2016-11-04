#[macro_use] extern crate log;
extern crate  bit_vec;
extern crate sdl2;

mod regset;
mod cpu;
mod mem;
mod gpu;
mod sdl_display;

use std::io::prelude::*;
use std::fs::File;
use std::io;

use cpu::GBCpu;
use mem::GBMem;
use sdl_display::{SDLDisplay, SDLDisplayEvent};
use gpu::GBGpu;

fn main() {

    let mut mem = GBMem::new();

    {
        let rom_file = File::open("etc/boot.bin").unwrap();

        for (pos, byte) in rom_file.bytes().enumerate() {

            mem.put(pos, byte.unwrap());

        }
    }

    let mut cpu = GBCpu::new(mem);

    let mut display = SDLDisplay::new(600, 800, "rust-gameboy".to_string());

    let mut gpu = GBGpu::new();

    let mut count = 0;

    'main_loop: loop {
    // for _ in 0..24577+5+12+39+39 {
    // for _ in 0..28817 {

        println!("Cycles: {}", cpu.get_last_op_cycles());
        println!("SP: 0x{:04X}", cpu.get_sp());
        println!("PC: 0x{:04X}", cpu.get_pc());
        println!("OP: 0x{:04X}", cpu.get_mem_ref().get(cpu.get_pc() as usize));
        println!("{:?}", cpu.get_regset_ref());

        cpu.step();
        gpu.step(&mut cpu, &mut display);
        display.step();

        println!("-------------", );

        count += 1;
        if cpu.get_pc() == 0x40 {
            break 'main_loop;
        }

        for event in display.get_events().iter() {
            match event {
                &SDLDisplayEvent::Quit => break 'main_loop,
            }
        }

    }


    'read_loop: loop {

        println!("Count: {:?}", count);
        println!("Cycles: {}", cpu.get_last_op_cycles());
        println!("SP: 0x{:04X}", cpu.get_sp());
        println!("PC: 0x{:04X}", cpu.get_pc());
        println!("OP: 0x{:04X}", cpu.get_mem_ref().get(cpu.get_pc() as usize));
        println!("{:?}", cpu.get_regset_ref());
        cpu.get_mem_ref().dump("tmp/mem.bin");

        cpu.step();
        gpu.step(&mut cpu, &mut display);

        println!("-------------", );

        count += 1;

        let stdin = io::stdin();
        let line = stdin.lock().lines().next().unwrap().unwrap();

        if line == "q" {
            break 'read_loop;
        }

        for event in display.get_events().iter() {
            match event {
                &SDLDisplayEvent::Quit => break 'read_loop,
            }
        }

    }

    println!("SP: 0x{:04X}", cpu.get_sp());
    println!("PC: 0x{:04X}", cpu.get_pc());
    println!("OP: 0x{:04X}", cpu.get_mem_ref().get(cpu.get_pc() as usize));
    println!("{:?}", cpu.get_regset_ref());

}

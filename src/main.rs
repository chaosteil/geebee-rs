mod bytes;
mod cart;
mod cpu;
mod memory;
mod timer;

use clap::{App, Arg};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("geebee-rs")
        .version("0.1.0")
        .about("barebones gameboy emulator")
        .arg(
            Arg::with_name("rom")
                .short("r")
                .long("rom")
                .takes_value(true)
                .required(true)
                .help("path to the gameboy rom"),
        )
        .arg(
            Arg::with_name("bootrom")
                .short("b")
                .long("bootrom")
                .takes_value(true)
                .help("path to the bootrom"),
        )
        .get_matches();

    let cart = cart::Cartridge::with_path(Path::new(matches.value_of("rom").unwrap()))?;
    println!("cart: {}", cart.title());
    let mut memory = memory::Memory::new(cart);
    if let Some(bootrom) = matches.value_of("bootrom") {
        memory = memory.with_bootrom_path(Path::new(bootrom))?;
    }
    let mut cpu = cpu::CPU::new(memory);
    loop {
        cpu.step();
        let serial = cpu.serial();
        if serial.len() != 0 {
            println!("{}", std::str::from_utf8(serial)?);
        }
    }
}

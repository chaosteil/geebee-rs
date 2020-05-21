mod bytes;
mod cart;
mod cpu;
mod joypad;
mod lcd;
mod mbc;
mod memory;
mod timer;
mod ui;

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
        .arg(
            Arg::with_name("serial-stdout")
                .short("s")
                .long("serial-stdout")
                .takes_value(false)
                .help("print out anything on the serial device into stdout"),
        )
        .get_matches();

    let cart = cart::Cartridge::new().with_path(Path::new(matches.value_of("rom").unwrap()))?;

    let mut memory = memory::Memory::with_cartridge(cart);
    if let Some(bootrom) = matches.value_of("bootrom") {
        memory = memory.with_bootrom_path(Path::new(bootrom))?;
    }

    let lcd = lcd::LCD::new(memory.gb());
    let mut cpu = cpu::CPU::new(memory, lcd);

    if matches.is_present("serial-stdout") {
        cpu.show_serial_output(true);
    }

    ui::launch(cpu)?;

    Ok(())
}

use crate::cart::{Cartridge, Controller};
use std::{fs::File, io::Read, path::Path};

pub struct Memory {
    cart: Cartridge,
    work_ram: [u8; 0x2000],
    work_ram_banks: Vec<Vec<u8>>,
    external_ram: [u8; 0x2000],
    high_ram: [u8; 0x7f],
    video: [u8; 0x4000],
    oam: [u8; 0xa0],
    bootrom: Vec<u8>,

    booting: bool,
    rom_ram_mode: u8,
    rom_bank: u8,
    work_ram_bank: u8,
    external_ram_bank: u8,
    video_bank: u8,
    external_ram_enabled: bool,
}

impl Memory {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            work_ram: [0; 0x2000],
            work_ram_banks: Vec::new(),
            external_ram: [0; 0x2000],
            high_ram: [0; 0x7f],
            video: [0; 0x4000],
            oam: [0; 0xa0],
            bootrom: Vec::new(),

            booting: false,
            rom_ram_mode: 0,
            rom_bank: 0,
            work_ram_bank: 0,
            external_ram_bank: 0,
            video_bank: 0,
            external_ram_enabled: false,
        }
    }
    pub fn with_bootrom(mut self, data: Vec<u8>) -> Self {
        self.booting = true;
        self.bootrom = data;
        self
    }

    pub fn with_bootrom_path(self, rom: &Path) -> Result<Self, std::io::Error> {
        let mut data = Vec::<u8>::new();
        let mut file = File::open(rom)?;
        file.read_to_end(&mut data)?;
        Ok(self.with_bootrom(data))
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x0100 => {
                if self.booting {
                    self.bootrom[address as usize]
                } else {
                    self.cart.data()[address as usize]
                }
            }
            0x0101..=0x3fff => self.cart.data()[address as usize],
            0x4000..=0x7fff => {
                let address = (0x4000 * self.rom_bank as u16) + (address - 0x4000);
                self.cart.data()[address as usize]
            }
            0x8000..=0x9fff => {
                let address = (0x8000 * self.video_bank as u16) + (address - 0x8000);
                self.video[address as usize]
            }
            0xa000..=0xbfff => {
                if self.external_ram_enabled {
                    let address = (0x8000 * self.external_ram_bank as u16) + (address - 0x8000);
                    self.external_ram[address as usize]
                } else {
                    0
                }
            }
            0xc000..=0xcfff => self.work_ram[address as usize - 0xc000],
            0xd000..=0xdfff => {
                if self.work_ram_bank == 0 {
                    self.work_ram[address as usize - 0xc000]
                } else {
                    self.work_ram_banks[self.work_ram_bank as usize - 1][address as usize - 0xd000]
                }
            }
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xfe00..=0xfe9f => self.oam[address as usize - 0xfe00],
            0xfea0..=0xff7f => 0,
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80],
            0xffff => 0,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1fff => {
                self.external_ram_enabled = match self.cart.cart_type().controller {
                    Controller::MBC1 => (value & 0x0f) == 0x0a,
                    Controller::MBC2 | Controller::MBC3 => {
                        if address & 0x0100 == 0 {
                            !self.external_ram_enabled
                        } else {
                            self.external_ram_enabled
                        }
                    }
                    _ => self.external_ram_enabled,
                }
            }
            0x2000..=0x3fff => {
                self.rom_bank = match self.cart.cart_type().controller {
                    Controller::MBC1 => {
                        let mut value = value & 0x1f;
                        value = match value {
                            0x00 | 0x20 | 0x40 | 0x60 => value + 0x01,
                            _ => value,
                        };
                        (self.rom_bank & 0xef) | (value & 0x1f)
                    }
                    Controller::MBC2 => {
                        if address & 0x0100 == 0 {
                            self.rom_bank
                        } else {
                            let value = value & 0x0f;
                            match value {
                                0x00 | 0x20 | 0x40 | 0x60 => value + 0x01,
                                _ => value,
                            }
                        }
                    }
                    _ => value,
                }
            }
            0x4000..=0x5fff => match self.cart.cart_type().controller {
                Controller::MBC1 => {
                    if self.rom_ram_mode == 0x01 {
                        self.external_ram_bank = value & 0x03;
                    } else {
                        self.rom_bank = (self.rom_bank & 0xcf) | ((value & 0x03) << 4);
                    }
                }
                _ => {}
            },
            0x6000..=0x7fff => match self.cart.cart_type().controller {
                Controller::MBC1 => {
                    self.rom_ram_mode = value & 0x01;
                    if self.rom_ram_mode == 0x00 {
                        self.external_ram_bank = 0;
                    } else {
                        self.rom_bank &= 0x1f;
                    }
                }
                _ => {}
            },
            0x8000..=0x9fff => {
                let address = (0x8000 * self.video_bank as u16) + (address - 0x8000);
                self.video[address as usize] = value;
            }
            0xa000..=0xbfff => {
                if self.external_ram_enabled {
                    let address = (0x8000 * self.external_ram_bank as u16) + (address - 0x8000);
                    self.external_ram[address as usize] = value;
                }
            }
            0xc000..=0xcfff => self.work_ram[address as usize - 0xc000] = value,
            0xd000..=0xdfff => {
                if self.work_ram_bank == 0 {
                    self.work_ram[address as usize - 0xc000] = value;
                } else {
                    self.work_ram_banks[self.work_ram_bank as usize - 1]
                        [address as usize - 0xd000] = value;
                }
            }
            0xe000..=0xfdff => self.write(address - 0x2000, value),
            0xfe00..=0xfe9f => self.oam[address as usize - 0xfe00] = value,
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80] = value,
            _ => {}
        }
    }

    pub fn disable_booting(&mut self) {
        self.booting = false;
    }
}

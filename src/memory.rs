use crate::cart::{Cartridge, Controller};
use crate::mbc;
use std::{fs::File, io::Read, path::Path};

pub struct Memory {
    mbc: Box<dyn mbc::MBC>,
    work_ram: [u8; 0x8000],
    high_ram: [u8; 0x7f],
    video: [u8; 0x4000],
    oam: [u8; 0xa0],
    io: [u8; 0x80],
    bootrom: Vec<u8>,

    booting: bool,
    work_ram_bank: u8,
    video_bank: u8,

    oam_access: bool,
    vram_access: bool,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            mbc: Box::new(mbc::None::new(Cartridge::new())),
            work_ram: [0; 0x8000],
            high_ram: [0; 0x7f],
            video: [0; 0x4000],
            oam: [0; 0xa0],
            io: [0; 0x80],
            bootrom: Vec::new(),

            booting: false,
            work_ram_bank: 1,
            video_bank: 0,

            oam_access: true,
            vram_access: true,
        }
    }

    pub fn with_cartridge(mut self, cart: Cartridge) -> Self {
        self.mbc = match cart.cart_type().controller {
            Controller::None => Box::new(mbc::None::new(cart)),
            Controller::MBC1 => Box::new(mbc::MBC1::new(cart)),
            Controller::MBC2 => Box::new(mbc::MBC2::new(cart)),
            Controller::MBC3 => Box::new(mbc::MBC3::new(cart)),
            _ => panic!("unsupprted mbc"),
        };
        self
    }

    pub fn with_bootrom(mut self, data: &[u8]) -> Self {
        self.booting = true;
        self.bootrom = data.to_vec();
        self
    }

    pub fn with_bootrom_path(self, rom: &Path) -> Result<Self, std::io::Error> {
        let mut data = Vec::<u8>::new();
        let mut file = File::open(rom)?;
        file.read_to_end(&mut data)?;
        Ok(self.with_bootrom(&data))
    }

    pub fn set_oam_access(&mut self, access: bool) {
        self.oam_access = access;
    }

    pub fn set_vram_access(&mut self, access: bool) {
        self.vram_access = access;
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x00ff => {
                if self.booting {
                    self.bootrom[address as usize]
                } else {
                    self.mbc.read(address)
                }
            }
            0x0100..=0x3fff => self.mbc.read(address),
            0x4000..=0x7fff => self.mbc.read(address),
            0x8000..=0x9fff => {
                if !self.vram_access {
                    return 0x00;
                }
                let address = (0x2000 * self.video_bank as u16) + (address - 0x8000);
                self.video[address as usize]
            }
            0xa000..=0xbfff => self.mbc.read(address),
            0xc000..=0xcfff | 0xe000..=0xefff => self.work_ram[address as usize & 0x0fff],
            0xd000..=0xdfff | 0xf000..=0xfdff => {
                self.work_ram[(self.work_ram_bank as usize * 0x1000) | address as usize & 0x0fff]
            }
            0xfe00..=0xfe9f => {
                if self.oam_access {
                    self.oam[address as usize - 0xfe00]
                } else {
                    0x00
                }
            }
            0xfea0..=0xfeff => 0xff,
            0xff70 => self.work_ram_bank,
            0xff00..=0xff7f => self.io[address as usize - 0xff00],
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80],
            0xffff => unreachable!(),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1fff => self.mbc.write(address, value),
            0x2000..=0x3fff => self.mbc.write(address, value),
            0x4000..=0x5fff => self.mbc.write(address, value),
            0x6000..=0x7fff => self.mbc.write(address, value),
            0x8000..=0x9fff => {
                if self.vram_access {
                    let address = (0x2000 * self.video_bank as u16) + (address - 0x8000);
                    self.video[address as usize] = value;
                }
            }
            0xa000..=0xbfff => self.mbc.write(address, value),
            0xc000..=0xcfff | 0xe000..=0xefff => self.work_ram[address as usize & 0x0fff] = value,
            0xd000..=0xdfff | 0xf000..=0xfdff => {
                self.work_ram
                    [(self.work_ram_bank as usize * 0x1000) | (address as usize & 0x0fff)] = value
            }
            0xfe00..=0xfe9f => {
                if self.oam_access {
                    self.oam[address as usize - 0xfe00] = value
                }
            }
            0xfea0..=0xfeff => {}
            0xff70 => {
                self.work_ram_bank = match value & 0x07 {
                    0 => 1,
                    n => n,
                }
            }
            0xff00..=0xff6f | 0xff71..=0xff7f => self.io[address as usize - 0xff00] = value,
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80] = value,
            0xffff => panic!("tried accessing regs"),
        }
    }

    pub fn has_bootrom(&self) -> bool {
        !self.bootrom.is_empty()
    }

    pub fn disable_booting(&mut self) {
        self.booting = false;
    }
}

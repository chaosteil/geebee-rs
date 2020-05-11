use crate::cart;
use crate::mbc::MBC;

pub struct MBC1 {
    cart: cart::Cartridge,
    rom_bank: usize,

    rom_ram_mode: u8,
    ram_enabled: bool,
    ram_bank: u8,
    ram: Vec<u8>,
}

impl MBC1 {
    pub fn new(cart: cart::Cartridge) -> Self {
        let ram_size = match cart.ram_size() {
            0 => 0,
            s => 0x800 << s,
        };
        Self {
            cart,
            rom_bank: 1,

            rom_ram_mode: 0,
            ram_enabled: false,
            ram_bank: 0,
            ram: vec![0; ram_size],
        }
    }
}

impl MBC for MBC1 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3fff => self.cart.data()[address as usize],
            0x4000..=0x7fff => {
                let address = (0x4000 * (self.rom_bank)) + (address as usize - 0x4000);
                self.cart.data()[address]
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let address = (0x1000 * self.ram_bank as u16) + (address - 0xa000);
                    self.ram[address as usize]
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1fff => {
                self.ram_enabled = (value & 0x0f) == 0x0a;
            }
            0x2000..=0x3fff => {
                let mut value = value & 0x1f;
                value = match value {
                    0x00 | 0x20 | 0x40 | 0x60 => value + 0x01,
                    _ => value,
                };
                self.rom_bank = (self.rom_bank & 0xe0) | (value as usize & 0x1f)
            }
            0x4000..=0x5fff => {
                if self.rom_ram_mode == 0x01 {
                    self.ram_bank = value & 0x03;
                } else {
                    self.rom_bank = (self.rom_bank & 0xcf) | ((value as usize & 0x03) << 4);
                }
            }
            0x6000..=0x7fff => {
                self.rom_ram_mode = value & 0x01;
                if self.rom_ram_mode == 0x00 {
                    self.ram_bank = 0;
                } else {
                    self.rom_bank &= 0x1f;
                }
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let address = (0x1000 * self.ram_bank as u16) + (address - 0xa000);
                    self.ram[address as usize] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

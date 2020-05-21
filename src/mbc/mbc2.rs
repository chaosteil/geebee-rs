use crate::cart;
use crate::mbc::{handle_save, prepare_save, MBC};

pub struct MBC2 {
    cart: cart::Cartridge,
    rom_bank: usize,

    ram_enabled: bool,
    ram: Vec<u8>,
}

impl MBC2 {
    pub fn new(cart: cart::Cartridge) -> Self {
        let ram = prepare_save(&cart, 512).unwrap();
        Self {
            cart,
            rom_bank: 1,
            ram_enabled: false,
            ram,
        }
    }
}

impl MBC for MBC2 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3fff => self.cart.data()[address as usize],
            0x4000..=0x7fff => {
                let address = (0x4000 * (self.rom_bank)) + (address as usize - 0x4000);
                self.cart.data()[address]
            }
            0xa000..=0xa1ff => {
                if self.ram_enabled {
                    self.ram[address as usize] & 0x0f
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
                if address & 0x0100 == 0 {
                    self.ram_enabled = !self.ram_enabled;
                }
                if !self.ram_enabled {
                    handle_save(&self.cart, &self.ram).unwrap();
                }
            }
            0x2000..=0x3fff => {
                self.rom_bank = if address & 0x0100 == 0 {
                    self.rom_bank
                } else {
                    let value = value as usize & 0x0f;
                    match value {
                        0x00 | 0x20 | 0x40 | 0x60 => value + 0x01,
                        _ => value,
                    }
                } & 0x0f;
            }
            0xa000..=0xa1ff => {
                if self.ram_enabled {
                    let address = address - 0xa000;
                    self.ram[address as usize] = value & 0x0f;
                }
            }
            _ => unreachable!(),
        }
    }
}

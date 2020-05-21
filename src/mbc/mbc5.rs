use crate::cart;
use crate::mbc::MBC;

pub struct MBC5 {
    cart: cart::Cartridge,
    rom_bank: usize,

    ram_enabled: bool,
    ram_bank: usize,
    ram: Vec<u8>,
}

impl MBC5 {
    pub fn new(cart: cart::Cartridge) -> Self {
        let ram_size = match cart.ram_size() {
            0 => 0,
            s => 0x2000 << s,
        };
        Self {
            cart,
            rom_bank: 1,

            ram_enabled: false,
            ram_bank: 0,
            ram: vec![0; ram_size],
        }
    }
}

impl MBC for MBC5 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3fff => self.cart.data()[address as usize],
            0x4000..=0x7fff => {
                let address = (0x4000 * (self.rom_bank)) + (address as usize - 0x4000);
                self.cart.data()[address]
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let address = (0x2000 * self.ram_bank as u16) + (address - 0xa000);
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
            0x0000..=0x1fff => self.ram_enabled = (value & 0x0f) == 0x0a,
            0x2000..=0x2fff => self.rom_bank = (self.rom_bank & 0xff00) | value as usize,
            0x3000..=0x3fff => {
                self.rom_bank =
                    if value & 0x01 != 0 { 0x0100 } else { 0x0000 } | (self.rom_bank & 0x00ff)
            }
            0x4000..=0x5fff => self.ram_bank = (value & 0x0f) as usize,
            0x6000..=0x7fff => {}
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let address = (0x2000 * self.ram_bank) + ((address as usize) - 0xa000);
                    self.ram[address] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

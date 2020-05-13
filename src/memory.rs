use crate::cart::{Cartridge, Controller};
use crate::mbc;
use crate::mbc::MBC;
use std::{fs::File, io::Read, path::Path};

pub struct Memory {
    state: State,
    work_ram: Vec<u8>,
    high_ram: [u8; 0x7f],
    io: [u8; 0x80],

    work_ram_bank: usize,

    cgb_mode: bool,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            state: State::None,
            high_ram: [0; 0x7f],
            io: [0; 0x80],

            work_ram_bank: 1,
            work_ram: vec![0; 0x2000],

            cgb_mode: false,
        }
    }

    pub fn with_cartridge(cart: Cartridge) -> Self {
        let mut mem = Self::new();
        mem.cgb_mode = cart.cgb();
        if mem.cgb_mode {
            mem.work_ram = vec![0; 0x8000];
        }
        mem.state = State::MBC(match cart.cart_type().controller {
            Controller::None => Box::new(mbc::None::new(cart)),
            Controller::MBC1 => Box::new(mbc::MBC1::new(cart)),
            Controller::MBC2 => Box::new(mbc::MBC2::new(cart)),
            Controller::MBC3 => Box::new(mbc::MBC3::new(cart)),
            Controller::MBC5 => Box::new(mbc::MBC5::new(cart)),
            _ => panic!("unsupprted mbc"),
        });
        mem
    }

    pub fn with_bootrom(mut self, data: &[u8]) -> Self {
        self.state = match self.state {
            State::MBC(m) => State::Boot(mbc::Boot::with_mbc(data, m)),
            State::None => State::Boot(mbc::Boot::with_data(data)),
            _ => panic!("already initialized with bootrom"),
        };
        self
    }

    pub fn with_bootrom_path(self, rom: &Path) -> Result<Self, std::io::Error> {
        let mut data = Vec::<u8>::new();
        let mut file = File::open(rom)?;
        file.read_to_end(&mut data)?;
        Ok(self.with_bootrom(&data))
    }

    pub fn cgb_mode(&self) -> bool {
        self.cgb_mode
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0xbfff => self.state.read(address),
            0xc000..=0xcfff => self.work_ram[address as usize - 0xc000],
            0xd000..=0xdfff => {
                self.work_ram[(self.work_ram_bank * 0x1000) + (address as usize - 0xd000)]
            }
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xfea0..=0xfeff => 0xff,
            0xff70 => {
                if self.cgb_mode {
                    self.work_ram_bank as u8
                } else {
                    0xff
                }
            }
            0xff00..=0xff7f => self.io[address as usize - 0xff00],
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80],
            _ => panic!("should have been handled earlier {:04x}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0xbfff => self.state.write(address, value),
            0xc000..=0xcfff => self.work_ram[address as usize - 0xc000] = value,
            0xd000..=0xdfff => {
                self.work_ram[(self.work_ram_bank * 0x1000) + (address as usize - 0xd000)] = value
            }
            0xe000..=0xfdff => self.write(address - 0x2000, value),
            0xfea0..=0xfeff => {}
            0xff70 => {
                if self.cgb_mode {
                    self.work_ram_bank = match value & 0x07 {
                        0 => 1,
                        n => n as usize,
                    }
                }
            }
            0xff00..=0xff6f | 0xff71..=0xff7f => self.io[address as usize - 0xff00] = value,
            0xff80..=0xfffe => self.high_ram[address as usize - 0xff80] = value,
            _ => panic!(
                "should have been handled earlier {:04x} {:02x}",
                address, value
            ),
        }
    }

    pub fn has_bootrom(&self) -> bool {
        match self.state {
            State::Boot(_) => true,
            _ => false,
        }
    }

    pub fn disable_booting(&mut self) {
        let state = std::mem::take(&mut self.state);
        self.state = match state {
            State::Boot(b) => State::MBC(b.mbc()),
            _ => state,
        }
    }
}

enum State {
    None,
    Boot(mbc::Boot),
    MBC(Box<dyn mbc::MBC>),
}

impl Default for State {
    fn default() -> State {
        State::None
    }
}

impl mbc::MBC for State {
    fn read(&self, address: u16) -> u8 {
        match self {
            State::MBC(m) => m.read(address),
            State::Boot(b) => b.read(address),
            _ => panic!("read from invalid MBC state"),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match self {
            State::MBC(m) => m.write(address, value),
            State::Boot(b) => b.write(address, value),
            _ => panic!("write into invalid MBC state"),
        }
    }
}

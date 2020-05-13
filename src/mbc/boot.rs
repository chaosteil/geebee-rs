use crate::mbc::MBC;

pub struct Boot {
    rom: Vec<u8>,
    mbc: Option<Box<dyn MBC>>,
}

impl Boot {
    pub fn with_data(data: &[u8]) -> Self {
        Self {
            rom: data.into(),
            mbc: None,
        }
    }

    pub fn with_mbc(data: &[u8], mbc: Box<dyn MBC>) -> Self {
        Self {
            rom: data.into(),
            mbc: Some(mbc),
        }
    }

    pub fn mbc(self) -> Box<dyn MBC> {
        self.mbc.unwrap()
    }
}

impl MBC for Boot {
    fn read(&self, address: u16) -> u8 {
        if address >= 0x100 && address <= 0x014f {
            self.mbc.as_ref().unwrap().read(address)
        } else {
            self.rom[address as usize]
        }
    }

    fn write(&mut self, _address: u16, _value: u8) {}
}

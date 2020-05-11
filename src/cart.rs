use std::fs::File;
use std::path::Path;
use std::string;
use std::{io, io::Read};
use thiserror::Error;

pub struct Cartridge {
    title: String,
    cart_type: CartType,
    ram_size: u8,
    cgb: bool,
    sgb: bool,
    data: Vec<u8>,
}

impl Cartridge {
    pub fn new() -> Self {
        Self {
            title: "EMPTY".to_string(),
            cart_type: CartType::default(),
            ram_size: 9,
            cgb: false,
            sgb: false,
            data: vec![],
        }
    }

    pub fn with_path(self, cart: &Path) -> Result<Self, Error> {
        let mut data = Vec::<u8>::new();
        let mut file = File::open(cart)?;
        file.read_to_end(&mut data)?;
        self.with_data(&data)
    }

    pub fn with_data(mut self, data: &[u8]) -> Result<Self, Error> {
        if data.len() < 16384 {
            return Err(Error::InvalidRom);
        }
        Self::verify_checksum(&data)?;
        self.title = String::from_utf8((data[0x0134..0x134 + 11]).to_vec())?
            .trim_matches(char::from(0))
            .to_string();
        self.cart_type = CartType::from(data[0x0147]);
        self.ram_size = data[0x149];
        self.cgb = match data[0x0143] {
            0x80 | 0xc0 => true,
            _ => false,
        };
        self.sgb = data[0x0146] == 0x03;
        self.data = data.to_vec();
        println!(
            "Cart Data: {}, {:?} CGB: {}",
            self.title(),
            self.cart_type(),
            self.cgb
        );
        Ok(self)
    }

    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn cart_type(&self) -> CartType {
        self.cart_type.clone()
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn ram_size(&self) -> u8 {
        self.ram_size
    }

    fn verify_checksum(data: &[u8]) -> Result<(), Error> {
        let mut x: u8 = 0;
        for i in data.iter().take(0x14c + 1).skip(0x0134) {
            x = x.overflowing_sub(*i).0.overflowing_sub(1).0;
        }
        if x != data[0x014d] {
            Err(Error::ChecksumFailed)
        } else {
            Ok(())
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct CartType {
    pub controller: Controller,
    pub ram: bool,
    pub battery: bool,
    pub timer: bool,
    pub rumble: bool,
}

impl CartType {
    fn new(controller: Controller) -> Self {
        Self {
            controller,
            ..Default::default()
        }
    }
    fn with_ram(mut self) -> Self {
        self.ram = true;
        self
    }
    fn with_battery(mut self) -> Self {
        self.battery = true;
        self
    }
    fn with_timer(mut self) -> Self {
        self.timer = true;
        self
    }
    fn with_rumble(mut self) -> Self {
        self.rumble = true;
        self
    }
}

#[derive(Clone, Debug)]
pub enum Controller {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC4,
    MBC5,
}

impl Default for Controller {
    fn default() -> Self {
        Self::None
    }
}

impl From<u8> for CartType {
    fn from(t: u8) -> CartType {
        match t {
            0x00 => CartType::new(Controller::None),
            0x01 => CartType::new(Controller::MBC1),
            0x02 => CartType::new(Controller::MBC1).with_ram(),
            0x03 => CartType::new(Controller::MBC1).with_ram().with_battery(),
            0x05 => CartType::new(Controller::MBC2),
            0x06 => CartType::new(Controller::MBC2).with_battery(),
            0x08 => CartType::new(Controller::None).with_ram(),
            0x09 => CartType::new(Controller::None).with_ram().with_battery(),
            0x0f => CartType::new(Controller::MBC3).with_timer().with_battery(),
            0x10 => CartType::new(Controller::MBC3)
                .with_timer()
                .with_ram()
                .with_battery(),
            0x11 => CartType::new(Controller::MBC3),
            0x12 => CartType::new(Controller::MBC3).with_ram(),
            0x13 => CartType::new(Controller::MBC3).with_ram().with_battery(),
            0x15 => CartType::new(Controller::MBC4),
            0x16 => CartType::new(Controller::MBC4).with_ram(),
            0x17 => CartType::new(Controller::MBC4).with_ram().with_battery(),
            0x19 => CartType::new(Controller::MBC5),
            0x1a => CartType::new(Controller::MBC5).with_ram(),
            0x1b => CartType::new(Controller::MBC5).with_ram().with_battery(),
            0x1c => CartType::new(Controller::MBC5).with_rumble(),
            0x1d => CartType::new(Controller::MBC5).with_rumble().with_ram(),
            0x1e => CartType::new(Controller::MBC5)
                .with_rumble()
                .with_ram()
                .with_battery(),
            _ => panic!("unable to handle cartridge type {}", t),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("rom is not a valid gameboy rom")]
    InvalidRom,
    #[error("checksum check fails")]
    ChecksumFailed,
    #[error("io error")]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("str error")]
    Str {
        #[from]
        source: string::FromUtf8Error,
    },
}

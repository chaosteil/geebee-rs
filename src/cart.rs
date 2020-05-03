use std::fs::File;
use std::path::Path;
use std::string;
use std::{io, io::Read};
use thiserror::Error;

pub struct Cartridge {
    title: String,
    cart_type: CartType,
    cgb: bool,
    sgb: bool,
    data: Vec<u8>,
}

impl Cartridge {
    pub fn new() -> Self {
        Self {
            title: "EMPTY".to_string(),
            cart_type: CartType::default(),
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
        self.title = String::from_utf8((data[0x0134..0x134 + 11]).to_vec())?;
        self.cart_type = CartType::from(data[0x0147]);
        self.cgb = match data[0x0143] {
            0x80 | 0xc0 => true,
            _ => false,
        };
        self.sgb = data[0x0146] == 0x03;
        self.data = data.to_vec();
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

#[derive(Default, Clone)]
pub struct CartType {
    pub controller: Controller,
    pub ram: bool,
    pub battery: bool,
    pub timer: bool,
    pub rumble: bool,
}

#[derive(Clone)]
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
            0x00 => CartType {
                controller: Controller::None,
                ..Default::default()
            },
            0x01 => CartType {
                controller: Controller::MBC1,
                ram: true,
                ..Default::default()
            },
            0x03 => CartType {
                controller: Controller::MBC1,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x05 => CartType {
                controller: Controller::MBC2,
                ..Default::default()
            },
            0x06 => CartType {
                controller: Controller::MBC2,
                battery: true,
                ..Default::default()
            },
            0x08 => CartType {
                controller: Controller::None,
                ram: true,
                ..Default::default()
            },
            0x09 => CartType {
                controller: Controller::None,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x0f => CartType {
                controller: Controller::MBC3,
                timer: true,
                battery: true,
                ..Default::default()
            },
            0x10 => CartType {
                controller: Controller::MBC3,
                timer: true,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x11 => CartType {
                controller: Controller::MBC3,
                ..Default::default()
            },
            0x12 => CartType {
                controller: Controller::MBC3,
                ram: true,
                ..Default::default()
            },
            0x13 => CartType {
                controller: Controller::MBC3,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x15 => CartType {
                controller: Controller::MBC4,
                ..Default::default()
            },
            0x16 => CartType {
                controller: Controller::MBC4,
                ram: true,
                ..Default::default()
            },
            0x17 => CartType {
                controller: Controller::MBC4,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x19 => CartType {
                controller: Controller::MBC5,
                ..Default::default()
            },
            0x1a => CartType {
                controller: Controller::MBC5,
                ram: true,
                ..Default::default()
            },
            0x1b => CartType {
                controller: Controller::MBC5,
                ram: true,
                battery: true,
                ..Default::default()
            },
            0x1c => CartType {
                controller: Controller::MBC5,
                rumble: true,
                ..Default::default()
            },
            0x1d => CartType {
                controller: Controller::MBC5,
                rumble: true,
                ram: true,
                ..Default::default()
            },
            0x1e => CartType {
                controller: Controller::MBC5,
                rumble: true,
                ram: true,
                battery: true,
                ..Default::default()
            },
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

use crate::cart;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{io, io::Read, io::Write};

pub trait MBC {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub fn prepare_save(cart: &cart::Cartridge, size: usize) -> Result<Vec<u8>, io::Error> {
    let mut data = vec![0; size];
    if !can_handle_savefiles(cart) {
        return Ok(data);
    }
    let p = savepath(cart.path().unwrap());
    match File::open(&p) {
        Ok(mut f) => {
            f.read_to_end(&mut data)?;
        }
        Err(_) => {}
    };
    Ok(data)
}

pub fn handle_save(cart: &cart::Cartridge, ram: &[u8]) -> Result<(), io::Error> {
    if !can_handle_savefiles(cart) {
        return Ok(());
    }
    let p = savepath(cart.path().unwrap());
    let mut f = File::create(p)?;
    f.write_all(ram)?;
    Ok(())
}

fn can_handle_savefiles(cart: &cart::Cartridge) -> bool {
    cart.cart_type().battery && cart.path().is_some()
}

fn savepath(path: &Path) -> PathBuf {
    path.with_extension(Path::new("gb.save"))
}

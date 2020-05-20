use crate::cart;
use crate::mbc::MBC;

pub struct None {
    cart: cart::Cartridge,
}

impl None {
    pub fn new(cart: cart::Cartridge) -> Self {
        Self { cart }
    }
}

impl MBC for None {
    fn read(&self, address: u16) -> u8 {
        self.cart.data()[address as usize]
    }

    fn write(&mut self, _address: u16, _value: u8) {}
}

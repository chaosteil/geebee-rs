pub fn extract(value: u16) -> (u8, u8) {
    (((value & 0xf0) >> 8) as u8, (value & 0x0f) as u8)
}

pub fn assemble(high: u8, low: u8) -> u16 {
    ((high as u16) << 8) | low as u16
}

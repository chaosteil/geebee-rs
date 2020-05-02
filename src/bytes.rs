pub fn extract(value: u16) -> (u8, u8) {
    (((value & 0xff00) >> 8) as u8, (value & 0x00ff) as u8)
}

pub fn assemble(high: u8, low: u8) -> u16 {
    ((high as u16) << 8) | (low as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract() {
        assert_eq!(extract(0x0000), (0x00, 0x00));
        assert_eq!(extract(0xffff), (0xff, 0xff));
        assert_eq!(extract(0x0102), (0x01, 0x02));
    }

    #[test]
    fn test_assemble() {
        assert_eq!(assemble(0x00, 0x00), 0x0000);
        assert_eq!(assemble(0xff, 0xff), 0xffff);
        assert_eq!(assemble(0x01, 0x02), 0x0102);
    }
}

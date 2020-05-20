pub type Timing = u16;

pub struct Timer {
    div: SubTimer,
    tima: SubTimer,
    tma: u8,
    tac: TAC,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: SubTimer::new(TAC::from(0x03).rate()),
            tima: SubTimer::new(TAC::from(0x00).rate()),
            tma: 0,
            tac: TAC::from(0),
        }
    }

    pub fn advance(&mut self, timing: Timing) -> bool {
        self.div.inc(timing);
        if !self.tac.start {
            return false;
        }
        let overflow = self.tima.inc(timing);
        if overflow {
            self.tima.set_timer(self.tma);
        }
        overflow
    }

    pub fn reset_div(&mut self) {
        self.div.set_timer(0)
    }

    pub fn div(&self) -> u8 {
        self.div.into()
    }

    pub fn set_tima(&mut self, value: u8) {
        self.tima.set_timer(value)
    }

    pub fn tima(&self) -> u8 {
        self.tima.into()
    }

    pub fn set_tma(&mut self, value: u8) {
        self.tma = value;
    }

    pub fn tma(&self) -> u8 {
        self.tma
    }

    pub fn set_tac(&mut self, value: u8) {
        self.tac = value.into();
        self.tima.set_rate(self.tac.rate());
    }

    pub fn tac(&self) -> u8 {
        self.tac.into()
    }
}

#[derive(Clone, Copy)]
struct SubTimer {
    timer: u32,
    value: u8,
    rate: u32,
}

impl SubTimer {
    fn new(rate: u32) -> Self {
        Self {
            timer: 0,
            value: 0,
            rate,
        }
    }
    fn set_timer(&mut self, value: u8) {
        self.value = value;
    }
    fn set_rate(&mut self, rate: u32) {
        self.rate = rate;
    }
    fn inc(&mut self, value: Timing) -> bool {
        let mut overflow = false;
        self.timer += value as u32;
        while self.timer > self.rate {
            self.timer -= self.rate;
            self.value = self.value.wrapping_add(1);
            if self.value == 0 {
                overflow = true;
            }
        }
        overflow
    }
}

impl From<SubTimer> for u8 {
    fn from(timer: SubTimer) -> u8 {
        timer.value
    }
}

#[derive(Clone, Copy)]
struct TAC {
    start: bool,
    clock: u8,
}

impl TAC {
    fn rate(self) -> u32 {
        match self.clock {
            0x00 => 1024,
            0x01 => 16,
            0x02 => 64,
            0x03 => 256,
            _ => panic!("invalid TAC clock"),
        }
    }
}

impl From<u8> for TAC {
    fn from(other: u8) -> TAC {
        TAC {
            start: (other & 0x04) != 0,
            clock: other & 0x03,
        }
    }
}

impl From<TAC> for u8 {
    fn from(other: TAC) -> u8 {
        other.clock | (if other.start { 0x04 } else { 0x00 })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn subtimer() {
        let mut st = SubTimer::new(16);
        assert_eq!(u8::from(st), 0);
        assert_eq!(st.inc(8), false);
        assert_eq!(u8::from(st), 0);
        assert_eq!(st.inc(8), false);
        assert_eq!(u8::from(st), 0);
        assert_eq!(st.inc(16), false);
        assert_eq!(u8::from(st), 1);
        st.set_timer(250);
        assert_eq!(u8::from(st), 250);

        st.set_rate(4);
        assert_eq!(u8::from(st), 250);
        assert_eq!(st.inc(4), false);
        assert_eq!(u8::from(st), 254);

        st.set_rate(1);
        assert_eq!(u8::from(st), 254);
        assert_eq!(st.inc(2), true);
        assert_eq!(u8::from(st), 3);
        assert_eq!(st.inc(0xff), true);
        assert_eq!(u8::from(st), 2);
    }
}

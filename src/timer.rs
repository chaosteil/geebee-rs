pub type Timing = u8;

pub struct Timer {
    div: SubTimer,
    tima: SubTimer,
    tma: u8,
    tac: TAC,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: SubTimer::new(256),
            tima: SubTimer::new(1024),
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
        if !overflow {
            return false;
        }
        self.tima.set_timer(self.tma);
        true
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
    rate: u32,
}

impl SubTimer {
    fn new(rate: u32) -> Self {
        Self { timer: 0, rate }
    }
    fn set_timer(&mut self, value: u8) {
        self.timer = value as u32 * self.rate;
    }
    fn set_rate(&mut self, rate: u32) {
        self.timer /= self.rate;
        self.rate = rate;
        self.timer *= self.rate;
    }
    fn inc(&mut self, value: u8) -> bool {
        self.timer += value as u32;
        if self.timer / self.rate > 0xff {
            self.timer -= self.rate * 0xff;
            return true;
        }
        false
    }
}

impl From<SubTimer> for u8 {
    fn from(timer: SubTimer) -> u8 {
        (timer.timer / timer.rate) as u8
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
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => panic!("invalid TAC clock"),
        }
    }
}

impl From<u8> for TAC {
    fn from(other: u8) -> TAC {
        TAC {
            start: (other & 0x04) > 0,
            clock: other & 0x03,
        }
    }
}

impl From<TAC> for u8 {
    fn from(other: TAC) -> u8 {
        other.clock | (if other.start { 0x04 } else { 0x00 })
    }
}

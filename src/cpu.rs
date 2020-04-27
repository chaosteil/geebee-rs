use crate::bytes;
use crate::memory::Memory;
use crate::timer;

pub struct CPU {
    memory: Memory,
    regs: Registers,
    interrupts: Interrupts,
    timer: timer::Timer,

    halt: bool,
    sp: u16,
    pc: u16,
}

impl CPU {
    pub fn new(memory: Memory) -> Self {
        Self {
            memory,
            regs: Registers::default(),
            interrupts: Interrupts::default(),
            timer: timer::Timer::new(),
            halt: false,
            sp: 0,
            pc: 0,
        }
    }
    fn init(&mut self) {
        self.pc = 0x0100;
    }
    pub fn step(&mut self) {
        let timing = if let Some(timing) = self.handle_interrupts() {
            timing
        } else if !self.halt {
            self.read_instruction()
        } else {
            4
        };
        self.advance_timer(timing);
    }

    fn read_instruction(&mut self) -> timer::Timing {
        4
    }

    fn handle_interrupts(&mut self) -> Option<timer::Timing> {
        let has_interrupt = (self.interrupts.enable & self.interrupts.flag) > 0;
        if !self.interrupts.enabled || !has_interrupt {
            if !self.interrupts.enabled && self.interrupts.flag > 0 && self.halt {
                self.halt = false;
                return Some(4);
            }
            return None;
        }
        for i in 0..=4 {
            if self.interrupts.flag & (0x01 << i) == 0 {
                continue;
            }
            self.interrupts.flag &= !(0x01 << i);
            self.interrupts.enabled = false;
            self.push(self.pc);
            self.pc = 0x40 + i * 0x08;
            self.halt = false;
        }
        Some(12)
    }

    fn advance_timer(&mut self, timing: timer::Timing) {
        if !self.timer.advance(timing) {
            return;
        }
        self.interrupts.flag |= 0x04;
    }

    fn read(&mut self, address: u16) -> u8 {
        match address {
            0xff04 => self.timer.div(),
            0xff05 => self.timer.tima(),
            0xff06 => self.timer.tma(),
            0xff07 => self.timer.tac(),
            0xff0f => self.interrupts.flag,
            0xffff => self.interrupts.enable,
            _ => self.memory.read(address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xff04 => self.timer.reset_div(),
            0xff05 => self.timer.set_tima(value),
            0xff06 => self.timer.set_tma(value),
            0xff07 => self.timer.set_tac(value),
            0xff0f => self.interrupts.flag = value,
            0xffff => self.interrupts.enable = value,
            _ => self.memory.write(address, value),
        }
    }

    fn push(&mut self, value: u16) -> timer::Timing {
        let (high, low) = bytes::extract(value);
        self.write(self.sp, high);
        self.write(self.sp - 1, low);
        self.sp -= 2;
        16
    }

    fn pop(&mut self) -> (u16, timer::Timing) {
        let value = bytes::assemble(self.read(self.sp + 1), self.read(self.sp));
        self.sp += 2;
        (value, 12)
    }
}

#[derive(Default)]
struct Interrupts {
    enabled: bool,
    enable: u8,
    flag: u8,
}

#[derive(Default)]
struct Registers {
    a: u8,
    f: Flags,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

impl Registers {
    fn af(&self) -> u16 {
        (self.a as u16) << 8 | u8::from(self.f) as u16
    }
    fn set_af(&mut self, af: u16) {
        self.a = ((af & 0xf0) >> 8) as u8;
        self.f = Flags::from((af & 0x0f) as u8);
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }
    fn set_bc(&mut self, bc: u16) {
        self.b = ((bc & 0xf0) >> 8) as u8;
        self.c = (bc & 0x0f) as u8;
    }

    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }
    fn set_de(&mut self, de: u16) {
        self.d = ((de & 0xf0) >> 8) as u8;
        self.e = (de & 0x0f) as u8;
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }
    fn set_hl(&mut self, hl: u16) {
        self.h = ((hl & 0xf0) >> 8) as u8;
        self.l = (hl & 0x0f) as u8;
    }
}

#[derive(Clone, Copy, Default)]
struct Flags {
    zero: bool,
    carry: bool,
    add_sub: bool,
    half_carry: bool,
}

impl From<u8> for Flags {
    fn from(f: u8) -> Flags {
        Flags {
            zero: f & 0x80 != 0,
            carry: f & 0x40 != 0,
            add_sub: f & 0x20 != 0,
            half_carry: f & 0x10 != 0,
        }
    }
}

impl From<Flags> for u8 {
    fn from(f: Flags) -> u8 {
        (if f.zero { 0x80 } else { 0 })
            | (if f.carry { 0x40 } else { 0 })
            | (if f.add_sub { 0x20 } else { 0 })
            | (if f.half_carry { 0x10 } else { 0 })
    }
}

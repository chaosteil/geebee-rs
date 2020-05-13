use crate::bytes;
use crate::joypad::Joypad;
use crate::lcd::LCD;
use crate::memory::Memory;
use crate::timer;

pub struct CPU {
    memory: Memory,
    lcd: LCD,
    joypad: Joypad,

    regs: Registers,
    interrupts: Interrupts,
    timer: timer::Timer,

    serial: Vec<u8>,
    halt: bool,
    sp: u16,
    pc: u16,

    sb: u8,
    sc: u8,

    speed: u8,
    prepare_speed: bool,
}

impl CPU {
    pub fn new(memory: Memory, lcd: LCD) -> Self {
        let has_bootrom = memory.has_bootrom();
        Self {
            memory,
            lcd,
            joypad: Joypad::new(),
            regs: Registers::new_boot(),
            interrupts: Interrupts::default(),
            timer: timer::Timer::new(),
            serial: Vec::new(),
            halt: false,
            sp: 0xfffe,
            pc: if has_bootrom { 0 } else { 0x0100 },
            sb: 0,
            sc: 0,
            speed: 1,
            prepare_speed: false,
        }
    }

    pub fn cycle(&mut self) {
        loop {
            self.step();
            if self.lcd.done_frame() {
                break;
            }
        }
    }

    pub fn step(&mut self) {
        if self.joypad.check_interrupts() {
            self.interrupts.flag |= 0x10;
        }
        let timing = if let Some(timing) = self.handle_interrupts() {
            timing
        } else if self.halt {
            4
        } else {
            self.handle_instruction()
        };
        if self.timer.advance(timing * self.speed) {
            self.interrupts.flag |= 0x04;
        }
        self.lcd.advance(&mut self.interrupts, timing);
    }

    pub fn lcd(&self) -> &LCD {
        &self.lcd
    }

    pub fn joypad(&mut self) -> &mut Joypad {
        &mut self.joypad
    }

    #[allow(dead_code)]
    fn dump(&self) {
        println!(
            "{:04x} af: {:04x} bc: {:04x} de: {:04x} hl: {:04x} sp: {:04x}",
            self.pc,
            self.regs.af(),
            self.regs.bc(),
            self.regs.de(),
            self.regs.hl(),
            self.sp,
        );
    }

    fn handle_instruction(&mut self) -> timer::Timing {
        let op = self.read_pc();
        self.handle_op(op)
    }

    fn handle_interrupts(&mut self) -> Option<timer::Timing> {
        let has_interrupt = (self.interrupts.enable & self.interrupts.flag) != 0;
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
            self.op_push(self.pc);
            self.pc = 0x40 + i * 0x08;
            self.halt = false;
            break;
        }
        Some(12)
    }

    fn read(&mut self, address: u16) -> u8 {
        match address {
            0xff00 => self.joypad.value(),
            0xff01 => self.sb,
            0xff02 => self.sc,
            0xff04 => self.timer.div(),
            0xff05 => self.timer.tima(),
            0xff06 => self.timer.tma(),
            0xff07 => self.timer.tac(),
            0xff0f => self.interrupts.flag,
            0x8000..=0x9fff
            | 0xfe00..=0xfe9f
            | 0xff40..=0xff4b
            | 0xff4f
            | 0xff51..=0xff55
            | 0xff68..=0xff6b => self.lcd.handle_read(address),
            0xff4d => {
                (if self.speed == 1 { 0x00 } else { 0x80 })
                    | (if self.prepare_speed { 0x01 } else { 0x0 })
            }
            0xff50 => 0,
            0xffff => self.interrupts.enable,
            _ => self.memory.read(address),
        }
    }

    fn read_pc(&mut self) -> u8 {
        let value = self.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xff00 => self.joypad.select(value),
            0xff01 => self.sb = value,
            0xff02 => {
                self.serial.push(self.sb);
                self.interrupts.flag |= 0x08;
            }
            0xff04 => self.timer.reset_div(),
            0xff05 => self.timer.set_tima(value),
            0xff06 => self.timer.set_tma(value),
            0xff07 => self.timer.set_tac(value),
            0xff0f => self.interrupts.flag = value & 0x1f,
            0x8000..=0x9fff
            | 0xfe00..=0xfe9f
            | 0xff40..=0xff4b
            | 0xff4f
            | 0xff51..=0xff55
            | 0xff68..=0xff6b => self.lcd.handle_write(&mut self.memory, address, value),
            0xff4d => self.prepare_speed = value == 0x01,
            0xff50 => {
                if value != 0 {
                    self.memory.disable_booting()
                }
            }
            0xffff => self.interrupts.enable = value & 0x1f,
            _ => self.memory.write(address, value),
        }
    }
}

pub struct Interrupts {
    pub enabled: bool,
    pub enable: u8,
    pub flag: u8,
}

impl Default for Interrupts {
    fn default() -> Self {
        Self {
            enabled: false,
            enable: 0,
            flag: 0,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct Registers {
    pub a: u8,
    pub f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
}

impl Registers {
    fn new_boot() -> Self {
        Self {
            a: 0x01,
            f: 0xb0.into(),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            h: 0x14,
            l: 0x4d,
        }
    }

    fn af(&self) -> u16 {
        bytes::assemble(self.a, self.f.into())
    }
    fn set_af(&mut self, af: u16) {
        let (a, f) = bytes::extract(af);
        self.a = a;
        self.f = f.into();
    }

    fn bc(&self) -> u16 {
        bytes::assemble(self.b, self.c)
    }
    fn set_bc(&mut self, bc: u16) {
        let (b, c) = bytes::extract(bc);
        self.b = b;
        self.c = c;
    }

    fn de(&self) -> u16 {
        bytes::assemble(self.d, self.e)
    }
    fn set_de(&mut self, de: u16) {
        let (d, e) = bytes::extract(de);
        self.d = d;
        self.e = e;
    }

    fn hl(&self) -> u16 {
        bytes::assemble(self.h, self.l)
    }
    fn set_hl(&mut self, hl: u16) {
        let (h, l) = bytes::extract(hl);
        self.h = h;
        self.l = l;
    }
}

#[derive(Clone, Copy, Default)]
pub struct Flags {
    pub zero: bool,
    pub add_sub: bool,
    pub half_carry: bool,
    pub carry: bool,
}

impl From<u8> for Flags {
    fn from(f: u8) -> Flags {
        Flags {
            zero: f & 0x80 != 0,
            add_sub: f & 0x40 != 0,
            half_carry: f & 0x20 != 0,
            carry: f & 0x10 != 0,
        }
    }
}

impl From<Flags> for u8 {
    fn from(f: Flags) -> u8 {
        (if f.zero { 0x80 } else { 0 })
            | (if f.add_sub { 0x40 } else { 0 })
            | (if f.half_carry { 0x20 } else { 0 })
            | (if f.carry { 0x10 } else { 0 })
    }
}

impl CPU {
    fn op_push(&mut self, value: u16) -> timer::Timing {
        let (high, low) = bytes::extract(value);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, high);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, low);
        16
    }

    fn op_pop(&mut self) -> (u16, timer::Timing) {
        let low = self.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let high = self.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        (bytes::assemble(high, low), 12)
    }

    fn op_jr(&mut self, jump: bool) -> timer::Timing {
        let address = self.read_pc();
        if jump {
            self.pc = (self.pc as i16).wrapping_add(address as i8 as i16) as u16;
            12
        } else {
            8
        }
    }

    fn op_ld_16(&mut self) -> (u16, timer::Timing) {
        let low = self.read_pc();
        let high = self.read_pc();

        (bytes::assemble(high, low), 12)
    }

    fn op_write_16_data(&mut self) -> timer::Timing {
        let low = self.read_pc();
        let high = self.read_pc();
        self.write(bytes::assemble(high, low), self.regs.a);
        16
    }

    fn op_load_16_data(&mut self) -> timer::Timing {
        let low = self.read_pc();
        let high = self.read_pc();
        self.regs.a = self.read(bytes::assemble(high, low));
        16
    }

    fn op_inc(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = (value & 0x0f) == 0x0f;
        let value = value.wrapping_add(1);
        self.regs.f.zero = value == 0;

        (value, 4)
    }

    fn op_dec(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.add_sub = true;
        self.regs.f.half_carry = (value & 0x0f) == 0x00;
        let value = value.wrapping_sub(1);
        self.regs.f.zero = value == 0;
        (value, 4)
    }

    fn op_rl(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.carry = value & 0x80 != 0;
        let value = (value << 1) | if self.regs.f.carry { 1 } else { 0 };
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_rlc(&mut self, value: u8) -> (u8, timer::Timing) {
        let carry = self.regs.f.carry;
        self.regs.f.carry = value & 0x80 != 0;
        let value = (value << 1) | if carry { 1 } else { 0 };
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_rr(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.carry = value & 0x01 != 0;
        let value = (value >> 1) | if self.regs.f.carry { 0x80 } else { 0x00 };
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_rrc(&mut self, value: u8) -> (u8, timer::Timing) {
        let carry = self.regs.f.carry;
        self.regs.f.carry = value & 0x01 != 0;
        let value = (value >> 1) | (if carry { 0x80 } else { 0x00 });
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_daa(&mut self) -> timer::Timing {
        let mut adjust = 0;
        let mut carry = false;

        if self.regs.f.half_carry || (!self.regs.f.add_sub && (self.regs.a & 0x0f) > 0x09) {
            adjust |= 0x06;
        }
        if self.regs.f.carry || (!self.regs.f.add_sub && (self.regs.a > 0x99)) {
            adjust |= 0x60;
            carry = true;
        }
        self.regs.a = if self.regs.f.add_sub {
            self.regs.a.wrapping_sub(adjust)
        } else {
            self.regs.a.wrapping_add(adjust)
        };
        self.regs.f.half_carry = false;
        self.regs.f.carry = carry;
        self.regs.f.zero = self.regs.a == 0;
        4
    }

    fn op_add_hl(&mut self, value: u16) -> timer::Timing {
        let (result, carry) = self.regs.hl().overflowing_add(value);
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = (self.regs.hl() & 0x07ff) + (value & 0x07ff) > 0x07ff;
        self.regs.f.carry = carry;
        self.regs.set_hl(result);
        8
    }

    fn op_add(&mut self, value: u8) -> timer::Timing {
        self.regs.f.carry = false;
        self.op_adc(value);
        4
    }

    fn op_adc(&mut self, value: u8) -> timer::Timing {
        let c = if self.regs.f.carry { 1 } else { 0 };
        self.regs.f.add_sub = false;
        self.regs.f.carry = (self.regs.a as u16) + (value as u16) + (c as u16) > 0xff;
        self.regs.f.half_carry = (self.regs.a & 0x0f) + (value & 0x0f) + c > 0x0f;
        self.regs.a = self.regs.a.wrapping_add(value).wrapping_add(c);
        self.regs.f.zero = self.regs.a == 0;
        4
    }

    fn op_sub(&mut self, value: u8) -> timer::Timing {
        self.regs.f.carry = false;
        self.op_sbc(value);
        4
    }

    fn op_sbc(&mut self, value: u8) -> timer::Timing {
        let c = if self.regs.f.carry { 1 } else { 0 };
        self.regs.f.add_sub = true;
        self.regs.f.carry = (self.regs.a as u16) < (value as u16) + (c as u16);
        self.regs.f.half_carry = (self.regs.a & 0x0f) < (value & 0x0f) + c;
        self.regs.a = self.regs.a.wrapping_sub(value).wrapping_sub(c);
        self.regs.f.zero = self.regs.a == 0;
        4
    }

    fn op_and(&mut self, value: u8) -> timer::Timing {
        self.regs.a = self.regs.a & value;
        self.regs.f.zero = self.regs.a == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = true;
        self.regs.f.carry = false;
        4
    }

    fn op_xor(&mut self, value: u8) -> timer::Timing {
        self.regs.a = self.regs.a ^ value;
        self.regs.f.zero = self.regs.a == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        self.regs.f.carry = false;
        4
    }

    fn op_or(&mut self, value: u8) -> timer::Timing {
        self.regs.a = self.regs.a | value;
        self.regs.f.zero = self.regs.a == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        self.regs.f.carry = false;
        4
    }

    fn op_cp(&mut self, value: u8) -> timer::Timing {
        let a = self.regs.a;
        self.op_sub(value);
        self.regs.a = a;
        4
    }

    fn op_ret(&mut self, jump: bool) -> timer::Timing {
        if jump {
            self.pc = self.op_pop().0;
            20
        } else {
            8
        }
    }

    fn op_jp(&mut self, jump: bool) -> timer::Timing {
        let low = self.read_pc();
        let high = self.read_pc();

        if jump {
            self.pc = bytes::assemble(high, low);
            16
        } else {
            12
        }
    }

    fn op_call(&mut self, jump: bool) -> timer::Timing {
        let low = self.read_pc();
        let high = self.read_pc();

        if jump {
            self.op_push(self.pc);
            self.pc = bytes::assemble(high, low);
            24
        } else {
            12
        }
    }

    fn op_rst(&mut self, offset: u8) -> timer::Timing {
        self.op_push(self.pc);
        self.pc = offset as u16;
        16
    }

    fn op_add_sp(&mut self) -> timer::Timing {
        let value = self.read_pc() as i8 as i16 as u16;
        self.regs.f.carry = (self.sp & 0x00ff) + (value & 0x00ff) > 0x00ff;
        self.regs.f.half_carry = (self.sp & 0x000f) + (value & 0x000f) > 0x000f;
        self.regs.f.zero = false;
        self.regs.f.add_sub = false;
        self.sp = self.sp.wrapping_add(value);
        16
    }

    fn op_sll(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.carry = (value & 0x80) != 0;
        let value = value << 1;
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_srl(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.carry = (value & 0x01) != 0;
        let value = value >> 1;
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_sr(&mut self, value: u8) -> (u8, timer::Timing) {
        self.regs.f.carry = (value & 0x01) != 0;
        let value = (value >> 1) | (value & 0x80);
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        (value, 8)
    }

    fn op_swap(&mut self, value: u8) -> (u8, timer::Timing) {
        let value = (value << 4) | (value >> 4);
        self.regs.f.zero = value == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = false;
        self.regs.f.carry = false;
        (value, 8)
    }

    fn op_bit(&mut self, value: u8, bit: u8) -> timer::Timing {
        self.regs.f.zero = ((value >> bit) & 0x01) == 0;
        self.regs.f.add_sub = false;
        self.regs.f.half_carry = true;
        8
    }

    fn op_res(&mut self, value: u8, bit: u8) -> (u8, timer::Timing) {
        (value & !(0x01 << bit), 8)
    }

    fn op_set(&mut self, value: u8, bit: u8) -> (u8, timer::Timing) {
        (value | (0x01 << bit), 8)
    }
}

impl CPU {
    #[rustfmt::skip]
    fn handle_op(&mut self, op: u8) -> timer::Timing {
        match op {
            0x00 => 4,
            0x10 => {
                self.read_pc();
                if self.prepare_speed {
                    self.speed = if self.speed == 1 { 2 } else { 1 };
                    self.prepare_speed = false;
                } else {
                    self.halt = true;
                }
                4
            }

            0x20 => self.op_jr(!self.regs.f.zero),
            0x30 => self.op_jr(!self.regs.f.carry),

            0x01 => { let (value, t) = self.op_ld_16(); self.regs.set_bc(value); t }
            0x11 => { let (value, t) = self.op_ld_16(); self.regs.set_de(value); t }
            0x21 => { let (value, t) = self.op_ld_16(); self.regs.set_hl(value); t }
            0x31 => { let (value, t) = self.op_ld_16(); self.sp = value; t }

            0x02 => { self.write(self.regs.bc(), self.regs.a); 8 }
            0x12 => { self.write(self.regs.de(), self.regs.a); 8 }
            0x22 => { self.write(self.regs.hl(), self.regs.a); self.regs.set_hl(self.regs.hl().wrapping_add(1)); 8 }
            0x32 => { self.write(self.regs.hl(), self.regs.a); self.regs.set_hl(self.regs.hl().wrapping_sub(1)); 8 }

            0x03 => { self.regs.set_bc(self.regs.bc().wrapping_add(1)); 8 }
            0x13 => { self.regs.set_de(self.regs.de().wrapping_add(1)); 8 }
            0x23 => { self.regs.set_hl(self.regs.hl().wrapping_add(1)); 8 }
            0x33 => { self.sp = self.sp.wrapping_add(1); 8 }

            0x04 => { let (value, timing) = self.op_inc(self.regs.b); self.regs.b = value; timing }
            0x14 => { let (value, timing) = self.op_inc(self.regs.d); self.regs.d = value; timing }
            0x24 => { let (value, timing) = self.op_inc(self.regs.h); self.regs.h = value; timing }
            0x34 => {
                let value = self.read(self.regs.hl());
                let value = self.op_inc(value).0;
                self.write(self.regs.hl(), value);
                12
            }

            0x05 => { let (value, timing) = self.op_dec(self.regs.b); self.regs.b = value; timing }
            0x15 => { let (value, timing) = self.op_dec(self.regs.d); self.regs.d = value; timing }
            0x25 => { let (value, timing) = self.op_dec(self.regs.h); self.regs.h = value; timing }
            0x35 => {
                let value = self.read(self.regs.hl());
                let value = self.op_dec(value).0;
                self.write(self.regs.hl(), value);
                12
            }

            0x06 => { self.regs.b = self.read_pc(); 8 }
            0x16 => { self.regs.d = self.read_pc(); 8 }
            0x26 => { self.regs.h = self.read_pc(); 8 }
            0x36 => { let value = self.read_pc(); self.write(self.regs.hl(), value); 12 }

            0x07 => { let (value, _) = self.op_rl(self.regs.a); self.regs.a = value; self.regs.f.zero = false; 4 }
            0x17 => { let (value, _) = self.op_rlc(self.regs.a); self.regs.a = value; self.regs.f.zero = false; 4 }
            0x27 => self.op_daa(),
            0x37 => { self.regs.f.add_sub = false; self.regs.f.half_carry = false; self.regs.f.carry = true; 4 }

            0x08 => {
                let low = self.read_pc();
                let high = self.read_pc();
                let address = bytes::assemble(high, low);

                let (high, low) = bytes::extract(self.sp);
                self.write(address, low);
                self.write(address.wrapping_add(1), high);
                20
            }
            0x18 => self.op_jr(true),
            0x28 => self.op_jr(self.regs.f.zero),
            0x38 => self.op_jr(self.regs.f.carry),

            0x09 => self.op_add_hl(self.regs.bc()),
            0x19 => self.op_add_hl(self.regs.de()),
            0x29 => self.op_add_hl(self.regs.hl()),
            0x39 => self.op_add_hl(self.sp),

            0x0a => { self.regs.a = self.read(self.regs.bc()); 8 }
            0x1a => { self.regs.a = self.read(self.regs.de()); 8 }
            0x2a => {
                self.regs.a = self.read(self.regs.hl()); self.regs.set_hl(self.regs.hl().wrapping_add(1)); 8
            }
            0x3a => { self.regs.a = self.read(self.regs.hl()); self.regs.set_hl(self.regs.hl().wrapping_sub(1)); 8 }

            0x0b => { self.regs.set_bc(self.regs.bc().wrapping_sub(1)); 8 }
            0x1b => { self.regs.set_de(self.regs.de().wrapping_sub(1)); 8 }
            0x2b => { self.regs.set_hl(self.regs.hl().wrapping_sub(1)); 8 }
            0x3b => { self.sp = self.sp.wrapping_sub(1); 8 }

            0x0c => { let (value, timing) = self.op_inc(self.regs.c); self.regs.c = value; timing }
            0x1c => { let (value, timing) = self.op_inc(self.regs.e); self.regs.e = value; timing }
            0x2c => { let (value, timing) = self.op_inc(self.regs.l); self.regs.l = value; timing }
            0x3c => { let (value, timing) = self.op_inc(self.regs.a); self.regs.a = value; timing }

            0x0d => { let (value, timing) = self.op_dec(self.regs.c); self.regs.c = value; timing }
            0x1d => { let (value, timing) = self.op_dec(self.regs.e); self.regs.e = value; timing }
            0x2d => { let (value, timing) = self.op_dec(self.regs.l); self.regs.l = value; timing }
            0x3d => { let (value, timing) = self.op_dec(self.regs.a); self.regs.a = value; timing }

            0x0e => { self.regs.c = self.read_pc(); 8 }
            0x1e => { self.regs.e = self.read_pc(); 8 }
            0x2e => { self.regs.l = self.read_pc(); 8 }
            0x3e => { self.regs.a = self.read_pc(); 8 }

            0x0f => { let (value, _) = self.op_rr(self.regs.a); self.regs.a = value; self.regs.f.zero = false; 4 }
            0x1f => { let (value, _) = self.op_rrc(self.regs.a); self.regs.a = value; self.regs.f.zero = false; 4 }
            0x2f => { self.regs.f.add_sub = true; self.regs.f.half_carry = true; self.regs.a = !self.regs.a; 4 }
            0x3f => { self.regs.f.add_sub = false; self.regs.f.half_carry = false; self.regs.f.carry = !self.regs.f.carry; 4 }

            0x40 => 4,
            0x41 => { self.regs.b = self.regs.c; 4 }
            0x42 => { self.regs.b = self.regs.d; 4 }
            0x43 => { self.regs.b = self.regs.e; 4 }
            0x44 => { self.regs.b = self.regs.h; 4 }
            0x45 => { self.regs.b = self.regs.l; 4 }
            0x46 => { self.regs.b = self.read(self.regs.hl()); 8 }
            0x47 => { self.regs.b = self.regs.a; 4 }

            0x48 => { self.regs.c = self.regs.b; 4 }
            0x49 => 4,
            0x4a => { self.regs.c = self.regs.d; 4 }
            0x4b => { self.regs.c = self.regs.e; 4 }
            0x4c => { self.regs.c = self.regs.h; 4 }
            0x4d => { self.regs.c = self.regs.l; 4 }
            0x4e => { self.regs.c = self.read(self.regs.hl()); 8 }
            0x4f => { self.regs.c = self.regs.a; 4 }

            0x50 => { self.regs.d = self.regs.b; 4 }
            0x51 => { self.regs.d = self.regs.c; 4 }
            0x52 => 4,
            0x53 => { self.regs.d = self.regs.e; 4 }
            0x54 => { self.regs.d = self.regs.h; 4 }
            0x55 => { self.regs.d = self.regs.l; 4 }
            0x56 => { self.regs.d = self.read(self.regs.hl()); 8 }
            0x57 => { self.regs.d = self.regs.a; 4 }

            0x58 => { self.regs.e = self.regs.b; 4 }
            0x59 => { self.regs.e = self.regs.c; 4 }
            0x5a => { self.regs.e = self.regs.d; 4 }
            0x5b => 4,
            0x5c => { self.regs.e = self.regs.h; 4 }
            0x5d => { self.regs.e = self.regs.l; 4 }
            0x5e => { self.regs.e = self.read(self.regs.hl()); 8 }
            0x5f => { self.regs.e = self.regs.a; 4 }

            0x60 => { self.regs.h = self.regs.b; 4 }
            0x61 => { self.regs.h = self.regs.c; 4 }
            0x62 => { self.regs.h = self.regs.d; 4 }
            0x63 => { self.regs.h = self.regs.e; 4 }
            0x64 => 4,
            0x65 => { self.regs.h = self.regs.l; 4 }
            0x66 => { self.regs.h = self.read(self.regs.hl()); 8 }
            0x67 => { self.regs.h = self.regs.a; 4 }

            0x68 => { self.regs.l = self.regs.b; 4 }
            0x69 => { self.regs.l = self.regs.c; 4 }
            0x6a => { self.regs.l = self.regs.d; 4 }
            0x6b => { self.regs.l = self.regs.e; 4 }
            0x6c => { self.regs.l = self.regs.h; 4 }
            0x6d => 4,
            0x6e => { self.regs.l = self.read(self.regs.hl()); 8 }
            0x6f => { self.regs.l = self.regs.a; 4 }

            0x70 => { self.write(self.regs.hl(), self.regs.b); 8 }
            0x71 => { self.write(self.regs.hl(), self.regs.c); 8 }
            0x72 => { self.write(self.regs.hl(), self.regs.d); 8 }
            0x73 => { self.write(self.regs.hl(), self.regs.e); 8 }
            0x74 => { self.write(self.regs.hl(), self.regs.h); 8 }
            0x75 => { self.write(self.regs.hl(), self.regs.l); 8 }

            0x76 => { self.halt = true; 4 }

            0x77 => { self.write(self.regs.hl(), self.regs.a); 8 }

            0x78 => { self.regs.a = self.regs.b; 4 }
            0x79 => { self.regs.a = self.regs.c; 4 }
            0x7a => { self.regs.a = self.regs.d; 4 }
            0x7b => { self.regs.a = self.regs.e; 4 }
            0x7c => { self.regs.a = self.regs.h; 4 }
            0x7d => { self.regs.a = self.regs.l; 4 }
            0x7e => { self.regs.a = self.read(self.regs.hl()); 8 }
            0x7f => 4,

            0x80 => self.op_add(self.regs.b),
            0x81 => self.op_add(self.regs.c),
            0x82 => self.op_add(self.regs.d),
            0x83 => self.op_add(self.regs.e),
            0x84 => self.op_add(self.regs.h),
            0x85 => self.op_add(self.regs.l),
            0x86 => { let value = self.read(self.regs.hl()); self.op_add(value) + 4 },
            0x87 => self.op_add(self.regs.a),

            0x88 => self.op_adc(self.regs.b),
            0x89 => self.op_adc(self.regs.c),
            0x8a => self.op_adc(self.regs.d),
            0x8b => self.op_adc(self.regs.e),
            0x8c => self.op_adc(self.regs.h),
            0x8d => self.op_adc(self.regs.l),
            0x8e => { let value = self.read(self.regs.hl()); self.op_adc(value) + 4 },
            0x8f => self.op_adc(self.regs.a),

            0x90 => self.op_sub(self.regs.b),
            0x91 => self.op_sub(self.regs.c),
            0x92 => self.op_sub(self.regs.d),
            0x93 => self.op_sub(self.regs.e),
            0x94 => self.op_sub(self.regs.h),
            0x95 => self.op_sub(self.regs.l),
            0x96 => { let value = self.read(self.regs.hl()); self.op_sub(value) + 4 },
            0x97 => self.op_sub(self.regs.a),

            0x98 => self.op_sbc(self.regs.b),
            0x99 => self.op_sbc(self.regs.c),
            0x9a => self.op_sbc(self.regs.d),
            0x9b => self.op_sbc(self.regs.e),
            0x9c => self.op_sbc(self.regs.h),
            0x9d => self.op_sbc(self.regs.l),
            0x9e => { let value = self.read(self.regs.hl()); self.op_sbc(value) + 4 },
            0x9f => self.op_sbc(self.regs.a),

            0xa0 => self.op_and(self.regs.b),
            0xa1 => self.op_and(self.regs.c),
            0xa2 => self.op_and(self.regs.d),
            0xa3 => self.op_and(self.regs.e),
            0xa4 => self.op_and(self.regs.h),
            0xa5 => self.op_and(self.regs.l),
            0xa6 => { let value = self.read(self.regs.hl()); self.op_and(value) + 4 },
            0xa7 => self.op_and(self.regs.a),

            0xa8 => self.op_xor(self.regs.b),
            0xa9 => self.op_xor(self.regs.c),
            0xaa => self.op_xor(self.regs.d),
            0xab => self.op_xor(self.regs.e),
            0xac => self.op_xor(self.regs.h),
            0xad => self.op_xor(self.regs.l),
            0xae => { let value = self.read(self.regs.hl()); self.op_xor(value) + 4 },
            0xaf => self.op_xor(self.regs.a),

            0xb0 => self.op_or(self.regs.b),
            0xb1 => self.op_or(self.regs.c),
            0xb2 => self.op_or(self.regs.d),
            0xb3 => self.op_or(self.regs.e),
            0xb4 => self.op_or(self.regs.h),
            0xb5 => self.op_or(self.regs.l),
            0xb6 => { let value = self.read(self.regs.hl()); self.op_or(value) + 4 },
            0xb7 => self.op_or(self.regs.a),

            0xb8 => self.op_cp(self.regs.b),
            0xb9 => self.op_cp(self.regs.c),
            0xba => self.op_cp(self.regs.d),
            0xbb => self.op_cp(self.regs.e),
            0xbc => self.op_cp(self.regs.h),
            0xbd => self.op_cp(self.regs.l),
            0xbe => { let value = self.read(self.regs.hl()); self.op_cp(value) + 4 },
            0xbf => self.op_cp(self.regs.a),

            0xc0 => self.op_ret(!self.regs.f.zero),
            0xd0 => self.op_ret(!self.regs.f.carry),
            0xe0 => { let word = self.read_pc() as u16; self.write(0xff00 + word, self.regs.a); 12 }
            0xf0 => { let word = self.read_pc() as u16; self.regs.a = self.read(0xff00 + word); 12 }

            0xc1 => { let (value, timing) = self.op_pop(); self.regs.set_bc(value); timing }
            0xd1 => { let (value, timing) = self.op_pop(); self.regs.set_de(value); timing }
            0xe1 => { let (value, timing) = self.op_pop(); self.regs.set_hl(value); timing }
            0xf1 => { let (value, timing) = self.op_pop(); self.regs.set_af(value); timing }

            0xc2 => self.op_jp(!self.regs.f.zero),
            0xd2 => self.op_jp(!self.regs.f.carry),
            0xe2 => { self.write(0xff00 + self.regs.c as u16, self.regs.a); 8 }
            0xf2 => { self.regs.a = self.read(0xff00 + self.regs.c as u16); 8 }

            0xc3 => self.op_jp(true),
            0xf3 => { self.interrupts.enabled = false; 4 }

            0xc4 => self.op_call(!self.regs.f.zero),
            0xd4 => self.op_call(!self.regs.f.carry),

            0xc5 => self.op_push(self.regs.bc()),
            0xd5 => self.op_push(self.regs.de()),
            0xe5 => self.op_push(self.regs.hl()),
            0xf5 => self.op_push(self.regs.af()),

            0xc6 => { let value = self.read_pc(); self.op_add(value) + 4 }
            0xd6 => { let value = self.read_pc(); self.op_sub(value) + 4 }
            0xe6 => { let value = self.read_pc(); self.op_and(value) + 4 }
            0xf6 => { let value = self.read_pc(); self.op_or(value) + 4 }

            0xc7 => self.op_rst(0x00),
            0xd7 => self.op_rst(0x10),
            0xe7 => self.op_rst(0x20),
            0xf7 => self.op_rst(0x30),

            0xc8 => self.op_ret(self.regs.f.zero),
            0xd8 => self.op_ret(self.regs.f.carry),
            0xe8 => self.op_add_sp(),
            0xf8 => { let prev = self.sp; self.op_add_sp(); self.regs.set_hl(self.sp); self.sp = prev; 12 }

            0xc9 => { self.op_ret(true); 16 }
            0xd9 => { self.op_ret(true); self.interrupts.enabled = true; 16 }
            0xe9 => { self.pc = self.regs.hl(); 4 }
            0xf9 => { self.sp = self.regs.hl(); 8 }

            0xca => self.op_jp(self.regs.f.zero),
            0xda => self.op_jp(self.regs.f.carry),
            0xea => self.op_write_16_data(),
            0xfa => self.op_load_16_data(),

            0xcb => { let op = self.read_pc(); self.handle_op_cb(op) }
            0xfb => { self.interrupts.enabled = true; 4 }

            0xcc => self.op_call(self.regs.f.zero),
            0xdc => self.op_call(self.regs.f.carry),

            0xcd => self.op_call(true),

            0xce => { let value = self.read_pc(); self.op_adc(value) + 4 }
            0xde => { let value = self.read_pc(); self.op_sbc(value) + 4 }
            0xee => { let value = self.read_pc(); self.op_xor(value) + 4 }
            0xfe => { let value = self.read_pc(); self.op_cp(value) + 4 }

            0xcf => self.op_rst(0x08),
            0xdf => self.op_rst(0x18),
            0xef => self.op_rst(0x28),
            0xff => self.op_rst(0x38),

            _ => unimplemented!("opcode {:x} not implemented", op),
        }
    }

    #[rustfmt::skip]
    fn handle_op_cb(&mut self, cb: u8) -> timer::Timing {
        match cb {
            0x00 => { let (value, timing) = self.op_rl(self.regs.b); self.regs.b = value; timing }
            0x01 => { let (value, timing) = self.op_rl(self.regs.c); self.regs.c = value; timing }
            0x02 => { let (value, timing) = self.op_rl(self.regs.d); self.regs.d = value; timing }
            0x03 => { let (value, timing) = self.op_rl(self.regs.e); self.regs.e = value; timing }
            0x04 => { let (value, timing) = self.op_rl(self.regs.h); self.regs.h = value; timing }
            0x05 => { let (value, timing) = self.op_rl(self.regs.l); self.regs.l = value; timing }
            0x06 => {
                let value = self.read(self.regs.hl());
                let value = self.op_rl(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x07 => { let (value, timing) = self.op_rl(self.regs.a); self.regs.a = value; timing }

            0x08 => { let (value, timing) = self.op_rr(self.regs.b); self.regs.b = value; timing }
            0x09 => { let (value, timing) = self.op_rr(self.regs.c); self.regs.c = value; timing }
            0x0a => { let (value, timing) = self.op_rr(self.regs.d); self.regs.d = value; timing }
            0x0b => { let (value, timing) = self.op_rr(self.regs.e); self.regs.e = value; timing }
            0x0c => { let (value, timing) = self.op_rr(self.regs.h); self.regs.h = value; timing }
            0x0d => { let (value, timing) = self.op_rr(self.regs.l); self.regs.l = value; timing }
            0x0e => {
                let value = self.read(self.regs.hl());
                let value = self.op_rr(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x0f => { let (value, timing) = self.op_rr(self.regs.a); self.regs.a = value; timing }

            0x10 => { let (value, timing) = self.op_rlc(self.regs.b); self.regs.b = value; timing }
            0x11 => { let (value, timing) = self.op_rlc(self.regs.c); self.regs.c = value; timing }
            0x12 => { let (value, timing) = self.op_rlc(self.regs.d); self.regs.d = value; timing }
            0x13 => { let (value, timing) = self.op_rlc(self.regs.e); self.regs.e = value; timing }
            0x14 => { let (value, timing) = self.op_rlc(self.regs.h); self.regs.h = value; timing }
            0x15 => { let (value, timing) = self.op_rlc(self.regs.l); self.regs.l = value; timing }
            0x16 => {
                let value = self.read(self.regs.hl());
                let value = self.op_rlc(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x17 => { let (value, timing) = self.op_rlc(self.regs.a); self.regs.a = value; timing }

            0x18 => { let (value, timing) = self.op_rrc(self.regs.b); self.regs.b = value; timing }
            0x19 => { let (value, timing) = self.op_rrc(self.regs.c); self.regs.c = value; timing }
            0x1a => { let (value, timing) = self.op_rrc(self.regs.d); self.regs.d = value; timing }
            0x1b => { let (value, timing) = self.op_rrc(self.regs.e); self.regs.e = value; timing }
            0x1c => { let (value, timing) = self.op_rrc(self.regs.h); self.regs.h = value; timing }
            0x1d => { let (value, timing) = self.op_rrc(self.regs.l); self.regs.l = value; timing }
            0x1e => {
                let value = self.read(self.regs.hl());
                let value = self.op_rrc(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x1f => { let (value, timing) = self.op_rrc(self.regs.a); self.regs.a = value; timing }

            0x20 => { let (value, timing) = self.op_sll(self.regs.b); self.regs.b = value; timing }
            0x21 => { let (value, timing) = self.op_sll(self.regs.c); self.regs.c = value; timing }
            0x22 => { let (value, timing) = self.op_sll(self.regs.d); self.regs.d = value; timing }
            0x23 => { let (value, timing) = self.op_sll(self.regs.e); self.regs.e = value; timing }
            0x24 => { let (value, timing) = self.op_sll(self.regs.h); self.regs.h = value; timing }
            0x25 => { let (value, timing) = self.op_sll(self.regs.l); self.regs.l = value; timing }
            0x26 => {
                let value = self.read(self.regs.hl());
                let value = self.op_sll(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x27 => { let (value, timing) = self.op_sll(self.regs.a); self.regs.a = value; timing }

            0x28 => { let (value, timing) = self.op_sr(self.regs.b); self.regs.b = value; timing }
            0x29 => { let (value, timing) = self.op_sr(self.regs.c); self.regs.c = value; timing }
            0x2a => { let (value, timing) = self.op_sr(self.regs.d); self.regs.d = value; timing }
            0x2b => { let (value, timing) = self.op_sr(self.regs.e); self.regs.e = value; timing }
            0x2c => { let (value, timing) = self.op_sr(self.regs.h); self.regs.h = value; timing }
            0x2d => { let (value, timing) = self.op_sr(self.regs.l); self.regs.l = value; timing }
            0x2e => {
                let value = self.read(self.regs.hl());
                let value = self.op_sr(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x2f => { let (value, timing) = self.op_sr(self.regs.a); self.regs.a = value; timing }

            0x30 => { let (value, timing) = self.op_swap(self.regs.b); self.regs.b = value; timing }
            0x31 => { let (value, timing) = self.op_swap(self.regs.c); self.regs.c = value; timing }
            0x32 => { let (value, timing) = self.op_swap(self.regs.d); self.regs.d = value; timing }
            0x33 => { let (value, timing) = self.op_swap(self.regs.e); self.regs.e = value; timing }
            0x34 => { let (value, timing) = self.op_swap(self.regs.h); self.regs.h = value; timing }
            0x35 => { let (value, timing) = self.op_swap(self.regs.l); self.regs.l = value; timing }
            0x36 => {
                let value = self.read(self.regs.hl());
                let value = self.op_swap(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x37 => { let (value, timing) = self.op_swap(self.regs.a); self.regs.a = value; timing }

            0x38 => { let (value, timing) = self.op_srl(self.regs.b); self.regs.b = value; timing }
            0x39 => { let (value, timing) = self.op_srl(self.regs.c); self.regs.c = value; timing }
            0x3a => { let (value, timing) = self.op_srl(self.regs.d); self.regs.d = value; timing }
            0x3b => { let (value, timing) = self.op_srl(self.regs.e); self.regs.e = value; timing }
            0x3c => { let (value, timing) = self.op_srl(self.regs.h); self.regs.h = value; timing }
            0x3d => { let (value, timing) = self.op_srl(self.regs.l); self.regs.l = value; timing }
            0x3e => {
                let value = self.read(self.regs.hl());
                let value = self.op_srl(value).0;
                self.write(self.regs.hl(), value);
                16
            }
            0x3f => { let (value, timing) = self.op_srl(self.regs.a); self.regs.a = value; timing }
            0x40..=0x7f => {
                let bit = (cb - 0x40) / 8;
                match cb - (0x40 + bit * 8) {
                    0 => self.op_bit(self.regs.b, bit),
                    1 => self.op_bit(self.regs.c, bit),
                    2 => self.op_bit(self.regs.d, bit),
                    3 => self.op_bit(self.regs.e, bit),
                    4 => self.op_bit(self.regs.h, bit),
                    5 => self.op_bit(self.regs.l, bit),
                    6 => {
                        let value = self.read(self.regs.hl());
                        self.op_bit(value, bit) + 4
                    }
                    7 => self.op_bit(self.regs.a, bit),
                    _ => unreachable!(),
                }
            }
            0x80..=0xbf => {
                let bit = (cb - 0x80) / 8;
                match cb - (0x80 + bit * 8) {
                    0 => { let (value, timing) = self.op_res(self.regs.b, bit); self.regs.b = value; timing }
                    1 => { let (value, timing) = self.op_res(self.regs.c, bit); self.regs.c = value; timing }
                    2 => { let (value, timing) = self.op_res(self.regs.d, bit); self.regs.d = value; timing }
                    3 => { let (value, timing) = self.op_res(self.regs.e, bit); self.regs.e = value; timing }
                    4 => { let (value, timing) = self.op_res(self.regs.h, bit); self.regs.h = value; timing }
                    5 => { let (value, timing) = self.op_res(self.regs.l, bit); self.regs.l = value; timing }
                    6 => {
                        let value = self.read(self.regs.hl());
                        let (value, _) = self.op_res(value, bit);
                        self.write(self.regs.hl(), value);
                        16
                    }
                    7 => { let (value, timing) = self.op_res(self.regs.a, bit); self.regs.a = value; timing }
                    _ => unreachable!(),
                }
            }
            0xc0..=0xff => {
                let bit = (cb - 0xc0) / 8;
                match cb - (0xc0 + bit * 8) {
                    0 => { let (value, timing) = self.op_set(self.regs.b, bit); self.regs.b = value; timing }
                    1 => { let (value, timing) = self.op_set(self.regs.c, bit); self.regs.c = value; timing }
                    2 => { let (value, timing) = self.op_set(self.regs.d, bit); self.regs.d = value; timing }
                    3 => { let (value, timing) = self.op_set(self.regs.e, bit); self.regs.e = value; timing }
                    4 => { let (value, timing) = self.op_set(self.regs.h, bit); self.regs.h = value; timing }
                    5 => { let (value, timing) = self.op_set(self.regs.l, bit); self.regs.l = value; timing }
                    6 => {
                        let value = self.read(self.regs.hl());
                        let (value, _) = self.op_set(value, bit);
                        self.write(self.regs.hl(), value);
                        16
                    }
                    7 => { let (value, timing) = self.op_set(self.regs.a, bit); self.regs.a = value; timing }
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_cpu(data: &[u8]) -> CPU {
        let mut cpu = CPU::new(Memory::new().with_bootrom(data), LCD::new(false));
        cpu.sp = 0xfffe;
        cpu.pc = 0;
        cpu
    }

    #[test]
    fn registers() {
        let mut r = Registers::new_boot();
        r.set_af(0x1230);
        r.set_bc(0x2345);
        r.set_de(0x3456);
        r.set_hl(0x4567);

        assert_eq!(r.a, 0x12);
        assert_eq!(u8::from(r.f), 0x30);
        assert_eq!(r.af(), 0x1230);

        assert_eq!(r.b, 0x23);
        assert_eq!(r.c, 0x45);
        assert_eq!(r.bc(), 0x2345);

        assert_eq!(r.d, 0x34);
        assert_eq!(r.e, 0x56);
        assert_eq!(r.de(), 0x3456);

        assert_eq!(r.h, 0x45);
        assert_eq!(r.l, 0x67);
        assert_eq!(r.hl(), 0x4567);
    }

    #[test]
    fn op_push_pop() {
        let mut cpu = new_cpu(&[]);
        cpu.sp = 0xfffe;
        cpu.op_push(0x0102);
        assert_eq!(cpu.sp, 0xfffc);
        assert_eq!(cpu.op_pop().0, 0x0102);
        assert_eq!(cpu.sp, 0xfffe);
    }

    #[test]
    fn op_jr() {
        let mut cpu = new_cpu(&[0x12, 0xfd]);
        assert_eq!(cpu.op_jr(false), 8);
        assert_eq!(cpu.pc, 0x0001);

        cpu.pc = 0;
        assert_eq!(cpu.op_jr(true), 12);
        assert_eq!(cpu.pc, 0x0013);

        cpu.pc = 0x01;
        cpu.op_jr(true);
        assert_eq!(cpu.pc, 0xffff);
    }

    #[test]
    fn op_ld_16() {
        let mut cpu = new_cpu(&[0x12, 0xfd]);
        let res = cpu.op_ld_16();
        assert_eq!(res.0, 0xfd12);
        assert_eq!(cpu.pc, 0x0002);
    }

    #[test]
    fn op_inc() {
        let mut cpu = new_cpu(&[]);

        assert_eq!(cpu.op_inc(0x01).0, 0x02);
        assert_eq!(cpu.regs.f.zero, false);
        assert_eq!(cpu.regs.f.add_sub, false);
        assert_eq!(cpu.regs.f.half_carry, false);

        assert_eq!(cpu.op_inc(0x0f).0, 0x10);
        assert_eq!(cpu.regs.f.zero, false);
        assert_eq!(cpu.regs.f.add_sub, false);
        assert_eq!(cpu.regs.f.half_carry, true);

        assert_eq!(cpu.op_inc(0xff).0, 0x00);
        assert_eq!(cpu.regs.f.zero, true);
        assert_eq!(cpu.regs.f.add_sub, false);
        assert_eq!(cpu.regs.f.half_carry, true);
    }

    #[test]
    fn op_dec() {
        let mut cpu = new_cpu(&[]);
        assert_eq!(cpu.op_dec(0x01).0, 0x00);
        assert_eq!(cpu.regs.f.zero, true);
        assert_eq!(cpu.regs.f.add_sub, true);
        assert_eq!(cpu.regs.f.half_carry, false);

        assert_eq!(cpu.op_dec(0x00).0, 0xff);
        assert_eq!(cpu.regs.f.zero, false);
        assert_eq!(cpu.regs.f.add_sub, true);
        assert_eq!(cpu.regs.f.half_carry, true);

        assert_eq!(cpu.op_dec(0xf0).0, 0xef);
        assert_eq!(cpu.regs.f.zero, false);
        assert_eq!(cpu.regs.f.add_sub, true);
        assert_eq!(cpu.regs.f.half_carry, true);
    }

    #[test]
    fn op_sp_hl() {
        let mut cpu = new_cpu(&[0xff]);
        cpu.sp = 0xffff;
        cpu.op_add_sp();
        assert_eq!(cpu.sp, 0xfffe);
        assert_eq!(cpu.regs.f.half_carry, true);
        assert_eq!(cpu.regs.f.carry, true);
    }
}

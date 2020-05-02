use crate::{memory::Memory, timer::Timing};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub struct LCD {
    regs: Registers,
    done_frame: bool,
    mode: Mode,

    enabled: bool,
    mode_timing: u16,
}

#[derive(Clone)]
enum Mode {
    HBlank,
    VBlank,
    OAM,
    VRAM,
}

#[derive(Default, Clone)]
pub struct Registers {
    pub lcdc: LCDC,
    pub stat: STAT,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub wy: u8,
    pub wx: u8,
    pub bgp: MonoPalette,
    pub obp0: MonoPalette,
    pub obp1: MonoPalette,
}

#[derive(Default, Clone)]
pub struct LCDC {
    display_enable: bool,
    window_tile_map_display_select: bool,
    window_display_enable: bool,
    bg_window_tile_data_select: bool,
    bg_tile_map_display_select: bool,
    obj_size: SpriteSize,
    obj_display_enable: bool,
    bg_display: bool,
}

#[derive(PartialEq, Clone)]
enum SpriteSize {
    Small,
    Large,
}

impl Default for SpriteSize {
    fn default() -> Self {
        Self::Small
    }
}

#[derive(Default, Clone)]
pub struct STAT {
    lyc_equals_lc: bool,
    mode_2_oam: bool,
    mode_1_vblank: bool,
    mode_0_hblank: bool,
    coincidence: bool,
    mode: StatMode,
}

#[derive(Clone)]
enum StatMode {
    HBlank,
    VBlank,
    OAM,
    Transfer,
}

impl Default for StatMode {
    fn default() -> Self {
        Self::HBlank
    }
}

#[derive(Default, Clone)]
pub struct MonoPalette {
    color3: GrayShades,
    color2: GrayShades,
    color1: GrayShades,
    color0: GrayShades,
}

#[derive(FromPrimitive, Clone)]
enum GrayShades {
    White = 0x00,
    LightGray = 0x01,
    DarkGray = 0x02,
    Black = 0x03,
}

impl Default for GrayShades {
    fn default() -> Self {
        Self::White
    }
}

impl LCD {
    pub fn new() -> Self {
        Self {
            regs: Registers::default(),
            done_frame: true,
            mode: Mode::HBlank,
            enabled: false,
            mode_timing: 0,
        }
    }

    pub fn regs(&self) -> Registers {
        self.regs.clone()
    }

    pub fn set_regs(&mut self, regs: Registers) {
        self.regs = regs;
    }

    pub fn advance(&mut self, mem: &mut Memory, timing: Timing) {
        self.done_frame = false;

        mem.set_oam_access(true);
        mem.set_vram_access(true);

        // TODO: reset interrupts
        // TODO: memory set OAM access to true
        // TODO: memory set VRAM access to true
        if !self.regs.lcdc.display_enable {
            if self.enabled {
                self.mode = Mode::HBlank;
                self.mode_timing = 0;
                self.enabled = false;
            }
            return;
        }
        if !self.enabled {
            self.mode = Mode::OAM;
            self.mode_timing = 0;
            self.enabled = true;
        }

        self.mode_timing += timing as u16;
        match self.mode {
            Mode::OAM => {
                if self.mode_timing >= 79 {
                    self.mode_timing -= 79;
                    self.mode = Mode::VRAM;
                }
            }
            Mode::VRAM => {
                if self.mode_timing >= 172 {
                    self.mode_timing -= 172;
                    self.mode = Mode::HBlank;
                    self.drawLine(mem, self.regs.ly);
                }
            }
            Mode::HBlank => {
                if self.mode_timing >= 205 {
                    self.mode_timing -= 205;
                    self.regs.ly += 1;
                    self.mode = if self.regs.ly >= 144 {
                        Mode::VBlank
                    } else {
                        Mode::OAM
                    }
                }
            }
            Mode::VBlank => {
                if self.mode_timing >= 4560 {
                    self.mode = Mode::OAM;
                    self.mode_timing -= 4560;
                    self.regs.ly = 0;
                } else {
                    let ly = (self.mode_timing / 456) + 144;
                    self.regs.ly = ly as u8;
                }
            }
        }

        mem.set_oam_access(true);
        mem.set_vram_access(true);
        if !self.enabled {
            return;
        }

        match self.mode {
            Mode::OAM => {
                mem.set_oam_access(false);
            }
            Mode::VRAM => {
                mem.set_oam_access(false);
                mem.set_vram_access(false);
            }
            _ => {}
        }
    }

    fn drawLine(&mut self, _mem: &mut Memory, ly: u8) {
        if ly >= 144 {
            return;
        }

        let bg_tile_data = if self.regs.lcdc.bg_window_tile_data_select {
            0x8000
        } else {
            0x9000
        };
        let bg_tile_map = if self.regs.lcdc.bg_tile_map_display_select {
            0x9c00
        } else {
            0x9800
        };

        let bgcolors = vec![0; 160];
        if self.regs.lcdc.bg_display {
            let y = self.regs.ly.overflowing_add(self.regs.scy).0 as u16;
            let last_tile_x: Option<u16> = None;

            for i in 0u8..160 {
                let x = i.overflowing_add(self.regs.scx).0 as u16;
                let (tile_x, tile_y) = (x / 8, y / 8);
                let (pixel_x, pixel_y) = (8 - x % 8 - 1, y % 8);

                if last_tile_x.is_none() || last_tile_x.unwrap() != tile_x {
                    let tile = 0; // TODO: read memory from [bg_tile_map + (tile_y * 32) + tile_x]
                                  // handle signed :x
                }
            }
        }
    }
}

impl From<u8> for LCDC {
    fn from(f: u8) -> LCDC {
        LCDC {
            display_enable: f & 0x80 != 0,
            window_tile_map_display_select: f & 0x40 != 0,
            window_display_enable: f & 0x20 != 0,
            bg_window_tile_data_select: f & 0x10 != 0,
            bg_tile_map_display_select: f & 0x08 != 0,
            obj_size: if f & 0x04 != 0 {
                SpriteSize::Large
            } else {
                SpriteSize::Small
            },
            obj_display_enable: f & 0x02 != 0,
            bg_display: f & 0x01 != 0,
        }
    }
}

impl From<LCDC> for u8 {
    fn from(lcdc: LCDC) -> u8 {
        (if lcdc.display_enable { 0x80 } else { 0 })
            | (if lcdc.window_tile_map_display_select {
                0x40
            } else {
                0
            })
            | (if lcdc.window_display_enable { 0x20 } else { 0 })
            | (if lcdc.bg_window_tile_data_select {
                0x10
            } else {
                0
            })
            | (if lcdc.bg_tile_map_display_select {
                0x08
            } else {
                0
            })
            | (if lcdc.obj_size == SpriteSize::Large {
                0x04
            } else {
                0
            })
            | (if lcdc.obj_display_enable { 0x02 } else { 0 })
            | (if lcdc.bg_display { 0x01 } else { 0 })
    }
}

impl From<u8> for STAT {
    fn from(f: u8) -> STAT {
        STAT {
            lyc_equals_lc: f & 0x40 != 0,
            mode_2_oam: f & 0x20 != 0,
            mode_1_vblank: f & 0x10 != 0,
            mode_0_hblank: f & 0x08 != 0,
            coincidence: f & 0x04 != 0,
            mode: match f & 0x03 {
                0x00 => StatMode::HBlank,
                0x01 => StatMode::VBlank,
                0x02 => StatMode::OAM,
                0x03 => StatMode::Transfer,
                _ => unreachable!(),
            },
        }
    }
}

impl From<STAT> for u8 {
    fn from(s: STAT) -> u8 {
        (if s.lyc_equals_lc { 0x40 } else { 0 })
            | (if s.mode_2_oam { 0x20 } else { 0 })
            | (if s.mode_1_vblank { 0x10 } else { 0 })
            | (if s.mode_0_hblank { 0x08 } else { 0 })
            | (if s.coincidence { 0x04 } else { 0 })
            | match s.mode {
                StatMode::HBlank => 0x00,
                StatMode::VBlank => 0x01,
                StatMode::OAM => 0x02,
                StatMode::Transfer => 0x03,
            }
    }
}

impl From<u8> for MonoPalette {
    fn from(f: u8) -> MonoPalette {
        MonoPalette {
            color3: FromPrimitive::from_u8((f & 0xc0) >> 6).unwrap(),
            color2: FromPrimitive::from_u8((f & 0x30) >> 4).unwrap(),
            color1: FromPrimitive::from_u8((f & 0x0c) >> 2).unwrap(),
            color0: FromPrimitive::from_u8(f & 0x03).unwrap(),
        }
    }
}

impl From<MonoPalette> for u8 {
    fn from(mp: MonoPalette) -> u8 {
        (mp.color3 as u8) << 6 | (mp.color2 as u8) << 4 | (mp.color1 as u8) << 2 | (mp.color0 as u8)
    }
}

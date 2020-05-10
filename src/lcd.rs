use crate::{cpu::Interrupts, memory::Memory, timer::Timing};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

pub const SCREEN_SIZE: (u8, u8) = (160, 144);

pub struct LCD {
    regs: Registers,
    done_frame: bool,

    enabled: bool,
    mode_timing: u16,

    screen: Vec<u8>,
}

#[derive(PartialEq, Copy, Clone)]
enum Mode {
    HBlank,
    VBlank,
    OAM,
    VRAM,
}

impl Default for Mode {
    fn default() -> Self {
        Self::OAM
    }
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

#[derive(PartialEq, Copy, Clone)]
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
    mode: Mode,
}

#[derive(Default, Copy, Clone)]
pub struct MonoPalette {
    color3: GrayShades,
    color2: GrayShades,
    color1: GrayShades,
    color0: GrayShades,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone)]
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

struct SpriteInfo {
    x: u8,
    y: u8,
    tile: u8,
    flags: u8,
}

impl SpriteInfo {
    fn from_memory(mem: &Memory, id: u8) -> Self {
        let id = id as u16;
        Self {
            y: mem.read(0xfe00 + id * 4),
            x: mem.read(0xfe00 + id * 4 + 1),
            tile: mem.read(0xfe00 + id * 4 + 2),
            flags: mem.read(0xfe00 + id * 4 + 3),
        }
    }
}

impl LCD {
    pub fn new() -> Self {
        Self {
            regs: Registers::default(),
            done_frame: true,
            enabled: false,
            mode_timing: 0,
            screen: vec![0xff; 4 * SCREEN_SIZE.0 as usize * SCREEN_SIZE.1 as usize],
        }
    }

    pub fn regs(&self) -> Registers {
        self.regs.clone()
    }

    pub fn set_regs(&mut self, regs: Registers) {
        self.regs = regs;
    }

    pub fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub fn done_frame(&self) -> bool {
        self.done_frame
    }

    pub fn advance(&mut self, mem: &mut Memory, interrupts: &mut Interrupts, timing: Timing) {
        self.done_frame = false;

        interrupts.flag &= !0x03;
        mem.set_oam_access(true);
        mem.set_vram_access(true);

        if !self.regs.lcdc.display_enable {
            if self.enabled {
                self.regs.ly = 0;
                self.set_mode(interrupts, Mode::HBlank);
                self.mode_timing = 0;
                self.enabled = false;
            }
            return;
        }
        if !self.enabled {
            self.set_mode(interrupts, Mode::OAM);
            self.mode_timing = 0;
            self.enabled = true;
        }

        self.mode_timing += timing as u16;
        match self.regs.stat.mode {
            Mode::OAM => {
                // Mode 2
                if self.mode_timing >= 80 {
                    self.mode_timing -= 80;
                    self.set_mode(interrupts, Mode::VRAM);
                }
            }
            Mode::VRAM => {
                // Mode 3
                if self.mode_timing >= 172 {
                    self.mode_timing -= 172;
                    self.set_mode(interrupts, Mode::HBlank);
                    self.draw_line(mem, self.regs.ly);
                }
            }
            Mode::HBlank => {
                // Mode 0
                if self.mode_timing >= 204 {
                    self.mode_timing -= 204;
                    self.regs.ly += 1;
                    self.set_mode(
                        interrupts,
                        if self.regs.ly >= SCREEN_SIZE.1 {
                            Mode::VBlank
                        } else {
                            Mode::OAM
                        },
                    )
                }
            }
            Mode::VBlank => {
                // Mode 1
                if self.mode_timing >= 4560 {
                    self.set_mode(interrupts, Mode::OAM);
                    self.mode_timing -= 4560;
                    self.regs.ly = 0;
                } else {
                    let ly = (self.mode_timing / 456) + SCREEN_SIZE.1 as u16;
                    self.regs.ly = ly as u8;
                }
            }
        }

        self.regs.stat.coincidence = self.regs.ly == self.regs.lyc;
        if self.regs.stat.coincidence && self.regs.stat.lyc_equals_lc {
            interrupts.flag |= 0x02;
        }

        mem.set_oam_access(true);
        mem.set_vram_access(true);

        match self.regs.stat.mode {
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

    fn set_mode(&mut self, interrupts: &mut Interrupts, mode: Mode) {
        if self.regs.stat.mode == mode {
            return;
        }
        self.regs.stat.mode = mode;
        if (mode == Mode::HBlank && self.regs.stat.mode_0_hblank)
            || (mode == Mode::VBlank && self.regs.stat.mode_1_vblank)
            || (mode == Mode::OAM && self.regs.stat.mode_2_oam)
        {
            interrupts.flag |= 0x02;
        }
        if mode == Mode::VBlank {
            interrupts.flag |= 0x01;
            self.done_frame = true;
        }
    }

    fn draw_line(&mut self, mem: &Memory, ly: u8) {
        if ly >= SCREEN_SIZE.1 {
            return;
        }

        // BG
        let unsigned = self.regs.lcdc.bg_window_tile_data_select;
        let bg_tile_data: u16 = if unsigned { 0x8000 } else { 0x9000 };
        let bg_tile_map = if self.regs.lcdc.bg_tile_map_display_select {
            0x9c00
        } else {
            0x9800
        };
        let mut bgcolors = vec![0; SCREEN_SIZE.0 as usize];
        if self.regs.lcdc.bg_display {
            let y = ly.wrapping_add(self.regs.scy);
            let mut last_tile_x: Option<u8> = None;
            let (mut bottom, mut top) = (0x00, 0x00);

            for i in 0..SCREEN_SIZE.0 {
                let x = i.wrapping_add(self.regs.scx);
                let (tile_x, tile_y) = (x / 8, y / 8);
                let (pixel_x, pixel_y) = (8 - (x % 8) - 1, y % 8);

                if last_tile_x.is_none() || last_tile_x.unwrap() != tile_x {
                    let tile = mem.read(bg_tile_map + (tile_y as u16 * 32) + tile_x as u16);
                    if !unsigned {
                        let address = (bg_tile_data as i16)
                            .wrapping_add(tile as i8 as i16 * 16)
                            .wrapping_add(pixel_y as i8 as i16 * 2)
                            as u16;
                        bottom = mem.read(address);
                        top = mem.read(address + 1);
                    } else {
                        let address = bg_tile_data
                            .wrapping_add(tile as u16 * 16)
                            .wrapping_add(pixel_y as u16 * 2);
                        bottom = mem.read(address);
                        top = mem.read(address + 1);
                    }
                    last_tile_x = Some(tile_x);
                }

                let color = LCD::color_number(pixel_x as u8, top, bottom);
                bgcolors[i as usize] = color;
                let pixel = self.regs.bgp.color(color);
                self.set_pixel(i, ly, pixel);
            }
        } else {
            for i in 0..SCREEN_SIZE.0 {
                self.set_pixel(i, ly as u8, 0xff);
            }
        }

        // Window
        let win_tile_map = if self.regs.lcdc.window_tile_map_display_select {
            0x9c00
        } else {
            0x9800
        };
        if self.regs.lcdc.window_display_enable && self.regs.wx <= 166 && self.regs.wy <= 143 {
            let y = ly.wrapping_sub(self.regs.wy);
            let (mut bottom, mut top) = (0x00, 0x00);
            let mut last_tile_x: Option<u8> = None;
            for i in self.regs.wx.wrapping_sub(7)..SCREEN_SIZE.0 {
                let x = i.wrapping_sub(self.regs.wx).wrapping_add(7);
                let (tile_x, tile_y) = (x / 8, y / 8);
                let (pixel_x, pixel_y) = (8 - (x % 8) - 1, y % 8);

                if last_tile_x.is_none() || last_tile_x.unwrap() != tile_x {
                    let tile = mem.read(win_tile_map + (tile_y as u16 * 32) + tile_x as u16);
                    let address = (0x8800u16 as i16)
                        .wrapping_add(tile as i8 as i16 * 16)
                        .wrapping_add(pixel_y as i8 as i16 * 2)
                        as u16;
                    bottom = mem.read(address);
                    top = mem.read(address + 1);
                    last_tile_x = Some(tile_x);
                }

                let color = LCD::color_number(pixel_x as u8, top, bottom);
                bgcolors[i as usize] = color;
                let pixel = self.regs.bgp.color(color);
                self.set_pixel(i, ly, pixel);
            }
        }

        // Sprites
        let sprites = LCD::get_sprites(&mem, ly, self.regs.lcdc.obj_size);
        let count = match self.regs.lcdc.obj_size {
            SpriteSize::Large => 2,
            SpriteSize::Small => 1,
        };
        for info in sprites.iter().rev() {
            let mut pixel_y = ly.wrapping_sub(info.y).wrapping_add(16);
            let obp = if info.flags & 0x10 != 0 {
                self.regs.obp1
            } else {
                self.regs.obp0
            };
            let (reverse_x, reverse_y, behind) = (
                info.flags & 0x20 != 0,
                info.flags & 0x40 != 0,
                info.flags & 0x80 != 0,
            );
            for i in 0..count {
                let mut sprite_tile = info.tile;
                if self.regs.lcdc.obj_size == SpriteSize::Large {
                    if i == 0 {
                        sprite_tile = info.tile & 0xfe;
                    } else {
                        sprite_tile = info.tile | 0x01;
                        pixel_y -= 8;
                    }
                }

                if reverse_y {
                    pixel_y = 8u8.wrapping_sub(pixel_y).wrapping_sub(1);
                    if self.regs.lcdc.obj_size == SpriteSize::Large {
                        pixel_y = if i == 1 {
                            pixel_y.wrapping_sub(8)
                        } else {
                            pixel_y.wrapping_add(8)
                        };
                    }
                }

                let address = 0x8000u16.wrapping_add((sprite_tile as u16).wrapping_mul(16) as u16)
                    + pixel_y as u16 * 2;
                let bottom = mem.read(address);
                let top = mem.read(address + 1);
                for x in (0..8).filter(|&x| info.x.wrapping_add(x).wrapping_sub(8) < SCREEN_SIZE.0)
                {
                    let mut pixel_x = 8u8.wrapping_sub(x % 8).wrapping_sub(1);
                    if reverse_x {
                        pixel_x = 8u8.wrapping_sub(pixel_x).wrapping_sub(1);
                    }
                    let color = LCD::color_number(pixel_x as u8, top, bottom);
                    if color != 0x00
                        && !(behind
                            && bgcolors[info.x.wrapping_add(x).wrapping_sub(8) as usize] > 0)
                    {
                        let pixel = obp.color(color);
                        self.set_pixel(info.x.wrapping_add(x).wrapping_sub(8), ly, pixel);
                    }
                }
            }
        }
    }

    fn set_pixel(&mut self, x: u8, y: u8, pixel: u8) {
        let (x, y, width) = (x as usize, y as usize, SCREEN_SIZE.0 as usize);
        // write in rgba, don't touch a
        for i in 0..3 {
            if x + i >= width {
                continue;
            }
            self.screen[(y * width * 4) + (x * 4) + i] = pixel;
        }
    }

    fn get_sprites(mem: &Memory, ly: u8, size: SpriteSize) -> Vec<SpriteInfo> {
        let size = if size == SpriteSize::Large { 0 } else { 8 };
        let mut sprites: Vec<SpriteInfo> = (0..40)
            .map(|i| SpriteInfo::from_memory(mem, i))
            .filter(|info| {
                !(info.y == 0
                    || info.y >= SCREEN_SIZE.0
                    || ly < info.y.wrapping_sub(16)
                    || ly >= info.y.wrapping_sub(size))
            })
            .collect();
        sprites.sort_by(|left, right| left.x.partial_cmp(&right.x).unwrap());
        sprites.truncate(10);
        sprites
    }

    fn color_number(bit: u8, top: u8, bottom: u8) -> u8 {
        (((top >> bit) & 1) << 1) | (bottom >> bit) & 1
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

impl STAT {
    pub fn update(&mut self, f: u8) {
        self.lyc_equals_lc = f & 0x40 != 0;
        self.mode_2_oam = f & 0x20 != 0;
        self.mode_1_vblank = f & 0x10 != 0;
        self.mode_0_hblank = f & 0x08 != 0;
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
                Mode::HBlank => 0x00,
                Mode::VBlank => 0x01,
                Mode::OAM => 0x02,
                Mode::VRAM => 0x03,
            }
    }
}

impl MonoPalette {
    fn color(self, color: u8) -> u8 {
        let color = match color & 0x03 {
            0x00 => ToPrimitive::to_u8(&self.color0).unwrap(),
            0x01 => ToPrimitive::to_u8(&self.color1).unwrap(),
            0x02 => ToPrimitive::to_u8(&self.color2).unwrap(),
            0x03 => ToPrimitive::to_u8(&self.color3).unwrap(),
            _ => unreachable!(),
        };
        match color & 0x03 {
            0x00 => 255,
            0x01 => 170,
            0x02 => 85,
            0x03 => 0,
            _ => unreachable!(),
        }
    }
}

impl From<u8> for MonoPalette {
    fn from(f: u8) -> MonoPalette {
        MonoPalette {
            color3: FromPrimitive::from_u8((f >> 6) & 0x03).unwrap(),
            color2: FromPrimitive::from_u8((f >> 4) & 0x03).unwrap(),
            color1: FromPrimitive::from_u8((f >> 2) & 0x03).unwrap(),
            color0: FromPrimitive::from_u8(f & 0x03).unwrap(),
        }
    }
}

impl From<MonoPalette> for u8 {
    fn from(mp: MonoPalette) -> u8 {
        (mp.color3 as u8) << 6 | (mp.color2 as u8) << 4 | (mp.color1 as u8) << 2 | (mp.color0 as u8)
    }
}

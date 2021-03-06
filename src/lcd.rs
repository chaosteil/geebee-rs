use crate::bytes;
use crate::cart::GBType;
use crate::{cpu::Interrupts, memory::Memory, timer::Timing};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

pub const SCREEN_SIZE: (u8, u8) = (160, 144);

pub struct LCD {
    regs: Registers,
    done_frame: bool,
    gb: GBType,

    enabled: bool,
    mode_timing: u16,

    vram_access: bool,
    video: Vec<u8>,
    video_bank: u8,

    oam_access: bool,
    oam: [u8; 0xa0],

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
struct Registers {
    lcdc: LCDC,
    stat: STAT,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    wy: u8,
    wx: u8,
    bgp: MonoPalette,
    obp0: MonoPalette,
    obp1: MonoPalette,
    bgpi: u8,
    bgpd: Vec<u8>,
    obpi: u8,
    obpd: Vec<u8>,
    dma_source: u16,
    dma_dest: u16,
    hdma_type: HDMA,
    hdma_transfer: u8,
}

#[derive(Debug, Copy, Clone)]
enum HDMA {
    None,
    GDMA,
    HDMA,
}

impl Default for HDMA {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default, Clone)]
struct LCDC {
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
struct STAT {
    lyc_equals_lc: bool,
    mode_2_oam: bool,
    mode_1_vblank: bool,
    mode_0_hblank: bool,
    coincidence: bool,
    mode: Mode,
}

impl LCD {
    pub fn new(gb: GBType) -> Self {
        Self {
            regs: Registers {
                bgpd: vec![0xff; 0x40],
                obpd: vec![0xff; 0x40],
                ..Default::default()
            },
            done_frame: false,
            gb,
            enabled: false,
            mode_timing: 0,
            vram_access: true,
            video: vec![0x00; 0x4000],
            video_bank: 0,
            oam_access: true,
            oam: [0; 0xa0],
            screen: vec![0xff; 4 * SCREEN_SIZE.0 as usize * SCREEN_SIZE.1 as usize],
        }
    }

    pub fn screen(&self) -> &[u8] {
        &self.screen
    }

    // Prints out a 32*32 map of tiles in VRAM using the regular background palette.
    #[allow(dead_code)]
    pub fn tiles(&self) -> Vec<u8> {
        let (width, height) = (32, 32);
        let mut data = vec![0xff; width * height * 8 * 8 * 4];
        for tile_y in 0..height {
            for tile_x in 0..width {
                for pixel_y in 0..8 {
                    let address = ((tile_y * width + tile_x) as u16 * 16)
                        .wrapping_add(pixel_y as u16 * 2)
                        as usize;
                    for pixel_x in 0..8 {
                        let (bottom, top) = (self.video[address], self.video[address + 1]);
                        let color = LCD::color_number(pixel_x as u8, top, bottom);
                        let pixel = self.regs.bgp.color(color);
                        for i in 0..3 {
                            data[(tile_y * 8 + pixel_y) * width * 8 * 4
                                + (tile_x * 8 + 7 - pixel_x) * 4
                                + i] = pixel.rgb[i];
                        }
                    }
                }
            }
        }
        data
    }

    pub fn done_frame(&self) -> bool {
        self.done_frame
    }

    pub fn handle_read(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9fff => {
                if self.vram_access {
                    let address = (0x2000 * self.video_bank as u16) + (address - 0x8000);
                    self.video[address as usize]
                } else {
                    0x00
                }
            }
            0xfe00..=0xfe9f => {
                if self.oam_access {
                    self.oam[address as usize - 0xfe00]
                } else {
                    0x00
                }
            }
            0xff40 => self.regs.lcdc.clone().into(),
            0xff41 => self.regs.stat.clone().into(),
            0xff42 => self.regs.scy,
            0xff43 => self.regs.scx,
            0xff44 => self.regs.ly,
            0xff45 => self.regs.lyc,
            0xff46 => 0xff,
            0xff47 => self.regs.bgp.into(),
            0xff48 => self.regs.obp0.into(),
            0xff49 => self.regs.obp1.into(),
            0xff4a => self.regs.wy,
            0xff4b => self.regs.wx,
            0xff4f => self.video_bank,
            0xff51 => ((self.regs.dma_source & 0xf0) >> 8) as u8,
            0xff52 => (self.regs.dma_source & 0x0f) as u8,
            0xff53 => ((self.regs.dma_dest & 0xf0) >> 8) as u8,
            0xff54 => (self.regs.dma_dest & 0x0f) as u8,
            0xff55 => self.regs.hdma_transfer,
            0xff68 => self.regs.bgpi,
            0xff69 => self.regs.bgpd[(self.regs.bgpi & 0x3f) as usize],
            0xff6a => self.regs.obpi,
            0xff6b => self.regs.obpd[(self.regs.obpi & 0x3f) as usize],
            _ => unreachable!(),
        }
    }

    pub fn handle_write(&mut self, mem: &mut Memory, address: u16, value: u8) {
        match address {
            0x8000..=0x9fff => {
                if self.vram_access {
                    let address = (0x2000 * self.video_bank as u16) + (address - 0x8000);
                    self.video[address as usize] = value;
                }
            }
            0xfe00..=0xfe9f => {
                if self.oam_access {
                    self.oam[address as usize - 0xfe00] = value
                }
            }
            0xff40 => self.regs.lcdc = value.into(),
            0xff41 => self.regs.stat.update(value),
            0xff42 => self.regs.scy = value,
            0xff43 => self.regs.scx = value,
            0xff44 => {}
            0xff45 => self.regs.lyc = value,
            0xff46 => self.dma(mem, value),
            0xff47 => self.regs.bgp = value.into(),
            0xff48 => self.regs.obp0 = value.into(),
            0xff49 => self.regs.obp1 = value.into(),
            0xff4a => self.regs.wy = value,
            0xff4b => self.regs.wx = value,
            0xff4f => self.video_bank = value & 0x01,
            0xff51 => self.regs.dma_source = ((value as u16) << 8) | (self.regs.dma_source & 0x0f),
            0xff52 => self.regs.dma_source = (value as u16) | (self.regs.dma_source & 0xf0),
            0xff53 => self.regs.dma_dest = ((value as u16) << 8) | (self.regs.dma_dest & 0x0f),
            0xff54 => self.regs.dma_dest = (value as u16) | (self.regs.dma_dest & 0xf0),
            0xff55 => self.start_hdma_transfer(value),
            0xff68 => self.regs.bgpi = value & 0xbf,
            0xff69 => {
                self.regs.bgpd[(self.regs.bgpi & 0x3f) as usize] = value;
                if self.regs.bgpi & 0x80 != 0 {
                    self.regs.bgpi = (self.regs.bgpi & 0x3f).wrapping_add(1) | 0x80;
                }
            }
            0xff6a => self.regs.obpi = value & 0xbf,
            0xff6b => {
                self.regs.obpd[(self.regs.obpi & 0x3f) as usize] = value;
                if self.regs.obpi & 0x80 != 0 {
                    self.regs.obpi = (self.regs.obpi & 0x3f).wrapping_add(1) | 0x80;
                }
            }
            _ => panic!("unreachable with {:04x}", address),
        }
    }

    fn dma(&mut self, mem: &mut Memory, value: u8) {
        let start = (value as u16) << 8;
        let end = ((value as u16) << 8) | 0x009f;
        for dest in start..=end {
            let v = mem.read(dest);
            self.handle_write(mem, 0xfe00 | (dest & 0x00ff), v);
        }
    }

    fn start_hdma_transfer(&mut self, value: u8) {
        if let HDMA::HDMA = self.regs.hdma_type {
            if value & 0x80 == 0 {
                self.regs.hdma_transfer |= 0x80;
                self.regs.hdma_type = HDMA::None;
            }
            return;
        }
        self.regs.dma_source &= 0xfff0;
        self.regs.dma_dest &= 0x1ff0;
        self.regs.hdma_type = if value & 0x80 != 0 {
            HDMA::HDMA
        } else {
            HDMA::GDMA
        };
        self.regs.hdma_transfer = value & 0x7f;
    }

    fn hdma_transfer(&mut self, mem: &mut Memory) {
        match self.regs.hdma_type {
            HDMA::GDMA => {
                while self.regs.hdma_transfer != 0xff {
                    self.hdma_transfer_block(mem);
                }
            }
            HDMA::HDMA => {
                self.hdma_transfer_block(mem);
            }
            _ => {}
        }
    }

    fn hdma_transfer_block(&mut self, mem: &mut Memory) {
        for i in 0..0x10 {
            let start = self.regs.dma_source & 0xfff0;
            let end = (self.regs.dma_dest & 0x1ff0) | 0x8000;
            let v = mem.read(start + i);
            self.handle_write(mem, end + i, v);
        }
        self.regs.dma_source += 0x10;
        self.regs.dma_dest += 0x10;
        self.regs.hdma_transfer = self.regs.hdma_transfer.wrapping_sub(1);
        if self.regs.hdma_transfer == 0xff {
            self.regs.hdma_type = HDMA::None;
        }
    }

    pub fn advance(&mut self, interrupts: &mut Interrupts, mem: &mut Memory, timing: Timing) {
        self.done_frame = false;

        self.oam_access = true;
        self.vram_access = true;

        if let HDMA::GDMA = self.regs.hdma_type {
            self.hdma_transfer(mem);
        }

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
                    self.draw_line(self.regs.ly);
                    self.hdma_transfer(mem);
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
                    self.done_frame = true;
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

        match self.regs.stat.mode {
            Mode::OAM => {
                self.oam_access = false;
                self.vram_access = true;
            }
            Mode::VRAM => {
                self.oam_access = false;
                self.vram_access = false;
            }
            _ => {
                self.oam_access = true;
                self.vram_access = true;
            }
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
        }
    }

    fn draw_line(&mut self, ly: u8) {
        if ly >= SCREEN_SIZE.1 {
            return;
        }

        let (bgcolors, priority) = self.draw_bg(ly);
        self.draw_sprites(ly, &bgcolors, &priority);
    }

    fn draw_bg(&mut self, ly: u8) -> (Vec<u8>, Vec<u8>) {
        let unsigned = self.regs.lcdc.bg_window_tile_data_select;
        let mut bgcolors = vec![0; SCREEN_SIZE.0 as usize];
        let mut priority = vec![0; SCREEN_SIZE.0 as usize];
        if self.regs.lcdc.bg_display
            || (self.regs.lcdc.window_display_enable && self.regs.wx <= 166 && self.regs.wy <= ly)
        {
            for i in 0..SCREEN_SIZE.0 {
                let show_window = self.regs.lcdc.window_display_enable
                    && self.regs.wx.wrapping_sub(7) <= i
                    && self.regs.wy <= ly;
                let (x, y, select_tile_map) = if show_window {
                    (
                        i.wrapping_sub(self.regs.wx).wrapping_add(7),
                        ly.wrapping_sub(self.regs.wy),
                        self.regs.lcdc.window_tile_map_display_select,
                    )
                } else {
                    (
                        i.wrapping_add(self.regs.scx),
                        ly.wrapping_add(self.regs.scy),
                        self.regs.lcdc.bg_tile_map_display_select,
                    )
                };
                let tile_map = if select_tile_map { 0x1c00 } else { 0x1800 };
                let tile_data: u16 = if unsigned { 0x0000 } else { 0x1000 };
                let (tile_x, tile_y) = (x / 8, y / 8);
                let tile_address = (tile_map + (tile_y as u16 * 32) + tile_x as u16) as usize;
                let tile = self.video[tile_address];
                let tile_info = if let GBType::CGB(_) = self.gb {
                    self.video[tile_address + 0x2000].into()
                } else {
                    BGMapAttributes::default()
                };
                let (pixel_x, pixel_y) = (
                    if !tile_info.reverse_x {
                        7 - (x % 8)
                    } else {
                        x % 8
                    },
                    if !tile_info.reverse_y {
                        y % 8
                    } else {
                        7 - (y % 8)
                    },
                );
                let address = if !unsigned {
                    (tile_data as i16)
                        .wrapping_add(tile as i8 as i16 * 16)
                        .wrapping_add(pixel_y as i8 as i16 * 2) as u16 as usize
                } else {
                    tile_data
                        .wrapping_add(tile as u16 * 16)
                        .wrapping_add(pixel_y as u16 * 2) as usize
                };
                let (bottom, top) = (
                    self.video[address + (0x2000 * tile_info.bank)],
                    self.video[address + 1 + (0x2000 * tile_info.bank)],
                );
                let (pixel, color) = if let GBType::CGB(_) = self.gb {
                    if !show_window {
                        priority[i as usize] = if tile_info.priority { 0x01 } else { 0x00 };
                    }
                    let palette = self.read_palette(&self.regs.bgpd, tile_info.palette);
                    let color = LCD::color_number(pixel_x as u8, top, bottom);
                    (palette.color(color), color)
                } else {
                    let color = LCD::color_number(pixel_x as u8, top, bottom);
                    (self.regs.bgp.color(color), color)
                };
                if !show_window {
                    bgcolors[i as usize] = color;
                }
                self.set_pixel(i, ly, pixel);
            }
        } else {
            for i in 0..SCREEN_SIZE.0 {
                self.set_pixel(i, ly, Color::new(0xff, 0xff, 0xff));
            }
        }
        (bgcolors, priority)
    }

    fn draw_sprites(&mut self, ly: u8, bgcolors: &[u8], priority: &[u8]) {
        let sprites = self.get_sprites(ly, self.regs.lcdc.obj_size);
        let size = match self.regs.lcdc.obj_size {
            SpriteSize::Large => 16,
            SpriteSize::Small => 8,
        };
        for info in sprites.iter().rev() {
            let (sprite_x, sprite_y) = (info.x as u16 as i16 - 8, info.y as u16 as i16 - 16);
            let tile_y = if info.flags.reverse_y {
                (size - 1 - (ly as i16 - sprite_y)) as u16
            } else {
                (ly as i16 - sprite_y) as u16
            } as u8;
            let address = ((info.tile as u16) * 16
                + tile_y.wrapping_mul(2) as u16
                + if let GBType::CGB(_) = self.gb {
                    0x2000 * (info.flags.bank as u16)
                } else {
                    0
                }) as usize;
            let (bottom, top) = (self.video[address], self.video[address + 1]);
            for x in (0..8).filter(|&x| {
                sprite_x.wrapping_add(x) < SCREEN_SIZE.0 as u16 as i16
                    && sprite_x.wrapping_add(x) >= 0
            }) {
                let pixel_x = if info.flags.reverse_x { x } else { 7 - x };
                let color = LCD::color_number(pixel_x as u8, top, bottom);
                let screen_x = sprite_x.wrapping_add(x) as usize;
                if color == 0x00
                    || (info.flags.priority
                        && (bgcolors[screen_x] > 0 || priority[screen_x] > 0)
                        && !self.regs.lcdc.bg_display)
                {
                    continue;
                }
                let pixel = if let GBType::CGB(_) = self.gb {
                    let palette = self.read_palette(&self.regs.obpd, info.flags.color_palette);
                    palette.color(color)
                } else {
                    let obp = if info.flags.palette == 1 {
                        self.regs.obp1
                    } else {
                        self.regs.obp0
                    };
                    obp.color(color)
                };
                self.set_pixel(screen_x as u8, ly, pixel);
            }
        }
    }

    fn set_pixel(&mut self, x: u8, y: u8, c: Color) {
        let (x, y, width) = (x as usize, y as usize, SCREEN_SIZE.0 as usize);
        for i in 0..3 {
            if x + i < width {
                self.screen[(y * width * 4) + (x * 4) + i] = c.rgb[i];
            }
        }
    }

    fn get_sprites(&mut self, ly: u8, size: SpriteSize) -> Vec<SpriteInfo> {
        let sprite_size = if size == SpriteSize::Large { 16 } else { 8 };
        let mut sprites: Vec<SpriteInfo> = (0..40)
            .map(|i| SpriteInfo::from_memory(self, i, size))
            .filter(|info| {
                ((info.x as i16).wrapping_sub(8) >= -8 && (info.x as i16).wrapping_sub(8) < 160)
                    && ly >= info.y.wrapping_sub(16)
                    && ly < info.y.wrapping_sub(16).wrapping_add(sprite_size)
            })
            .collect();
        sprites.sort_by(|left, right| left.x.partial_cmp(&right.x).unwrap());
        sprites.truncate(10);
        sprites
    }

    fn color_number(bit: u8, top: u8, bottom: u8) -> u8 {
        (((top >> bit) & 1) << 1) | (bottom >> bit) & 1
    }

    fn read_palette(&self, pd: &[u8], index: u8) -> ColorPalette {
        let index = (index & 0x3f) as usize;

        let mut colors = [0; 4];
        for i in 0..4 {
            let low = pd[index * 8 + i * 2];
            let high = pd[index * 8 + i * 2 + 1];
            colors[i] = bytes::assemble(high, low);
        }
        ColorPalette::from_u16(colors[0], colors[1], colors[2], colors[3])
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

trait Palette {
    fn color(&self, color: u8) -> Color;
}

#[derive(Default, Copy, Clone)]
struct MonoPalette {
    color: [GrayShades; 4],
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

impl Palette for MonoPalette {
    fn color(&self, color: u8) -> Color {
        let color = ToPrimitive::to_u8(&self.color[(color & 0x03) as usize]).unwrap();
        match color & 0x03 {
            0x00 => Color::new(255, 255, 255),
            0x01 => Color::new(170, 170, 170),
            0x02 => Color::new(85, 85, 85),
            0x03 => Color::new(0, 0, 0),
            _ => unreachable!(),
        }
    }
}

impl From<u8> for MonoPalette {
    fn from(f: u8) -> MonoPalette {
        MonoPalette {
            color: [
                FromPrimitive::from_u8(f & 0x03).unwrap(),
                FromPrimitive::from_u8((f >> 2) & 0x03).unwrap(),
                FromPrimitive::from_u8((f >> 4) & 0x03).unwrap(),
                FromPrimitive::from_u8((f >> 6) & 0x03).unwrap(),
            ],
        }
    }
}

impl From<MonoPalette> for u8 {
    fn from(mp: MonoPalette) -> u8 {
        (mp.color[3] as u8) << 6
            | (mp.color[2] as u8) << 4
            | (mp.color[1] as u8) << 2
            | (mp.color[0] as u8)
    }
}

#[derive(Copy, Clone)]
struct Color {
    rgb: [u8; 3],
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Color {
        Color { rgb: [r, g, b] }
    }
}

impl From<u16> for Color {
    fn from(color: u16) -> Color {
        Color::new(
            ((color & 0x001f) * 8) as u8,
            (((color & 0x03e0) >> 5) * 8) as u8,
            (((color & 0x7c00) >> 10) * 8) as u8,
        )
    }
}

struct ColorPalette {
    color: [Color; 4],
}

impl Palette for ColorPalette {
    fn color(&self, color: u8) -> Color {
        self.color[(color & 0x03) as usize]
    }
}

impl ColorPalette {
    fn from_u16(color0: u16, color1: u16, color2: u16, color3: u16) -> Self {
        Self {
            color: [color0.into(), color1.into(), color2.into(), color3.into()],
        }
    }
}

struct SpriteInfo {
    x: u8,
    y: u8,
    tile: u8,
    flags: SpriteAttributes,
}

struct SpriteAttributes {
    priority: bool,
    reverse_y: bool,
    reverse_x: bool,
    palette: u8,
    bank: usize,
    color_palette: u8,
}

impl From<u8> for SpriteAttributes {
    fn from(f: u8) -> SpriteAttributes {
        SpriteAttributes {
            priority: f & 0x80 != 0,
            reverse_y: f & 0x40 != 0,
            reverse_x: f & 0x20 != 0,
            palette: (f & 0x10) >> 4,
            bank: ((f & 0x08) >> 3) as usize,
            color_palette: f & 0x07,
        }
    }
}

impl SpriteInfo {
    fn from_memory(lcd: &LCD, id: u8, size: SpriteSize) -> Self {
        let id = id as u16;
        Self {
            y: lcd.handle_read(0xfe00 + id * 4),
            x: lcd.handle_read(0xfe00 + id * 4 + 1),
            tile: lcd.handle_read(0xfe00 + id * 4 + 2)
                & match size {
                    SpriteSize::Large => 0xfe,
                    SpriteSize::Small => 0xff,
                },
            flags: lcd.handle_read(0xfe00 + id * 4 + 3).into(),
        }
    }
}

#[derive(Default)]
struct BGMapAttributes {
    palette: u8,
    bank: usize,
    reverse_x: bool,
    reverse_y: bool,
    priority: bool,
}

impl From<u8> for BGMapAttributes {
    fn from(f: u8) -> BGMapAttributes {
        BGMapAttributes {
            palette: f & 0x07,
            bank: ((f & 0x08) >> 3) as usize,
            reverse_x: (f & 0x20) != 0,
            reverse_y: (f & 0x40) != 0,
            priority: (f & 0x80) != 0,
        }
    }
}

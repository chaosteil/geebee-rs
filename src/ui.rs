use crate::cpu::CPU;
use crate::joypad;
use crate::lcd;

use ::image as im;
use piston_window::*;

const SCALE: u32 = 2;
const SCREEN_WIDTH: u32 = lcd::SCREEN_SIZE.0 as u32;
const SCREEN_HEIGHT: u32 = lcd::SCREEN_SIZE.1 as u32;

pub fn launch(mut cpu: CPU) -> Result<(), Box<dyn std::error::Error>> {
    let mut window: PistonWindow =
        WindowSettings::new("GeeBee", (SCREEN_WIDTH * SCALE, SCREEN_HEIGHT * SCALE))
            .resizable(false)
            .build()?;
    window.set_ups(60);
    let mut texture_context = window.create_texture_context();
    let mut texture = Texture::from_image(
        &mut texture_context,
        &im::RgbaImage::from_vec(SCREEN_WIDTH, SCREEN_HEIGHT, cpu.lcd().screen().to_vec()).unwrap(),
        &TextureSettings::new().filter(texture::Filter::Nearest),
    )
    .unwrap();
    while let Some(e) = window.next() {
        if let Some(Button::Keyboard(key)) = e.press_args() {
            if let Some(b) = map_button(key) {
                cpu.joypad().press(b);
            }
        }
        if let Some(Button::Keyboard(key)) = e.release_args() {
            if let Some(b) = map_button(key) {
                cpu.joypad().release(b);
            }
        }

        if e.update_args().is_some() {
            cpu.cycle();
            texture
                .update(
                    &mut texture_context,
                    &im::RgbaImage::from_vec(
                        SCREEN_WIDTH,
                        SCREEN_HEIGHT,
                        cpu.lcd().screen().to_vec(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        window.draw_2d(&e, |c, g, d| {
            texture_context.encoder.flush(d);
            image(&texture, c.transform.zoom(SCALE as f64), g);
        });
    }
    Ok(())
}

fn map_button(key: keyboard::Key) -> Option<joypad::Button> {
    match key {
        Key::W => Some(joypad::Button::Up),
        Key::A => Some(joypad::Button::Left),
        Key::S => Some(joypad::Button::Down),
        Key::D => Some(joypad::Button::Right),
        Key::Z => Some(joypad::Button::Start),
        Key::X => Some(joypad::Button::Select),
        Key::N => Some(joypad::Button::B),
        Key::M => Some(joypad::Button::A),
        _ => None,
    }
}

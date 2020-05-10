use crate::cpu::CPU;
use crate::joypad;

use ::image as im;
use piston_window::*;

const SCALE: u32 = 2;

pub fn launch(mut cpu: CPU) -> Result<(), Box<dyn std::error::Error>> {
    let mut window: PistonWindow = WindowSettings::new("GeeBee", (160 * SCALE, 144 * SCALE))
        .resizable(false)
        .build()?;
    window.set_ups(60);
    let mut texture_context = TextureContext {
        factory: window.factory.clone(),
        encoder: window.factory.create_command_buffer().into(),
    };
    let mut texture = Texture::from_memory_alpha(
        &mut texture_context,
        cpu.lcd().screen(),
        160,
        144,
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
        }
        window.draw_2d(&e, |c, g, d| {
            texture_context.encoder.flush(d);
            let screen = cpu.lcd().screen();
            texture
                .update(
                    &mut texture_context,
                    &im::RgbaImage::from_vec(160, 144, screen.to_vec()).unwrap(),
                )
                .unwrap();
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
        Key::Return => Some(joypad::Button::Start),
        Key::Space => Some(joypad::Button::Select),
        Key::N => Some(joypad::Button::B),
        Key::M => Some(joypad::Button::A),
        _ => None,
    }
}

use crate::cpu::CPU;

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

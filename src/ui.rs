use crate::cpu::CPU;

use ::image as im;
use piston_window::*;

pub fn launch(mut cpu: CPU) -> Result<(), Box<dyn std::error::Error>> {
    let mut window: PistonWindow = WindowSettings::new("GeeBee", (160, 144))
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
        &TextureSettings::new(),
    )
    .unwrap();
    while let Some(e) = window.next() {
        if let Some(_) = e.update_args() {
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
            image(&texture, c.transform, g);
        });
    }
    Ok(())
}

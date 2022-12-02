use sdl2::event::*;

use self::renderer::*;

mod renderer;

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video_subsystem = sdl.video()?;

    let window = video_subsystem
        .window("Office Apocalypse", 640, 480)
        .opengl()
        .build()
        .unwrap();

    let mut renderer = Renderer::new(&window, video_subsystem.clone())?;

    let mut event_pump = sdl.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                }
                _ => {}
            }
        }

        renderer.render()?;
    }

    Ok(())
}

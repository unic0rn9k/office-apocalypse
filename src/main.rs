#![feature(adt_const_params)]

use sdl2::event::*;
use sdl2::keyboard::*;
use sdl2::video::*;
use sdl2::*;

use self::renderer::*;

mod renderer;
mod rhi;

fn setup_window(video_subsystem: &VideoSubsystem) -> Window {
    let window = video_subsystem
        .window("Office Apocalypse", 640, 480)
        .fullscreen_desktop()
        .allow_highdpi()
        .opengl()
        .build()
        .unwrap();

    window
}

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video_subsystem = sdl.video()?;
    let audio_subsystem = sdl.audio()?;

    let mut window = setup_window(&video_subsystem);

    let mut renderer = Renderer::new(&window);

    let mut event_pump = sdl.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown { keycode, .. } => {
                    if let Some(keycode) = keycode {
                        match keycode {
                            Keycode::Escape => match window.fullscreen_state() {
                                FullscreenType::Desktop => {
                                    window.set_fullscreen(FullscreenType::Off).unwrap();
                                }
                                FullscreenType::Off => {
                                    window.set_fullscreen(FullscreenType::Desktop).unwrap();
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                Event::Quit { .. } => break 'running,
                _ => {}
            }
        }

        renderer.run();
    }

    Ok(())
}

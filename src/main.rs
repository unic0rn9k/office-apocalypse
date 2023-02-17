#![feature(iter_array_chunks, let_chains, slice_as_chunks, array_chunks, test)]

use glam::*;
use sdl2::event::*;
use sdl2::keyboard::Scancode;
use sdl2::video::*;
use sdl2::*;

use crate::game::*;
use crate::renderer::*;
use crate::scene::*;

mod ai;
mod format;
mod game;
mod renderer;
mod rhi;
mod scene;
mod tensor;
mod terrain;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

fn setup_window(video_subsystem: &VideoSubsystem) -> Window {
    video_subsystem.gl_attr().set_context_version(4, 6);
    // video_subsystem.gl_attr().set_multisample_samples(4);
    video_subsystem
        .gl_attr()
        .set_context_profile(GLProfile::Core);

    let window = video_subsystem
        .window("Office Apocalypse", WIDTH, HEIGHT)
        .resizable()
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
    let mut event_pump = sdl.event_pump()?;

    let mut window = setup_window(&video_subsystem);
    let mut renderer = Renderer::new(&window, true);

    let camera = Camera::new(Vec3::new(0.0, 0.0, -2.0), ASPECT_RATIO);
    let mut scene = Scene::new(camera);
    let mut game = Game::new(&mut scene);

    let mut dt = 1.0;
    'running: loop {
        for event in event_pump.poll_iter() {
            #[allow(clippy::collapsible_match, clippy::single_match)]
            match event {
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(width, height) => {
                        scene.camera_mut().resize(width as f32, height as f32);
                        renderer.resize(uvec2(width as _, height as _));
                    }
                    WindowEvent::Close => break 'running,
                    _ => {}
                },
                Event::KeyDown { scancode, .. } if scancode == Some(Scancode::Escape) => {
                    let fullscreen = match window.fullscreen_state() {
                        FullscreenType::Off => {
                            sdl.mouse().show_cursor(false);
                            FullscreenType::Desktop
                        }
                        FullscreenType::Desktop => {
                            sdl.mouse().show_cursor(true);
                            FullscreenType::Off
                        }
                        _ => FullscreenType::Off,
                    };

                    window.set_fullscreen(fullscreen).unwrap();
                }
                Event::Quit { .. } => break 'running,
                _ => {}
            }
        }

        if let Some(time) = renderer.render(&mut scene) {
            dt = (time / 1000.0) as _;
        }

        let mut systems = GameSystems {
            keyboard: event_pump.keyboard_state(),
            mouse: event_pump.mouse_state(),
            dt,
        };

        game.run(&mut systems, &mut scene);
    }

    Ok(())
}

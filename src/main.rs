#![feature(iter_array_chunks)]
#![feature(let_chains)]
#![feature(slice_as_chunks)]
#![feature(array_chunks)]

use glam::*;
use sdl2::event::*;
use sdl2::keyboard::*;
use sdl2::video::*;
use sdl2::*;

use crate::renderer::*;
use crate::scene::*;

mod renderer;
mod rhi;
mod scene;
mod vox;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

fn setup_window(video_subsystem: &VideoSubsystem) -> Window {
    video_subsystem.gl_attr().set_context_version(4, 6);
    video_subsystem.gl_attr().set_multisample_samples(4);
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

    let mut window = setup_window(&video_subsystem);

    let mut renderer = Renderer::new(&window, true);

    let camera = Camera::new(Vec3::new(0.0, 0.0, -2.0), ASPECT_RATIO);
    let mut scene = Scene::open("./assets/gun.vox", camera);

    let mut event_pump = sdl.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown { keycode, .. } => {
                    if let Some(code) = keycode {
                        match code {
                            Keycode::Escape => match window.fullscreen_state() {
                                FullscreenType::Desktop => {
                                    window.set_fullscreen(FullscreenType::Off).unwrap();
                                }
                                FullscreenType::Off => {
                                    window.set_fullscreen(FullscreenType::Desktop).unwrap();
                                }
                                _ => {}
                            },
                            Keycode::W => {
                                scene.camera.translate(Vec3::new(0.0, 0.0, -1.0));
                            }
                            Keycode::A => {
                                scene.camera.translate(Vec3::new(-1.0, 0.0, 0.0));
                            }
                            Keycode::S => {
                                scene.camera.translate(Vec3::new(0.0, 0.0, 1.0));
                            }
                            Keycode::D => {
                                scene.camera.translate(Vec3::new(1.0, 0.0, 0.0));
                            }
                            Keycode::Space => {
                                scene.camera.translate(Vec3::new(0.0, 1.0, 0.0));
                            }
                            Keycode::N => {
                                scene.camera.translate(Vec3::new(0.0, -1.0, 0.0));
                            }
                            _ => {}
                        }
                    }
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(width, height) => {
                        renderer.resize(width as u32, height as u32);
                        scene.camera.resize(width as f32, height as f32);
                    }
                    _ => {}
                },
                Event::Quit { .. } => break 'running,
                _ => {}
            }
        }

        // let start = std::time::Instant::now();
        renderer.run(&scene);
        // let ft = std::time::Instant::now().duration_since(start);
        // println!("{} frames/s", 1.0 / ft.as_secs_f32());
    }

    Ok(())
}

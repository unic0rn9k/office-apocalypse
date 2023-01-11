use glam::*;
use sdl2::keyboard::{KeyboardState, Scancode};

use crate::scene::Scene;

pub struct GameSystems<'a> {
    pub keyboard: KeyboardState<'a>,
    pub dt: f32,
}

pub struct Game {
    health: u32,
}

impl Game {
    pub fn new(scene: &mut Scene) -> Self {
        Self { health: 100 }
    }

    pub fn run(&mut self, systems: &mut GameSystems, scene: &mut Scene) {
        let GameSystems { keyboard, dt } = systems;

        if keyboard.is_scancode_pressed(Scancode::W) {
            scene.camera.translate(Vec3::new(0.0, 0.0, -1.0) * *dt);
        }

        if keyboard.is_scancode_pressed(Scancode::A) {
            scene.camera.translate(Vec3::new(-1.0, 0.0, 0.0) * *dt);
        }

        if keyboard.is_scancode_pressed(Scancode::S) {
            scene.camera.translate(Vec3::new(0.0, 0.0, 1.0) * *dt);
        }

        if keyboard.is_scancode_pressed(Scancode::D) {
            scene.camera.translate(Vec3::new(1.0, 0.0, 0.0) * *dt);
        }
    }
}

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
    const SPEED: f32 = 100.0;

    pub fn new(scene: &mut Scene) -> Self {
        Self { health: 100 }
    }

    pub fn run(&mut self, systems: &mut GameSystems, scene: &mut Scene) {
        let GameSystems { keyboard, dt } = systems;
        let Scene { camera, .. } = scene;
        let dt = *dt;

        if keyboard.is_scancode_pressed(Scancode::W) {
            camera.translate(Vec3::new(0.0, 0.0, -Self::SPEED) * dt);
        }

        if keyboard.is_scancode_pressed(Scancode::A) {
            camera.translate(Vec3::new(-Self::SPEED, 0.0, 0.0) * dt);
        }

        if keyboard.is_scancode_pressed(Scancode::S) {
            camera.translate(Vec3::new(0.0, 0.0, Self::SPEED) * dt);
        }

        if keyboard.is_scancode_pressed(Scancode::D) {
            camera.translate(Vec3::new(Self::SPEED, 0.0, 0.0) * dt);
        }
    }
}

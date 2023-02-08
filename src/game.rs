use glam::*;
use sdl2::keyboard::{KeyboardState, Scancode};

use crate::format::vox;
use crate::scene::{Camera, Entity, Light, Model, Object, Scene, SceneNode, SceneNodeId, Text};

pub struct GameSystems<'a> {
    pub keyboard: KeyboardState<'a>,
    pub dt: f32,
}

pub struct Game {
    health: u32,
    ammo: u32,
    gun: SceneNodeId,
}

impl Game {
    const SPEED: f32 = 1000.0;
    const CAPACITY: u32 = 9;

    pub fn new(scene: &mut Scene) -> Self {
        let (gun, magazine) = {
            let (models, materials) = vox::open("./assets/gun.vox");
            if !scene.has_materials() {
                let materials = Box::new(materials.map(Into::into));
                scene.set_materials(materials);
            }

            let gun_model = Model::from(models[3].clone());
            let gun = Object::with_tag(Mat4::IDENTITY, gun_model, "gun".to_string());

            let magazine_model = Model::from(models[2].clone());

            let magazine = Object::with_tag(
                Mat4::from_translation(vec3(-37.0, -2.0, 20.0)),
                magazine_model,
                "magazine".to_string(),
            );

            (gun, magazine)
        };

        let gun_id = scene
            .scene_graph
            .insert_entity(gun, &scene.scene_graph.root());

        let _magazine_id = scene.scene_graph.insert_entity(magazine, &gun_id);

        scene.text.push(Text {
            position: uvec2(0, 0),
            text: "FPS".to_string(),
            color: vec4(0.0, 1.0, 0.0, 1.0),
            scale: 0.5,
        });

        Self {
            health: 100,
            gun: gun_id,
            ammo: 9,
        }
    }

    pub fn run(&mut self, systems: &mut GameSystems, scene: &mut Scene) {
        let keyboard = &mut systems.keyboard;
        let dt = systems.dt;

        let camera = scene.camera_mut();

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

        // TODO: Aksel fix le shooting.
        // TODO: Bech fix le camera movement.
        let gun_entity = scene
            .scene_graph
            .object_mut(&self.gun)
            .expect("Unable to find gun");

        // gun_entity.transform *= Mat4::from_translation(vec3(0.01, 0.0, 0.0));

        scene.text[0].text = format!("FPS {:05.1}", 1.0 / dt);
    }
}

use glam::*;
use sdl2::keyboard::{KeyboardState, Scancode};
use sdl2::mouse::*;

use crate::format::vox;
use crate::scene::{Camera, Entity, Light, Model, Object, Scene, SceneNode, SceneNodeId, Text};
use crate::tensor::SparseTensorChunk;

pub struct GameSystems<'a> {
    pub keyboard: KeyboardState<'a>,
    pub mouse: MouseState,
    pub dt: f32,
}

enum Weapon {
    Gun(SceneNodeId, u32),
    Knife(SceneNodeId),
}

pub struct Game {
    health: u32,
    weapon: Weapon,
    nframes_since_jump: Option<usize>,
    nframes_since_shoot: Option<usize>,
    nframes_since_reload: Option<usize>,
    nframes_since_attack: Option<usize>,
}

impl Game {
    const SPEED: f32 = 100.0;
    const CAPACITY: u32 = 9;

    pub fn new(scene: &mut Scene) -> Self {
        // Terrain
        {
            let (models, _) = vox::open("./assets/kitchen.vox");
            let kitchen = Model::from(models[0].clone());

            let mut chunk = SparseTensorChunk::from(kitchen);
            chunk.transform *= Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);

            scene.terrain.push(chunk);

            let (models, _) = vox::open("./assets/kitchen_island.vox");
            let kitchen_island = Model::from(models[0].clone());

            let mut chunk = SparseTensorChunk::from(kitchen_island);
            chunk.transform *= Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
            scene.terrain.push(chunk);

            let (models, _) = vox::open("./assets/floor.vox");
            let mut floor = SparseTensorChunk::from(Model::from(models[0].clone()));
            floor.transform *= Mat4::from_translation(vec3(-200.0, -5.0, 0.0));
            floor.transform *= Mat4::from_scale(vec3(10.0, 10.0, 0.1));

            scene.terrain.push(floor);

            let (models, _) = vox::open("./assets/doorframe.vox");
            let mut frame = SparseTensorChunk::from(Model::from(models[0].clone()));
            frame.transform *= Mat4::from_translation(vec3(0.0, 1.0, 0.0));

            scene.terrain.push(frame);

            let (models, _) = vox::open("./assets/wall.vox");
            let mut wall = SparseTensorChunk::from(Model::from(models[0].clone()));
            wall.transform *= Mat4::from_translation(vec3(-40.0, 1.0, 0.0));

            scene.terrain.push(wall);
        }

        // FPS
        scene.text.push(Text {
            position: uvec2(0, 0),
            text: "FPS".to_string(),
            color: vec4(0.0, 1.0, 0.0, 1.0),
            scale: 0.5,
        });

        scene.camera_mut().translate(vec3(0.0, 16.0, 0.0));

        let gun_id = Self::spawn_gun(scene);

        Self {
            health: 100,
            weapon: Weapon::Gun(gun_id, Self::CAPACITY),
            nframes_since_jump: None,
            nframes_since_reload: None,
            nframes_since_shoot: None,
            nframes_since_attack: None,
        }
    }

    pub fn run(&mut self, systems: &mut GameSystems, scene: &mut Scene) {
        let keyboard = &systems.keyboard;
        let mouse = &systems.mouse;
        let dt = systems.dt;

        let camera = scene.camera_mut();

        // Controller
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

        let is_grounded = camera.translation().y == 16.0;
        if keyboard.is_scancode_pressed(Scancode::Space) && is_grounded {
            camera.translate(vec3(0.0, 2.0, 0.0));
        }

        // Weapon switch
        if keyboard.is_scancode_pressed(Scancode::Num1) {
            if let Weapon::Knife(knife_id) = &self.weapon {
                scene.scene_graph.remove_entity(knife_id);
                self.weapon = Weapon::Gun(Self::spawn_gun(scene), Self::CAPACITY);
            }
        } else if keyboard.is_scancode_pressed(Scancode::Num2) {
            if let Weapon::Gun(gun_id, _) = &self.weapon {
                scene.scene_graph.remove_entity(gun_id);
                self.weapon = Weapon::Knife(Self::spawn_knife(scene));
            }
        }

        match &mut self.weapon {
            Weapon::Gun(gun_id, ammo) => {
                // Shoot
                if mouse.left() {
                    // TODO
                }

                // Reload
                if keyboard.is_scancode_pressed(Scancode::R) {
                    *ammo = Self::CAPACITY;
                }

                // Ammo Counter
                scene.text.push(Text::black(
                    uvec2(0, 0),
                    format!("{ammo}/{}", Self::CAPACITY),
                ));
            }
            Weapon::Knife(knife_id) => {
                // Attack
                if mouse.left() {
                    // TODO
                }
            }
        }

        // Update the fps counter with the latest delta time.
        scene.text[0].text = format!("FPS {:05.1}", 1.0 / dt);
    }

    fn spawn_gun(scene: &mut Scene) -> SceneNodeId {
        let (gun, magazine) = {
            let (models, materials) = vox::open("./assets/gun.vox");
            if !scene.has_materials() {
                let materials = Box::new(materials.map(Into::into));
                scene.set_materials(materials);
            }

            let gun_model = Model::from(models[3].clone());
            let mut gun = Object::new(Mat4::IDENTITY, gun_model);
            gun.transform *= Mat4::from_translation(vec3(-0.5, -3.0, 10.25));
            gun.transform *= Mat4::from_scale(vec3(0.1, 0.1, 0.1));
            gun.transform *= Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2);

            let magazine_model = Model::from(models[2].clone());

            let magazine = Object::new(
                Mat4::from_translation(vec3(-37.0, -2.0, 20.0)),
                magazine_model,
            );

            (gun, magazine)
        };

        let Scene { scene_graph, .. } = scene;
        let gun_id = scene_graph.insert_entity(gun, &scene.camera);
        let _ = scene_graph.insert_entity(magazine, &gun_id);

        gun_id
    }

    fn spawn_knife(scene: &mut Scene) -> SceneNodeId {
        let (models, _) = vox::open("./assets/knife.vox");
        let mut knife = Object::new(Mat4::IDENTITY, Model::from(models[0].clone()));

        knife.transform *= Mat4::from_translation(vec3(3.0, -16.0, 10.0));
        knife.transform *= Mat4::from_scale(vec3(0.25, 0.25, 0.25));
        knife.transform *= Mat4::from_rotation_x(1.1);
        knife.transform *= Mat4::from_rotation_y(-1.6);

        scene.scene_graph.insert_entity(knife, &scene.camera)
    }

    fn handle_attack(&mut self) {}

    fn handle_jump(&mut self) {}

    fn handle_reload(&mut self) {}
}

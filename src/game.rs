use glam::*;
use sdl2::keyboard::{KeyboardState, Scancode};

use crate::ai::Brain;
use crate::format::vox;
use crate::scene::{Camera, Entity, Light, Model, Object, Scene, SceneNode, SceneNodeId, Text};
use crate::tensor::{self, SparseTensorChunk};
use crate::terrain;

#[derive(Debug, Default)]
pub struct MouseState {
    pub has_mouse_left_been_clicked: bool,
    pub has_mouse_right_been_clicked: bool,
    pub dx: i32,
    pub dy: i32,
}

pub struct GameSystems<'a> {
    pub window_size: UVec2,
    pub keyboard: KeyboardState<'a>,
    pub mouse: MouseState,
    pub dt: f32,
}


enum Weapon {
    Gun(SceneNodeId, u32),
    Knife(SceneNodeId),
}

struct Enemy {
    id: SceneNodeId,
    health: u32,
}

pub struct Game {
    yaw: f32,
    pitch: f32,

    // Player state
    health: u32,
    weapon: Weapon,

    // Enemy state
    enemies: Vec<(Brain, Enemy)>,
    
    // Animation state
    nframes_since_spawn: usize,
    nframes_since_jump: Option<usize>,
    nframes_since_shoot: Option<usize>,
    nframes_since_reload: Option<usize>,
    nframes_since_attack: Option<usize>,
}

impl Game {
    const SPEED: f32 = 1.0;
    const CAPACITY: u32 = 9;

    pub fn new(scene: &mut Scene) -> Self {
        // Terrain
        {
            let (models, _) = vox::open("./assets/floor.vox");
            let mut floor = SparseTensorChunk::from(Model::from(models[0].clone()));
            floor.transform *= Mat4::from_translation(vec3(-200.0, -5.0, 0.0));
            floor.transform *= Mat4::from_scale(vec3(10.0, 10.0, 0.1));

            scene.terrain.push(floor);

            let player_block = terrain::closest_block(scene.camera().position);
            let map_block = terrain::MapBlock::from_scratch(player_block);
            let mut terrain = map_block.gen_terrain(terrain::EMPTY_MASK);
            terrain.compress();
            scene.terrain.push(terrain);
        }

        // FPS
        scene.text.push(Text {
            position: uvec2(0, 0),
            text: "FPS".to_string(),
            color: vec4(0.0, 1.0, 0.0, 1.0),
            scale: 0.5,
        });

        scene.camera_mut().translate(vec3(0.0, 16.0, 0.0));

        let gun = Self::spawn_gun(scene);

        let enemy = Self::spawn_enemy(scene);

        Self {
            yaw: -90.0,
            pitch: 0.0,

            health: 100,
            weapon: Weapon::Gun(gun, Self::CAPACITY),
            enemies: vec![enemy],
            
            nframes_since_spawn: 0,
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

        self.handle_movement(systems, scene);
        self.handle_shoot(scene);

        // self.shoot_animation(scene);
        self.jump_animation(scene);

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
            Weapon::Gun(id, ammo) => {
                // Make the gun follow the camera. Doesn't work with the scenegraph for some reason.
                let position = scene.camera().translation();
                let direction = scene.camera().direction();
                let gun = scene.scene_graph.object_mut(id).unwrap();
                gun.transform = Mat4::from_translation(position);
                gun.transform *= Mat4::from_translation(vec3(-1.0, 0.0, 2.5));
                gun.transform *= Mat4::from_scale(vec3(0.05, 0.05, 0.05)); 
                gun.transform *= Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2 + 0.1);
                gun.transform *= Mat4::from_rotation_y(direction.x) * Mat4::from_rotation_x(direction.y);


                // Shoot
                if mouse.has_mouse_left_been_clicked && *ammo != 0 {
                    self.nframes_since_shoot = Some(0);
                    println!("pew");
                }

                // Reload
                if keyboard.is_scancode_pressed(Scancode::R) && *ammo != Self::CAPACITY {
                    *ammo = Self::CAPACITY;

                    self.nframes_since_reload = Some(0);
                }

                // Ammo Counter
                scene.text.push(Text::black(
                    uvec2(0, 0),
                    format!("{ammo}/{}", Self::CAPACITY),
                ));
            }
            Weapon::Knife(knife_id) => {
                // Attack
                if mouse.has_mouse_right_been_clicked {
                    // TODO
                }
            }
        }

        // Update the fps counter with the latest delta time.
        scene.text[0].text = format!("FPS {:05.1}", 1.0 / dt);
    }

    fn update_enemies(&mut self, scene: &mut Scene) {
        for (brain, enemy) in &mut self.enemies {
            brain.route.clear();
            // brain.append_destination(scene.camera().translation(), scene);
        }
    }

    fn spawn_enemy(scene: &mut Scene) -> (Brain, Enemy) {
        let Scene { scene_graph, .. } = scene;

        // This should be cached...
        let (models, _) = vox::open("./assets/zombie.vox");
        let zombie = Model::from(models[0].clone());

        // Determine zombie spawn location
        let transform = Mat4::IDENTITY;

        let id = scene_graph.insert_entity(Object::new(transform, zombie), &scene_graph.root());
        (Brain {position: uvec3(0, 0, 0), route: vec![]}, Enemy { id, health: 100 })
    }

    fn spawn_gun(scene: &mut Scene) -> SceneNodeId {
        let (gun, magazine) = {
            let (models, materials) = vox::open("./assets/gun.vox");
            if !scene.has_materials() {
                let materials = Box::new(materials.map(Into::into));
                scene.set_materials(materials);
            }

            // To have decent rotations we must map the coordinates from 0..40 to -20..20 which moves the origin to the center of the chunk.
            let mut model = models[3].clone();
            model.positions.iter_mut().for_each(|(position, _)| {
                *position -= vec3(40.0, 40.0, 40.0);
            });

            let gun_model = Model::from(model);
            let mut gun = Object::new(Mat4::IDENTITY, gun_model);
            gun.transform *= Mat4::from_translation(vec3(0.0, 0.0, 0.0));
            // gun.transform *= Mat4::from_scale(vec3(0.1, 0.1, 0.1));

            let mut magazine_model = models[2].clone();
            magazine_model.positions.iter_mut().for_each(|(position, _)| {
                *position -= vec3(40.0, 40.0, 40.0);
            });

            let magazine = Object::new(
                Mat4::from_translation(vec3(-37.0, -2.0, 20.0)),
                Model::from(magazine_model),
            );

            (gun, magazine)
        };

        let Scene { scene_graph, .. } = scene;
        let gun_id = scene_graph.insert_entity(gun, &scene_graph.root());
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

    fn handle_movement(&mut self, systems: &GameSystems, scene: &mut Scene) {
        let keyboard = &systems.keyboard;
        let mouse = &systems.mouse;
        let dt = systems.dt;

        let camera = *scene.camera();
        // Walk around with WASD keys
        // TODO fix that we are moving slower when pointing upwards
        let speed = vec3(Self::SPEED, 0.0, Self::SPEED);
        if keyboard.is_scancode_pressed(Scancode::W) {
            scene.camera_mut().translate(camera.direction() * speed);
        }

        if keyboard.is_scancode_pressed(Scancode::A) {
            scene.camera_mut().translate(-camera.right() * speed);
        }   

        if keyboard.is_scancode_pressed(Scancode::S) {
            scene.camera_mut().translate(-camera.direction() * speed);
        }

        if keyboard.is_scancode_pressed(Scancode::D) {
            scene.camera_mut().translate(camera.right() * speed);
        }

        // Like in real life we can only jump if we are grounded.
        let is_grounded = camera.translation().y == 16.0;
        if keyboard.is_scancode_pressed(Scancode::Space) && is_grounded {
            self.nframes_since_jump = Some(0);
        }

        // Look around using the mouse
        let Self {yaw, pitch, ..} = self;
        *yaw += mouse.dx as f32;
        *pitch -= mouse.dy as f32;

        match *pitch {
            p if p > 89.0 => *pitch = 89.0,
            p if p < -89.0 => *pitch = -89.0,
            _ => {}
        }

        // We must convert to radians since the trigometric functions only work in radians
        let [yaw, pitch] = [*yaw, *pitch].map(f32::to_radians);
        let direction = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
        scene.camera_mut().set_direction(direction);
    }

    fn jump_animation(&mut self, scene: &mut Scene) {
        let camera = scene.camera_mut();

        if let Some(n) = &mut self.nframes_since_jump {
            *n += 1;

            match *n - 1 {
                n if n < 8 => camera.translate(vec3(0.0, 1.5, 0.0)),
                n if n >= 8 && n < 10 => {}
                n if n >= 10 && n < 14 => camera.translate(vec3(0.0, -(12.0 / 4.0), 0.0)),
                _ => self.nframes_since_jump = None,
            }

            if self.nframes_since_jump.is_none() {
                camera.translate(vec3(0.0, 16.0 - camera.translation().y, 0.0))
            }
        }
    }

    fn handle_shoot(&mut self, scene: &mut Scene) {
        let camera = *scene.camera();
        let Scene { scene_graph, .. } = scene;

        if let Weapon::Gun(gun_id, ammo) = &self.weapon && let Some(n) = &mut self.nframes_since_shoot {
            let gun = scene_graph.object_mut(gun_id).unwrap();

            if *n == 0 {
                let ray = Ray::with_len(vec3(0.0, 0.0, 0.0), camera.direction(), 100.0);
                
            }

            *n += 1;

            match *n - 1 {
                n if n < 2 => gun.transform *= Mat4::from_translation(vec3(-2.0, 0.0, 0.0)),
                n if (2..4).contains(&n) => gun.transform *= Mat4::from_translation(vec3(2.0, 0.0, 0.0)),
                _ => self.nframes_since_shoot = None
            }


        }
    }

    fn handle_reload(&mut self) {}

    fn handle_attack(&mut self) {}
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
    len: f32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction,
            len: f32::INFINITY,
        }
    }

    pub fn with_len(origin: Vec3, direction: Vec3, len: f32) -> Self {
        Self {
            origin,
            direction,
            len,
        }
    }

    /// Checks if a ray intersects with an entity in the scene
    pub fn cast_entity(&self, steps: f32, scene: &mut Scene) -> Option<SceneNodeId> {
        let Scene {
            scene_graph,
            terrain,
            ..
        } = scene;

        // First we find all chunks where the ray intersects the scene.
        let mut t = 0.0;
        while t <= self.len {
            // The location in world-space.
            let mut v = self.origin + self.direction * t;

            // We convert the vector to a UVec3 eg. voxel-space.
            let mut v_voxel = uvec3(0, 0, 0);

            for chunk in &scene.terrain {
                if chunk.transform == Mat4::IDENTITY {
                    chunk.idx(v_voxel);
                }
            }

            t += self.len / steps;
        }

        // Then we sort all these voxels, so that the first hit

        todo!()
    }
}

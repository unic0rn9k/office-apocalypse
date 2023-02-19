use glam::*;
use sdl2::keyboard::{KeyboardState, Scancode};

use crate::ai::Brain;
use crate::format::vox;
use crate::scene::{Camera, Entity, Light, Model, Object, Scene, SceneNode, SceneNodeId, Text};
use crate::tensor::{self, SparseTensorChunk};

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
            let (models, _) = vox::open("./assets/kitchen.vox");
            let kitchen = Model::from(models[0].clone());
            //assert_eq!(kitchen.transform, Mat4::IDENTITY);

            let mut src_chunk = SparseTensorChunk::nothing(UVec3::ZERO);

            let mut chunk = SparseTensorChunk::from(kitchen);
            chunk.transform *= Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);

            src_chunk = tensor::combine(src_chunk, chunk);

            let (models, _) = vox::open("./assets/kitchen_island.vox");
            let kitchen_island = Model::from(models[0].clone());

            let mut chunk = SparseTensorChunk::from(kitchen_island);
            chunk.transform *= Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);

            src_chunk = tensor::combine(src_chunk, chunk);
            scene.terrain.push(src_chunk);

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
                // Make the gun follow the camera. Doesn't work with the scenegraph for some
                // reason.
                let position = scene.camera().translation();
                let direction = scene.camera().direction();
                let gun = scene.scene_graph.object_mut(id).unwrap();
                gun.transform = Mat4::from_translation(position);
                gun.transform *= Mat4::from_translation(vec3(-1.0, 0.0, 2.5));
                gun.transform *= Mat4::from_scale(vec3(0.05, 0.05, 0.05));
                gun.transform *= Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2 + 0.1);
                gun.transform *= Mat4::from_rotation_y(direction.x);

                // Shoot
                if mouse.has_mouse_left_been_clicked && *ammo != 0 {
                    self.nframes_since_shoot = Some(0);
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
        let position = uvec3(0, 0, 0);
        let transform = Mat4::from_translation(position.as_vec3());

        let id = scene_graph.insert_entity(
            Object::with_tag(transform, zombie, "enemy".to_string()),
            &scene_graph.root(),
        );
        (
            Brain {
                position,
                route: vec![],
            },
            Enemy { id, health: 100 },
        )
    }

    fn spawn_gun(scene: &mut Scene) -> SceneNodeId {
        let (gun, magazine) = {
            let (models, materials) = vox::open("./assets/gun.vox");
            if !scene.has_materials() {
                let materials = Box::new(materials.map(Into::into));
                scene.set_materials(materials);
            }

            // To have decent rotations we must map the coordinates from 0..40 to -20..20
            // which moves the origin to the center of the chunk.
            let mut model = models[3].clone();
            model.positions.iter_mut().for_each(|(position, _)| {
                *position -= vec3(40.0, 40.0, 40.0);
            });

            let gun_model = Model::from(model);
            let mut gun = Object::new(Mat4::IDENTITY, gun_model);

            let mut magazine_model = models[2].clone();
            magazine_model
                .positions
                .iter_mut()
                .for_each(|(position, _)| {
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
        let Self { yaw, pitch, .. } = self;
        *yaw += mouse.dx as f32;
        *pitch -= mouse.dy as f32;

        match *pitch {
            p if p > 89.0 => *pitch = 89.0,
            p if p < -89.0 => *pitch = -89.0,
            _ => {}
        }

        // We must convert to radians since the trigometric functions only work in
        // radians
        let [yaw, pitch] = [*yaw, *pitch].map(f32::to_radians);
        let direction = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        );
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
        // let Scene { scene_graph, .. } = scene;

        if let Weapon::Gun(gun_id, ammo) = &self.weapon && let Some(n) = &mut self.nframes_since_shoot {
            
            if *n == 0 {
                let ray = Ray::with_len(vec3(0.0, 0.0, 0.0), camera.direction(), 100.0);
                if let Some(id) = ray.cast_object(1000.0, scene, "enemy") {
                    println!("hit");
                    // let enemy = scene.scene_graph.object_mut(&id).unwrap();
                    let (i, enemy) = self.enemies.iter_mut().enumerate().find_map(|(i, (_, enemy))| (enemy.id == id).then_some((i, enemy))).unwrap();
                    enemy.health -= 10;

                    if enemy.health == 0 {
                        self.enemies.remove(i);
                        scene.scene_graph.remove_entity(&id);
                    }
                }
            }
            
            let gun = scene.scene_graph.object_mut(gun_id).unwrap();
            
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

    /// Checks if a ray intersects with an object in the scene
    pub fn cast_object(&self, steps: f32, scene: &mut Scene, tag: &str) -> Option<SceneNodeId> {
        let Scene { scene_graph, .. } = scene;

        let mut objects = Vec::new();

        for (id, entity) in scene_graph.mutated_entities() {
            if let Entity::Object(o) = entity && o.tag.contains(&tag) {
                objects.push((id, SparseTensorChunk::from(o.clone())));
            }
        }

        let mut t = 0.0;
        while t <= self.len {
            // The location in world-space.
            let v = self.origin + self.direction * t;

            // We convert the vector to a UVec3 eg. voxel-space.
            let v_voxel = uvec3(v.x as _, v.y as _, v.z as _);

            // First we find all chunks where the ray intersects the scene terrain.
            // let mut terrain_hit = None;
            // // let mut object_hit = None;

            // for chunk in &scene.terrain {
            //     if chunk.voxel(v_voxel).is_some() {
            //         // HIT!
            //         terrain_hit = Some(t);
            //     }
            // }

            for (id, chunk) in &objects {
                if chunk.voxel(v_voxel).is_some() {
                    return Some(id.clone());
                }
            }

            t += self.len / steps;
        }

        None
    }
}

use std::mem::MaybeUninit;
use std::path::Path;

use glam::*;

use crate::vox::{self, VoxMaterial};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaterialId(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: [u8; 4],
    pub roughness: f32,
    pub metalness: f32,
}

/// A chunk is a cube with consisting of `x` by `y` by `z` voxels.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub transform: Mat4,
    pub positions: Vec<(Vec3, MaterialId)>,
}

#[derive(Debug, Clone)]
pub struct Light {
    transform: Mat4,
}

#[derive(Debug, Clone)]
pub struct Object {
    transform: Mat4,
}

#[derive(Debug, Clone)]
pub enum Entity {
    Light(Light),
    Object(Object),
    Terrain(Chunk),
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub camera: Camera,
    pub entities: Vec<Entity>,
    materials: Box<[Material; 256]>,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            entities: Vec::default(),
            materials: Box::new([Material::default(); 256]),
        }
    }

    pub fn open(path: impl AsRef<Path>, camera: Camera) -> Self {
        let (models, materials) = vox::open(path);

        let rotation_x = Mat4::from_rotation_x(std::f32::consts::PI / 2.0);
        let rotation_y = Mat4::from_rotation_y(std::f32::consts::PI);

        let mut terrain = Vec::with_capacity(models.len());
        for model in models {
            let chunk = Chunk {
                transform: rotation_x * rotation_y * model.transform,
                positions: model
                    .positions
                    .into_iter()
                    .map(|(p, id)| (p, MaterialId(id.0 - 1)))
                    .collect(),
            };
            terrain.push(Entity::Terrain(chunk));
        }

        let materials = Box::new(materials.map(|vox| Material {
            albedo: vox.albedo,
            roughness: vox.roughness,
            metalness: vox.metalness,
        }));

        Self {
            camera,
            entities: terrain,
            materials,
        }
    }

    pub fn terrain(&self) -> impl Iterator<Item = &Chunk> {
        self.entities.iter().filter_map(|entity| match entity {
            Entity::Terrain(chunk) => Some(chunk),
            _ => None,
        })
    }

    pub fn materials(&self) -> &[Material] {
        self.materials.as_slice()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    position: Vec3,
    direction: Vec3,
    view: Mat4,
    projection: Mat4,
}

impl Camera {
    const FOV: f32 = std::f32::consts::FRAC_PI_2;

    pub fn new(position: Vec3, aspect_ratio: f32) -> Self {
        let direction = Vec3::new(0.0, 0.0, 1.0);

        Self {
            position,
            direction,
            view: Mat4::look_at_rh(position, position + direction, Vec3::new(0.0, 1.0, 0.0)),
            projection: Mat4::perspective_rh_gl(Self::FOV, aspect_ratio, 0.1, 100.0),
        }
    }

    pub fn view(&self) -> &Mat4 {
        &self.view
    }

    pub fn projection(&self) -> &Mat4 {
        &self.projection
    }

    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }

    pub fn translate(&mut self, by: Vec3) {
        self.position += by * Vec3::new(-1.0, 1.0, -1.0);

        self.view = Mat4::look_at_rh(
            self.position,
            self.position + self.direction,
            Vec3::new(0.0, 1.0, 0.0),
        );
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.projection = Mat4::perspective_rh_gl(Self::FOV, width / height, 0.1, 100.0);
    }
}

struct SceneNodeId(usize);

struct SceneNode {
    parent: usize,
    entity: Entity,
}

struct SceneGraph {
    nodes: Vec<SceneNode>,
}

impl SceneGraph {
    fn new() -> Self {
        todo!()
    }
    fn insert_entity(&mut self, entity: Entity, parent: &SceneNodeId) -> SceneNodeId {
        todo!()
    }
    fn evaluate(&self) -> Vec<Entity> {
        todo!()
    }
    fn entity(&self, id: &SceneNodeId) -> &Entity {
        todo!()
    }
    fn entity_mut(&mut self, id: &SceneNodeId) -> &mut Entity {
        todo!()
    }
}

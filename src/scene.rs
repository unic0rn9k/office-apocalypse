use std::mem::MaybeUninit;
use std::path::Path;

use glam::*;

use crate::vox::{self, VoxMaterial};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaterialID(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: [u8; 4],
    pub roughness: f32,
    pub metalness: f32,
}

/// A chunk is a cube consisting of `x` by `y` by `z` voxels.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub transform: Mat4,
    pub positions: Vec<(Vec3, MaterialID)>,
    pub size: Vec3,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub transform: Mat4,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub transform: Mat4,
}

#[derive(Debug, Clone)]
pub enum Entity {
    Light(Light),
    Object(Object),
    Terrain(Chunk),
}

impl Entity {
    fn transform(&self) -> Option<&Mat4> {
        match self {
            Entity::Light(l) => Some(&l.transform),
            Entity::Object(o) => Some(&o.transform),
            Entity::Terrain(t) => Some(&t.transform),
        }
    }

    fn transform_mut(&mut self) -> Option<&mut Mat4> {
        match self {
            Entity::Light(l) => Some(&mut l.transform),
            Entity::Object(o) => Some(&mut o.transform),
            Entity::Terrain(t) => Some(&mut t.transform),
        }
    }
}

macro_rules! impl_into_entity {
    ($($entity: ident),*) => {
        $(impl Into<Entity> for $entity{
            fn into(self) -> Entity{
                Entity::$entity(self)
            }
        })*
    };
}

impl_into_entity!(Light, Object);

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
            let size = model.size;
            let chunk = Chunk {
                transform: rotation_x * rotation_y * model.transform,
                size: Vec3::new(size.0 as _, size.1 as _, size.2 as _),
                positions: model
                    .positions
                    .into_iter()
                    .map(|(p, id)| (p, MaterialID(id.0 - 1)))
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

pub struct SceneNodeID(usize);

pub struct SceneNode {
    parent: SceneNodeID,
    base_entity: Entity,
    mutated_entity: Entity,
}

impl Clone for SceneNode {
    fn clone(&self) -> Self {
        Self {
            parent: SceneNodeID(self.parent.0),
            base_entity: self.base_entity.clone(),
            mutated_entity: self.mutated_entity.clone(),
        }
    }
}

impl SceneNode {
    fn new(entity: Entity, parent: &SceneNodeID) -> Self {
        Self {
            parent: SceneNodeID(parent.0),
            base_entity: entity.clone(),
            mutated_entity: entity,
        }
    }

    fn evaluate(&mut self, parent: &Option<SceneNode>) {
        if let Some(trans) = self.base_entity.transform() {
            let new_trans =
                if let Some(p_trans) = parent.as_ref().and_then(|p| p.mutated_entity.transform()) {
                    *p_trans * *trans
                } else {
                    *trans
                };
            *self
                .mutated_entity
                .transform_mut()
                .expect("base_entity and mutated entity differ in type") = new_trans
        }
    }
}

// Actually a scene tree, because each node only has one parent.
pub struct SceneGraph {
    nodes: Vec<Option<SceneNode>>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self { nodes: vec![None] }
    }

    pub fn insert_entity(&mut self, entity: Entity, parent: &SceneNodeID) -> SceneNodeID {
        self.nodes.push(Some(SceneNode::new(entity, parent)));
        SceneNodeID(self.nodes.len() - 1)
    }

    pub fn entity(&self, id: &SceneNodeID) -> Option<&Entity> {
        self.nodes[id.0].as_ref().map(|s| &s.base_entity)
    }

    pub fn entity_mut(&mut self, id: &SceneNodeID) -> Option<&mut Entity> {
        self.nodes[id.0].as_mut().map(|s| &mut s.base_entity)
    }

    pub fn evaluate_all(&mut self) {
        for n in 0..self.nodes.len() {
            let parent = if let Some(node) = &self.nodes[n] {
                self.nodes[node.parent.0].clone()
            } else {
                continue;
            };
            if let Some(node) = &mut self.nodes[n] {
                node.evaluate(&parent);
            }
        }
    }

    pub fn root_node(&self) -> SceneNodeID {
        SceneNodeID(0)
    }

    pub fn mutated_entity(&self, id: &SceneNodeID) -> Option<&Entity> {
        self.nodes[id.0].as_ref().map(|s| &s.mutated_entity)
    }
}

#[test]
fn graph() {
    let mut g = SceneGraph::new();
    let root = g.root_node();

    let transform = Mat4::from_cols_array_2d(&[[1., 2., 3., 4.]; 4]);

    let a = g.insert_entity(Object { transform }.into(), &root);
    let b = g.insert_entity(Object { transform }.into(), &a);
    g.evaluate_all();

    assert_eq!(
        g.mutated_entity(&b).unwrap().transform().unwrap(),
        &(transform * transform)
    );
}

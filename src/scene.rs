use std::mem::MaybeUninit;
use std::path::Path;

use glam::*;

use crate::format::vox::{VoxMaterial, VoxModel};
use crate::tensor::SparseTensorChunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaterialId(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: [u8; 4],
    pub roughness: f32,
    pub metalness: f32,
}

impl From<VoxMaterial> for Material {
    fn from(value: VoxMaterial) -> Self {
        Self {
            albedo: value.albedo,
            roughness: value.roughness,
            metalness: value.metalness,
        }
    }
}

/// A chunk is a cube consisting of `x` by `y` by `z` voxels.
#[derive(Debug, Default, Clone)]
pub struct Model {
    pub transform: Mat4,
    pub positions: Vec<(Vec3, MaterialId)>,
    pub size: UVec3,
}

impl From<VoxModel> for Model {
    fn from(value: VoxModel) -> Self {
        let positions = value
            .positions
            .into_iter()
            .map(|(position, mat)| (position, MaterialId(mat.0 - 1)))
            .collect();

        let transform = value.transform
            * Mat4::from_rotation_x(std::f32::consts::PI / 2.0)
            * Mat4::from_rotation_y(std::f32::consts::PI);

        let size = uvec3(value.size.0 as _, value.size.1 as _, value.size.2 as _);
        Self {
            positions,
            transform,
            size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Light {
    pub transform: Mat4,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub transform: Mat4,
    pub model: Model,
    pub tag: Option<String>,
}

impl Object {
    pub fn new(transform: Mat4, model: Model) -> Self {
        Self {
            transform,
            model,
            tag: None,
        }
    }

    pub fn with_tag(transform: Mat4, model: Model, tag: String) -> Self {
        Self {
            transform,
            model,
            tag: Some(tag),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Entity {
    Light(Light),
    Object(Object),
    Camera(Camera),
}

impl Entity {
    fn transform(&self) -> Option<&Mat4> {
        match self {
            Entity::Light(l) => Some(&l.transform),
            Entity::Object(o) => Some(&o.transform),
            Entity::Camera(c) => Some(&c.transform),
        }
    }

    fn transform_mut(&mut self) -> Option<&mut Mat4> {
        match self {
            Entity::Light(l) => Some(&mut l.transform),
            Entity::Object(o) => Some(&mut o.transform),
            Entity::Camera(c) => Some(&mut c.transform),
        }
    }
}

macro_rules! impl_into_entity {
    ($($entity: ident),*) => {
        $(impl From<$entity> for Entity {
            fn from(value: $entity) -> Entity{
                Entity::$entity(value)
            }
        })*
    };
}

impl_into_entity!(Light, Object, Camera);

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub position: UVec2,
    pub text: String,
    pub color: Vec4,
    pub scale: f32,
}

impl Text {
    pub fn white(position: UVec2, text: String) -> Self {
        Self {
            position,
            text,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            scale: 1.0,
        }
    }

    pub fn black(position: UVec2, text: String) -> Self {
        Self {
            position,
            text,
            color: vec4(0.0, 0.0, 0.0, 1.0),
            scale: 1.0,
        }
    }

    pub fn with_color(position: UVec2, text: String, color: Vec4) -> Self {
        Self {
            position,
            text,
            color,
            scale: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    pub camera: SceneNodeId,
    pub scene_graph: SceneGraph,
    pub terrain: Vec<SparseTensorChunk>,
    pub text: Vec<Text>,
    has_materials: bool,
    materials: Box<[Material; 256]>,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        let mut scene_graph = SceneGraph::new();
        let camera_id = scene_graph.insert_entity(camera, &scene_graph.root());

        Self {
            camera: camera_id,
            scene_graph,
            terrain: Vec::default(),
            text: Vec::default(),
            has_materials: false,
            materials: Box::new([Material::default(); 256]),
        }
    }

    pub fn camera(&self) -> &Camera {
        let entity = self.scene_graph.entity(&self.camera).unwrap();
        match entity {
            Entity::Camera(camera) => camera,
            _ => panic!("camera is not a Entity::Camera"),
        }
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        let entity = self.scene_graph.entity_mut(&self.camera).unwrap();
        match entity {
            Entity::Camera(camera) => camera,
            _ => panic!("camera is not a Entity::Camera"),
        }
    }

    pub fn has_materials(&self) -> bool {
        self.has_materials
    }

    pub fn materials(&self) -> &[Material] {
        self.materials.as_slice()
    }

    pub fn set_materials(&mut self, materials: Box<[Material; 256]>) {
        self.has_materials = true;
        self.materials = materials;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    position: Vec3,
    transform: Mat4,
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
            transform: Mat4::from_translation(position),
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

#[derive(Debug)]
pub struct SceneNodeId(usize);

#[derive(Debug)]
pub struct SceneNode {
    parent: SceneNodeId,
    base_entity: Entity,
    mutated_entity: Entity,
}

impl Clone for SceneNode {
    fn clone(&self) -> Self {
        Self {
            parent: SceneNodeId(self.parent.0),
            base_entity: self.base_entity.clone(),
            mutated_entity: self.mutated_entity.clone(),
        }
    }
}

impl SceneNode {
    fn new(entity: Entity, parent: &SceneNodeId) -> Self {
        Self {
            parent: SceneNodeId(parent.0),
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
#[derive(Debug)]
pub struct SceneGraph {
    nodes: Vec<Option<SceneNode>>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self { nodes: vec![None] }
    }

    pub fn insert_entity(
        &mut self,
        entity: impl Into<Entity>,
        parent: &SceneNodeId,
    ) -> SceneNodeId {
        self.nodes.push(Some(SceneNode::new(entity.into(), parent)));
        SceneNodeId(self.nodes.len() - 1)
    }

    pub fn entity(&self, id: &SceneNodeId) -> Option<&Entity> {
        self.nodes[id.0].as_ref().map(|s| &s.base_entity)
    }

    pub fn object_mut(&mut self, id: &SceneNodeId) -> Option<&mut Object> {
        let entity = self.entity_mut(id);
        match entity {
            Some(Entity::Object(object)) => Some(object),
            Some(_) => panic!("Found {id:?}, but wasn't an object"),
            _ => None,
        }
    }

    pub fn entity_mut(&mut self, id: &SceneNodeId) -> Option<&mut Entity> {
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

    pub fn root(&self) -> SceneNodeId {
        SceneNodeId(0)
    }

    pub fn mutated_entity(&self, id: &SceneNodeId) -> Option<&Entity> {
        self.nodes[id.0].as_ref().map(|s| &s.mutated_entity)
    }

    pub fn mutated_entities(&self) -> impl Iterator<Item = &Entity> {
        self.nodes
            .iter()
            .filter_map(|node| node.as_ref().map(|node| &node.mutated_entity))
    }
}

#[test]
fn graph() {
    let mut g = SceneGraph::new();
    let root = g.root();

    let transform = Mat4::from_cols_array_2d(&[[1., 2., 3., 4.]; 4]);

    let a = g.insert_entity(
        Object {
            transform,
            model: Model::default(),
            tag: None,
        },
        &root,
    );
    let b = g.insert_entity(
        Object {
            transform,
            model: Model::default(),
            tag: None,
        },
        &a,
    );

    g.evaluate_all();

    assert_eq!(
        g.mutated_entity(&b).unwrap().transform().unwrap(),
        &(transform * transform)
    );
}

use glam::*;

/// A chunk is a cube with consisting of `x` by `y` by `z` voxels.
#[derive(Debug, Clone)]
pub struct Chunk {
    transform: Mat4,
    positions: Vec<(u8, u8, u8)>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    transform: Mat4,
    chunk: Option<Chunk>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub camera: Camera,
    pub chunks: Vec<Chunk>,
    pub entities: Vec<Entity>,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            chunks: Vec::default(),
            entities: Vec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

use glam::*;

pub struct Scene {
    pub camera: Camera,
}

pub struct Camera {
    position: Vec3,
    direction: Vec3,
    view: Mat4,
    projection: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        let direction = Vec3::new(0.0, 0.0, 1.0);

        Self {
            position,
            direction,
            view: Mat4::look_at_rh(position, position + direction, Vec3::new(0.0, 1.0, 0.0)),
            projection: Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_2, aspect, 0.1, 100.0),
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
}

use glam::*;

use crate::rhi::*;
use crate::scene::*;

pub struct TextRenderer {}

impl TextRenderer {
    pub fn new(device: Device, window_size: UVec2) -> Self {
        todo!()
    }

    pub fn render(&mut self, scene: &mut Scene) {}

    pub fn resize(&mut self, window_size: UVec2) {}
}

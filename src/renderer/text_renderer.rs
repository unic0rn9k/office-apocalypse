use glam::*;

use crate::rhi::*;
use crate::scene::*;

pub struct TextRenderer<'a> {
    device: Device<'a>,
}

impl<'a> TextRenderer<'a> {
    pub fn new(device: Device<'a>, window_size: UVec2) -> Self {
        TextRenderer { device }
    }

    pub fn render(&mut self, scene: &mut Scene) {}

    pub fn resize(&mut self, window_size: UVec2) {}
}

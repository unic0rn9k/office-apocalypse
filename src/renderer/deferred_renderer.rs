use glam::*;

use super::{CubeVertex, QuadVertex, CUBE, QUAD};
use crate::rhi::*;
use crate::scene::*;

unsafe impl BufferLayout for [Mat4; 2] {
    const LAYOUT: &'static [Format] = &[Format::Mat4, Format::Mat4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

unsafe impl BufferLayout for Light {
    const LAYOUT: &'static [Format] = &[Format::Vec4, Format::Vec4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = false;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(items.len() * Self::stride());

        for item in items {
            let position: [u8; 12] = unsafe { std::mem::transmute(item.position) };
            bytes.extend(position);
            bytes.extend(1.0f32.to_ne_bytes());

            let color: [u8; 12] = unsafe { std::mem::transmute(item.color) };
            bytes.extend(color);
            bytes.extend(1.0f32.to_ne_bytes());
        }

        bytes
    }
}

pub struct DeferredRenderer<'a> {
    device: Device<'a>,
    vertex_buffer: Buffer<CubeVertex, false, false>,
    quad_buffer: Buffer<QuadVertex, false, false>,
    matrix_buffer: Buffer<[Mat4; 2], false, true>,
    material_buffer: Buffer<Material, false, true>,
    light_buffer: Buffer<Light, false, true>,
    program: ShaderProgram,
    lighting_program: ShaderProgram,
}

impl<'a> DeferredRenderer<'a> {
    const DS_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds.vert");
    const DS_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds.frag");
    const DS_LIGHTING_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.vert");
    const DS_LIGHTING_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.frag");

    // The maximum amount of chunks that can be grouped into a single drawcall.
    //
    // OpenGL is required to support at least 16384 bytes for uniform buffers.
    // MAX_CHUNKS * (2 * std::mem::size_of::<Mat4>()) < 16384
    const MAX_CHUNKS: usize = 170;

    // The maximum amount of materials that can be used at any given time.
    const MAX_MATERIALS: usize = 256;

    // The maximum amount of lights that can be used at any given time.
    const MAX_LIGHTS: usize = 256;

    pub fn new(device: Device<'a>, window_size: UVec2) -> Self {
        let vertex_buffer = device.new_buffer(BufferInit::Data(&CUBE));
        let quad_buffer = device.new_buffer(BufferInit::Data(&QUAD));
        let matrix_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_CHUNKS));
        let material_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_MATERIALS));
        let light_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_LIGHTS));

        let program = {
            let vertex_shader = device.new_shader(VertexStage, Self::DS_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::DS_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let lighting_program = {
            let vertex_shader = device.new_shader(VertexStage, Self::DS_LIGHTING_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::DS_LIGHTING_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        Self {
            device,
            vertex_buffer,
            quad_buffer,
            matrix_buffer,
            material_buffer,
            light_buffer,
            program,
            lighting_program,
        }
    }

    pub fn render(&mut self, scene: &mut Scene) {}

    pub fn resize(&mut self, window_size: UVec2) {}
}

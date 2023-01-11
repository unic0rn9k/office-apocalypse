use std::time::Instant;

use glam::*;
use sdl2::video::*;

use crate::rhi::*;
use crate::scene::*;

pub struct Cache {
    vertices: Buffer<Vec3>,
    indices: Buffer<u32>,
    matrices: Buffer<Mat4, false, true>,
    materials: Option<Buffer<Material>>,
    buffers: Vec<Buffer<Vec3, false, true>>,
}

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    shaders: ShaderProgram,
    cache: Cache,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/shader.vert");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/shader.frag");

    #[rustfmt::skip]
    const VERTICES: [Vec3; 8] = [
        Vec3::new(-1.0, -1.0,  1.0),
        Vec3::new( 1.0, -1.0,  1.0),
        Vec3::new( 1.0,  1.0,  1.0),
        Vec3::new(-1.0,  1.0,  1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new( 1.0, -1.0, -1.0),
        Vec3::new( 1.0,  1.0, -1.0),
        Vec3::new(-1.0,  1.0, -1.0),
    ];

    #[rustfmt::skip]
    const INDICES: [u32; 36] = [
        0, 1, 2,
    	2, 3, 0,
    	1, 5, 6,
    	6, 2, 1,
    	7, 6, 5,
    	5, 4, 7,
    	4, 0, 3,
    	3, 7, 4,
    	4, 5, 1,
    	1, 0, 4,
    	3, 2, 6,
    	6, 7, 3
    ];

    pub fn new(window: &Window, vsync: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(1, vsync);

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);
        let shaders = device.new_shader_program(&vertex_shader, &pixel_shader);

        let vertices = device.new_buffer(BufferInit::Data(&Self::VERTICES));
        let indices = device.new_buffer(BufferInit::Data(&Self::INDICES));
        let matrices = device.new_buffer(BufferInit::Capacity(2));

        Self {
            _instance,
            device,
            swapchain,
            shaders,
            cache: Cache {
                vertices,
                indices,
                matrices,
                materials: None,
                buffers: Vec::default(),
            },
        }
    }

    /// Renders a single frame
    pub fn run(&mut self, scene: &Scene) {
        let Self { device, cache, .. } = self;

        unsafe { gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT)) }.unwrap();

        if cache.materials.is_none() {
            cache.materials = Some(device.new_buffer(BufferInit::Data(scene.materials())));
        }

        let view_projection = scene.camera.view_projection();

        // TODO: Batch multiple chunks into a single drawcall.
        for chunk in scene.terrain() {
            self.render_chunk(view_projection, chunk);
        }

        self.swapchain.present();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        unsafe { gl!(gl::Viewport(0, 0, width as _, height as _)) }.unwrap();
    }

    fn render_chunk(&mut self, view_projection: Mat4, chunk: &Chunk) {
        let Self { device, cache, .. } = self;
        let Cache { matrices, .. } = cache;

        let materials = cache
            .materials
            .as_ref()
            .expect("Materials haven't been uploaded to the GPU");

        let mvp = view_projection * chunk.transform;
        matrices.map_write().write(&[chunk.transform, mvp]);

        let offsets: Vec<_> = chunk.positions.iter().map(|(offset, _)| *offset).collect();
        let offsets: Buffer<_> = device.new_buffer(BufferInit::Data(&offsets));

        let material_ids: Vec<_> = chunk.positions.iter().map(|(_, id)| id.0 as u32).collect();
        let material_ids: Buffer<_, false> = device.new_buffer(BufferInit::Data(&material_ids));

        assert_eq!(offsets.len(), material_ids.len());

        unsafe {
            let mut vao = 0;
            gl!(gl::CreateVertexArrays(1, &mut vao)).unwrap();

            gl!(gl::VertexArrayVertexBuffers(
                vao,
                0,
                3,
                [cache.vertices.id, offsets.id, material_ids.id].as_ptr(),
                [0, 0, 0].as_ptr(),
                [12, 12, 4].as_ptr()
            ))
            .unwrap();

            gl!(gl::VertexArrayElementBuffer(vao, cache.indices.id)).unwrap();

            gl!(gl::EnableVertexArrayAttrib(vao, 0)).unwrap();
            gl!(gl::EnableVertexArrayAttrib(vao, 1)).unwrap();
            gl!(gl::EnableVertexArrayAttrib(vao, 2)).unwrap();

            gl!(gl::VertexArrayAttribFormat(
                vao,
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                0
            ))
            .unwrap();

            gl!(gl::VertexArrayAttribFormat(
                vao,
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                0
            ))
            .unwrap();

            gl!(gl::VertexArrayAttribIFormat(vao, 2, 1, gl::UNSIGNED_INT, 0)).unwrap();

            gl!(gl::VertexArrayAttribBinding(vao, 0, 0)).unwrap();
            gl!(gl::VertexArrayAttribBinding(vao, 1, 1)).unwrap();
            gl!(gl::VertexArrayAttribBinding(vao, 2, 2)).unwrap();
            gl!(gl::VertexArrayBindingDivisor(vao, 1, 1)).unwrap();
            gl!(gl::VertexArrayBindingDivisor(vao, 2, 1)).unwrap();

            gl!(gl::UseProgram(self.shaders.id)).unwrap();
            gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, matrices.id)).unwrap();
            gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 1, materials.id)).unwrap();

            gl!(gl::BindVertexArray(vao)).unwrap();

            gl!(gl::DrawElementsInstanced(
                gl::TRIANGLES,
                cache.indices.len() as _,
                gl::UNSIGNED_INT,
                std::ptr::null(),
                offsets.len() as _
            ))
            .unwrap();

            gl!(gl::DeleteVertexArrays(1, &vao)).unwrap();
        }
    }
}

unsafe impl BufferLayout for Material {
    const LAYOUT: &'static [Format] = &[Format::Vec4, Format::F32, Format::F32];
    const PADDING: &'static [usize] = &[0, 0, 8];
    const COPYABLE: bool = false;

    // OpenGL require that arrays are aligned to a multiple of 16.
    // Since the material contains a total of 24 bytes, the next multiple is 32.
    // Because of that we must add 8 empty bytes at the end of our material.
    fn to_bytes(items: &[Self]) -> Vec<u8> {
        // assert_eq!(std::mem::size_of::<Self>(), 24);

        let mut bytes: Vec<u8> = Vec::with_capacity(items.len() * std::mem::size_of::<Self>());
        for item in items {
            let albedo = {
                let [r, g, b, a] = item.albedo;
                Vec4::new(r as _, g as _, b as _, a as _) / 255.0
            };

            for component in albedo.as_ref() {
                bytes.extend_from_slice(&component.to_ne_bytes()) // 4 bytes * 4
            }

            bytes.extend_from_slice(&item.roughness.to_ne_bytes()); // 4 bytes
            bytes.extend_from_slice(&item.metalness.to_ne_bytes()); // 4 bytes
            bytes.extend_from_slice(&[0; 8]); // 8 bytes
        }

        bytes
    }
}

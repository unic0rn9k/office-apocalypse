use glam::*;
use sdl2::video::*;

use crate::rhi::*;
use crate::scene::*;

#[derive(Default)]
pub struct Cache {
    vertex_buffer: Option<Buffer<Vec3, false, false>>,
    index_buffer: Option<Buffer<u32, false, false>>,
    matrix_buffer: Option<Buffer<Mat4, false, true>>,
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

        Self {
            _instance,
            device,
            swapchain,
            shaders,
            cache: Cache::default(),
        }
    }

    /// Renders a single frame
    pub fn run(&mut self, scene: &Scene) {
        let Self { device, cache, .. } = self;

        let vertex_buffer = match cache.vertex_buffer.take() {
            Some(buffer) => buffer,
            _ => device.new_buffer(BufferInit::Data(&Self::VERTICES)),
        };

        let index_buffer = match cache.index_buffer.take() {
            Some(buffer) => buffer,
            _ => device.new_buffer(BufferInit::Data(&Self::INDICES)),
        };

        let mut matrix_buffer: Buffer<_, false, true> = match cache.matrix_buffer.take() {
            Some(buffer) => buffer,
            _ => device.new_buffer(BufferInit::Capacity(std::mem::size_of::<Mat4>() * 2)),
        };

        unsafe { gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT)).unwrap() };
        for chunk in &scene.terrain {
            let mvp = scene.camera.view_projection() * chunk.transform;
            matrix_buffer.map_write().write(&[mvp]);

            let offsets: Vec<_> = chunk
                .positions
                .iter()
                .map(|&(position, _)| position)
                .collect();

            let offset_buffer: Buffer<_, false, false> =
                device.new_buffer(BufferInit::Data(&offsets));

            unsafe {
                let mut vao = 0;
                gl!(gl::CreateVertexArrays(1, &mut vao)).unwrap();
                gl!(gl::VertexArrayVertexBuffer(vao, 0, vertex_buffer.id, 0, 12)).unwrap();
                gl!(gl::VertexArrayVertexBuffer(vao, 1, offset_buffer.id, 0, 12)).unwrap();
                gl!(gl::VertexArrayElementBuffer(vao, index_buffer.id)).unwrap();

                gl!(gl::EnableVertexArrayAttrib(vao, 0)).unwrap();

                gl!(gl::VertexArrayAttribFormat(
                    vao,
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    0
                ))
                .unwrap();

                gl!(gl::VertexArrayAttribBinding(vao, 0, 0)).unwrap();

                gl!(gl::EnableVertexArrayAttrib(vao, 1)).unwrap();

                gl!(gl::VertexArrayAttribFormat(
                    vao,
                    1,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    0
                ))
                .unwrap();

                gl!(gl::VertexArrayAttribBinding(vao, 1, 1)).unwrap();
                gl!(gl::VertexArrayBindingDivisor(vao, 1, 1)).unwrap();

                gl!(gl::UseProgram(self.shaders.id)).unwrap();
                gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, matrix_buffer.id)).unwrap();

                gl!(gl::BindVertexArray(vao)).unwrap();

                gl!(gl::DrawElementsInstanced(
                    gl::TRIANGLES,
                    Self::INDICES.len() as _,
                    gl::UNSIGNED_INT,
                    std::ptr::null(),
                    offset_buffer.len() as _,
                ))
                .unwrap();

                gl!(gl::DeleteVertexArrays(1, &vao)).unwrap();
            }
        }

        self.swapchain.present();

        cache.vertex_buffer = Some(vertex_buffer);
        cache.index_buffer = Some(index_buffer);
        cache.matrix_buffer = Some(matrix_buffer);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        unsafe { gl!(gl::Viewport(0, 0, width as _, height as _)) }.unwrap();
    }
}

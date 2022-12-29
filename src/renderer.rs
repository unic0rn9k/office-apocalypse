use glam::*;
use sdl2::video::*;

use crate::rhi::*;

pub struct Renderer<'a> {
    instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    shaders: ShaderProgram,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/vertex_shader.glsl");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/pixel_shader.glsl");

    pub fn new(window: &Window) -> Self {
        let instance = Instance::new(window, true);
        let device = instance.new_device();
        let swapchain = instance.new_swapchain(1);

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);
        let shaders = device.new_shader_program(&vertex_shader, &pixel_shader);

        Self {
            instance,
            device,
            swapchain,
            shaders,
        }
    }

    pub fn run(&mut self) {
        // unsafe { gl::ClearColor(1.0, 0.0, 0.0, 1.0) };
        // unsafe { gl::Clear(gl::COLOR_BUFFER_BIT) };

        #[rustfmt::skip]
        const VERTICES: [Vertex; 4] = [
            Vertex::new(Vec3::new(0.5, 0.5, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            Vertex::new(Vec3::new( 0.5, -0.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            Vertex::new(Vec3::new(-0.5,  -0.5, 0.0),  Vec3::new(0.0, 0.0, 1.0)),
            Vertex::new(Vec3::new(-0.5,  0.5, 0.0),  Vec3::new(0.0, 0.0, 1.0)),
        ];

        #[rustfmt::skip]
        const INDICES: [u32; 6] = [
            0, 1, 3,
            1, 2, 3
        ];

        let vb: Buffer<_, false, false> = self.device.new_buffer(BufferInit::Data(&VERTICES));
        let ib: Buffer<_, false, false> = self.device.new_buffer(BufferInit::Data(&INDICES));

        self.device.set_vertex_buffers(&[&vb]);
        self.device.set_index_buffer(&ib);
        self.device.set_shaders(&self.shaders);
        self.device.draw_indexed(INDICES.len());

        self.swapchain.present();
    }
}

struct Vertex {
    position: Vec3,
    color: Vec3,
}

impl Vertex {
    pub const fn new(position: Vec3, color: Vec3) -> Self {
        Self { position, color }
    }
}

unsafe impl BufferLayout for Vertex {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(0, gl::FLOAT), (1, gl::FLOAT)];
}

// unsafe impl Layout for Vertex {}

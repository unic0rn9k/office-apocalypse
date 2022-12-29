use glam::*;
use sdl2::video::*;

use crate::rhi::*;

pub struct Renderer<'a> {
    instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/vertex_shader.glsl");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/pixel_shader.glsl");

    pub fn new(window: &Window) -> Self {
        let instance = Instance::new(window);
        let device = instance.new_device(true);
        let swapchain = instance.new_swapchain(1);

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);

        Self {
            instance,
            device,
            swapchain,
        }
    }

    pub fn run(&mut self) {
        unsafe { gl::ClearColor(1.0, 0.0, 0.0, 1.0) };
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT) };

        #[rustfmt::skip]
        const VERTICES: [Vertex; 3] = [
            Vertex::new(Vec3::new(-0.5, -0.5, 0.0)),
            Vertex::new(Vec3::new( 0.5, -0.5, 0.0)),
            Vertex::new(Vec3::new(0.0,  0.5, 0.0))
        ];

        let vb: Buffer<_, false, false> = self.device.new_buffer(BufferInit::Data(&VERTICES));

        // let mut vao = 0;
        // unsafe { gl::CreateVertexArrays(1, &mut vao) };

        // unsafe { gl::BindVertexBuffer(0, vb, 0, std::mem::size_of::<[f32; 3]>() as
        // i32) };

        // unsafe { gl::BindVertexArray(vao) };
        // unsafe { gl::GetBin}

        // unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 2) };

        self.device.set_vertex_buffers(&[&vb]);
        self.device.draw();

        self.swapchain.present();
    }
}

struct Vertex {
    position: Vec3,
}

impl Vertex {
    pub const fn new(position: Vec3) -> Self {
        Self { position }
    }
}

// unsafe impl Layout for Vertex {}

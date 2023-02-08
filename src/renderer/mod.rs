use glam::*;
use sdl2::video::*;

use self::profiler::*;
use crate::rhi::*;
use crate::scene::*;

mod profiler;

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    framebuffer: Option<Framebuffer>,
    shaders: ShaderProgram,

    window_size: UVec2,
    profiler: Profiler,
}

impl Renderer<'_> {
    const VERTEX_SHADER_SRC: &str = include_str!("./shaders/shader.vert");
    const PIXEL_SHADER_SRC: &str = include_str!("./shaders/shader.frag");

    pub fn new(window: &Window, vsync: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(vsync);

        let window_size = UVec2::from(window.size());

        let framebuffer = {
            let window_width = window_size.x as _;
            let window_height = window_size.y as _;
            let texture = device.new_texture_2d(window_width, window_height, Format::R8G8B8A8);
            let depth = device.new_texture_2d(window_width, window_height, Format::D24);
            Some(device.new_framebuffer(texture, Some(depth)))
        };

        let shaders = {
            let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let profiler = Profiler::new(false);

        Self {
            _instance,
            device,
            swapchain,
            window_size,
            framebuffer,
            shaders,
            profiler,
        }
    }

    pub fn render(&mut self, scene: &mut Scene) -> Option<f64> {
        let mut framebuffer = self
            .framebuffer
            .take()
            .expect("Failed to retrieve framebuffer");

        let mut default_framebuffer = self.device.default_framebuffer();

        framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0), true);
        default_framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0), false);

        self.geometry_pass(scene, &mut framebuffer);
        self.postprocess_pass(&mut framebuffer);
        self.text_pass(&mut framebuffer);

        // Copy the image from the framebuffer to the default framebuffer and present
        // the image.
        let Self { device, .. } = self;
        device.blit(&framebuffer, &mut default_framebuffer, false);

        self.swapchain.present();

        Some(1.0)
    }

    pub fn resize(&mut self, window_size: UVec2) {
        self.window_size = window_size;
    }

    fn geometry_pass(&mut self, scene: &mut Scene, framebuffer: &mut Framebuffer) {
        let Self { device, .. } = self;

        scene.entities.evaluate_all();

        for entity in scene.entities.mutated_entities() {}

        device.bind_vertex_buffer(BindProps {
            attributes: [0, 1]
        })

        device.bind_framebuffer(framebuffer);
        device.draw_instanced(vertices, instances);
    }

    fn postprocess_pass(&mut self, framebuffer: &mut Framebuffer) {}

    fn text_pass(&mut self, framebuffer: &mut Framebuffer) {}
}

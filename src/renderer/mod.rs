use glam::*;
use sdl2::video::Window;

use self::deferred_renderer::*;
use self::text_renderer::*;
use crate::rhi::*;
use crate::scene::*;

mod deferred_renderer;
mod text_renderer;

#[repr(C)]
struct QuadVertex(Vec2, Vec2);

unsafe impl BufferLayout for QuadVertex {
    const LAYOUT: &'static [Format] = &[Format::Vec2, Format::Vec2];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

#[rustfmt::skip]
const QUAD: [QuadVertex; 6] = [
    QuadVertex(vec2(-1.0,  1.0), vec2(0.0, 1.0)),
    QuadVertex(vec2( 1.0,  1.0), vec2(1.0, 1.0)),
    QuadVertex(vec2(-1.0, -1.0), vec2(0.0, 0.0)),
    QuadVertex(vec2( 1.0,  1.0), vec2(1.0, 1.0)),
    QuadVertex(vec2( 1.0, -1.0), vec2(1.0, 0.0)),
    QuadVertex(vec2(-1.0, -1.0), vec2(0.0, 0.0)),
];

#[repr(C)]
struct CubeVertex(Vec4, Vec4);

unsafe impl BufferLayout for CubeVertex {
    const LAYOUT: &'static [Format] = &[Format::Vec4, Format::Vec4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

#[rustfmt::skip]
const CUBE: [CubeVertex; 36] = [
    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),
    CubeVertex(vec4( 0.5, -0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0),  vec4(0.0,  0.0, -1.0, 0.0)),

    CubeVertex(vec4(-0.5, -0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),
    CubeVertex(vec4( 0.5, -0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5,  0.5, 1.0),  vec4(0.0,  0.0,  1.0, 0.0)),

    CubeVertex(vec4(-0.5,  0.5,  0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5, -0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5,  0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5,  0.5, 1.0), vec4(-1.0,  0.0,  0.0, 0.0)),

    CubeVertex(vec4(0.5,  0.5,  0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(0.5,  0.5, -0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(0.5, -0.5, -0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(0.5, -0.5, -0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(0.5, -0.5,  0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),
    CubeVertex(vec4(0.5,  0.5,  0.5, 1.0),  vec4(1.0,  0.0,  0.0, 0.0)),

    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5, -0.5, -0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5, -0.5,  0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5, -0.5,  0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5,  0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5, -0.5, -0.5, 1.0),  vec4(0.0, -1.0,  0.0, 0.0)),

    CubeVertex(vec4(-0.5,  0.5, -0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5, -0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5,  0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0)),
    CubeVertex(vec4( 0.5,  0.5,  0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5,  0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0)),
    CubeVertex(vec4(-0.5,  0.5, -0.5, 1.0),  vec4(0.0,  1.0,  0.0, 0.0))
];

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    deferred_renderer: DeferredRenderer<'a>,
    text_renderer: TextRenderer<'a>,
}

impl<'a> Renderer<'a> {
    pub fn new(window: &Window, vsync: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(vsync);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let window_size = UVec2::from(window.size());

        Self {
            _instance,
            device: device.clone(),
            swapchain,
            deferred_renderer: DeferredRenderer::new(device.clone(), window_size),
            text_renderer: TextRenderer::new(device.clone(), window_size),
        }
    }

    pub fn render(&mut self, scene: &mut Scene) -> Option<f32> {
        let Self {
            device,
            swapchain,
            deferred_renderer,
            text_renderer,
            ..
        } = self;

        scene.scene_graph.evaluate_all();

        device
            .default_framebuffer()
            .clear(vec4(0.0, 0.0, 0.0, 1.0), true);

        deferred_renderer.render(scene);
        // text_renderer.render(scene);

        // device.unbind_framebuffer();
        swapchain.present();

        Some(1.0)
    }

    pub fn resize(&mut self, window_size: UVec2) {
        let Self {
            deferred_renderer,
            text_renderer,
            ..
        } = self;

        unsafe { gl::Viewport(0, 0, window_size.x as _, window_size.y as _) };

        deferred_renderer.resize(window_size);
        text_renderer.resize(window_size);
    }
}

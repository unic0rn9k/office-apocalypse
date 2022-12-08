use std::marker::PhantomData;

use sdl2::video::*;

use crate::rhi::*;

pub struct Renderer {
    instance: Instance,
    device: Device,
    swapchain: Swapchain,
}

impl Renderer {
    const SHADER: &'static str = include_str!("./shaders/shader.glsl");

    pub fn new(window: &Window) -> Self {
        let instance = Instance::new(window);
        let device = instance.new_device();
        let swapchain = instance.new_swapchain(1);

        Self {
            instance,
            device,
            swapchain,
        }
    }

    pub fn run(&mut self) {
        unsafe { gl::ClearColor(1.0, 0.0, 0.0, 1.0) };
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT) };
        self.swapchain.present();
    }
}

use std::ffi::*;
use std::marker::PhantomData;
use std::rc::Rc;

use sdl2::video::*;

pub struct InstanceShared {
    window_context: Rc<WindowContext>,
    _context: GLContext,
}

pub struct Instance(Rc<InstanceShared>);

impl Instance {
    pub fn new(window: &Window) -> Self {
        let _context = window.gl_create_context().unwrap();
        gl::load_with(|s| window.subsystem().gl_get_proc_address(s) as *const _);
        Self(Rc::new(InstanceShared {
            window_context: window.context(),
            _context,
        }))
    }

    pub fn new_device(&self) -> Device {
        Device {
            _instance: Rc::clone(&self.0),
        }
    }

    pub fn new_swapchain(&self, nframebuffers: usize) -> Swapchain {
        let mut framebuffers = vec![0; nframebuffers];
        unsafe { gl::CreateFramebuffers(nframebuffers as i32, framebuffers.as_mut_ptr()) };

        let framebuffers = framebuffers.into_iter().map(Framebuffer).collect();

        Swapchain {
            _instance: Rc::clone(&self.0),
            window: unsafe { Window::from_ref(Rc::clone(&self.0.window_context)) },
            framebuffers,
        }
    }
}

pub struct Device {
    _instance: Rc<InstanceShared>,
}

impl Device {
    pub fn new_buffer<T>(&self, capacity: usize, data: Option<&[T]>) -> Buffer<T> {
        let mut handle = 0;
        unsafe { gl::CreateBuffers(1, &mut handle) };

        unsafe {
            gl::NamedBufferStorage(
                handle,
                capacity as isize,
                std::ptr::null(),
                gl::MAP_READ_BIT | gl::MAP_WRITE_BIT,
            )
        };

        unsafe { gl::MapNamedBuffer(handle, gl::WRITE_ONLY) };

        todo!()
    }

    pub fn submit(&self) {
        todo!()
    }

    // pub fn new_texture(&self, width: usize, height: usize) -> Texture {
    //     let mut handle = 0;
    //     unsafe { gl::CreateTextures(gl::TEXTURE_2D, 1, &mut handle) };

    //     unsafe { gl::TextureStorage2D(handle, 2, gl::SRGB8, width as isize,
    // height as isize) }; }
}

pub struct Swapchain {
    _instance: Rc<InstanceShared>,
    window: Window,
    framebuffers: Vec<Framebuffer>,
}

impl Swapchain {
    pub fn present(&mut self) {
        self.window.gl_swap_window();
    }

    pub fn framebuffers(&self) -> &[Framebuffer] {
        &self.framebuffers
    }
}

pub struct Framebuffer(u32);

pub trait Resource {
    type Item;

    fn handle(&self) -> u32;

    fn len(&self) -> usize;
}

pub struct Buffer<T> {
    handle: u32,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> Buffer<T> {
    pub fn map_read(&self) -> MapRead<Self> {
        MapRead::new(self)
    }

    pub fn map_write(&mut self) -> MapWrite<Self> {
        MapWrite::new(self)
    }
}

pub struct Texture;

impl<T> Resource for Buffer<T> {
    type Item = T;

    fn handle(&self) -> u32 {
        self.handle
    }

    fn len(&self) -> usize {
        self.len
    }
}

pub struct MapRead<'a, R: Resource>(&'a R);

impl<'a, R: Resource> MapRead<'a, R> {
    pub fn new(resource: &'a R) -> Self {
        unsafe { gl::MapNamedBuffer(resource.handle(), gl::MAP_READ_BIT) };
        Self(resource)
    }
}

impl<'a, R: Resource> Drop for MapRead<'a, R> {
    fn drop(&mut self) {
        unsafe { gl::UnmapNamedBuffer(self.0.handle()) };
    }
}

pub struct MapWrite<'a, R: Resource>(&'a mut R, *mut c_void);

impl<'a, R: Resource> MapWrite<'a, R> {
    pub fn new(resource: &'a mut R) -> Self {
        let ptr = unsafe { gl::MapNamedBuffer(resource.handle(), gl::MAP_WRITE_BIT) };
        Self(resource, ptr)
    }

    pub fn write(&mut self) {}
}

impl<'a, R: Resource> Drop for MapWrite<'a, R> {
    fn drop(&mut self) {
        unsafe { gl::UnmapNamedBuffer(self.0.handle()) };
    }
}

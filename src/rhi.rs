use std::cell::*;
use std::ffi::*;
use std::marker::*;
use std::rc::*;

use sdl2::video::*;

macro_rules! gl {
    ($f: expr) => {{
        let value = $f;
        let error = gl::GetError();
        let result = match error {
            gl::NO_ERROR => Ok(value),
            gl::INVALID_ENUM
            | gl::INVALID_VALUE
            | gl::INVALID_OPERATION
            | gl::INVALID_FRAMEBUFFER_OPERATION
            | gl::OUT_OF_MEMORY
            | gl::STACK_UNDERFLOW
            | gl::STACK_OVERFLOW => Err(error),
            #[allow(unused_unsafe)]
            _ => unsafe { std::hint::unreachable_unchecked() },
        };

        result
    }};
}

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

    pub fn new_device<'a>(&self, debug: bool) -> Device<'a> {
        if debug {}

        let shared = DeviceShared {
            vbos: Vec::with_capacity(16),
            ibos: Vec::with_capacity(16),
            _instance: Rc::clone(&self.0),
        };

        Device(Rc::new(RefCell::new(shared)), PhantomData)
    }

    pub fn new_swapchain(&self, nframebuffers: usize) -> Swapchain {
        let mut framebuffers = vec![0; nframebuffers];
        let n = nframebuffers as i32;
        unsafe { gl!(gl::CreateFramebuffers(n, framebuffers.as_mut_ptr())) }.unwrap();

        let framebuffers = framebuffers.into_iter().map(Framebuffer).collect();

        Swapchain {
            _instance: Rc::clone(&self.0),
            window: unsafe { Window::from_ref(Rc::clone(&self.0.window_context)) },
            framebuffers,
        }
    }
}

struct DeviceShared {
    vbos: Vec<u32>,
    ibos: Vec<u32>,
    _instance: Rc<InstanceShared>,
}

pub struct Device<'a>(Rc<RefCell<DeviceShared>>, PhantomData<&'a ()>);

impl<'a> Device<'a> {
    pub fn new_buffer<T, const R: bool, const W: bool>(&self, b: BufferInit<T>) -> Buffer<T, R, W> {
        let mut vbo = 0;
        unsafe { gl::CreateBuffers(1, &mut vbo) };

        let mut flags = if R { gl::MAP_READ_BIT } else { 0 };
        W.then(|| flags |= gl::MAP_WRITE_BIT);

        let (capacity, data) = match b {
            BufferInit::Data(_data) => todo!(),
            BufferInit::Capacity(capacity) => (capacity, std::ptr::null()),
        };

        unsafe { gl!(gl::NamedBufferStorage(vbo, capacity as isize, data, flags)) }.unwrap();

        Buffer {
            vbo,
            capacity,
            len: 0,
            _device: Rc::clone(&self.0),
            _marker: PhantomData,
        }
    }

    pub fn set_vertex_buffers<T, const R: bool, const W: bool>(&self, bufs: &'a [Buffer<T, R, W>]) {
        let mut device = self.0.borrow_mut();
        device.vbos.clear();
        device.vbos.extend(bufs.into_iter().map(|b| b.vbo));
    }

    pub fn set_index_buffers<const R: bool, const W: bool>(&self, bufs: &'a [Buffer<u32, R, W>]) {
        let mut device = self.0.borrow_mut();
        device.ibos.clear();
        device.ibos.extend(bufs.into_iter().map(|b| b.vbo));
    }

    pub fn submit(&self) {
        todo!()
    }
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

pub trait BufferApi {
    fn len(&self) -> usize;
}

pub enum BufferInit<'a, T> {
    Data(&'a [T]),
    Capacity(usize),
}

pub struct Buffer<T, const R: bool, const W: bool> {
    vbo: u32,
    capacity: usize,
    len: usize,
    _device: Rc<RefCell<DeviceShared>>,
    _marker: PhantomData<T>,
}

impl<T, const W: bool> Buffer<T, true, W> {
    pub fn map_read(&self) -> MapRead<Self> {
        MapRead(self)
    }
}

impl<T, const R: bool> Buffer<T, R, true> {
    pub fn map_write(&mut self) -> MapWrite<Self> {
        MapWrite(self)
    }
}

impl<T, const R: bool, const W: bool> BufferApi for Buffer<T, R, W> {
    fn len(&self) -> usize {
        self.len
    }
}

pub struct MapRead<'a, B: BufferApi>(&'a B);

pub struct MapWrite<'a, B: BufferApi>(&'a mut B);

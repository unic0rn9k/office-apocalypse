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
        if debug {
            unsafe { gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS) }
            unsafe { gl::DebugMessageCallback(Some(Self::debug_callback), std::ptr::null()) };
        }

        let shared = DeviceShared {
            vbos: Vec::with_capacity(16),
            ibo: None,
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

    extern "system" fn debug_callback(
        src: u32,
        _type: u32,
        id: u32,
        sev: u32,
        len: i32,
        msg: *const i8,
        _: *mut c_void,
    ) {
        let msg = unsafe { CStr::from_ptr(msg) }.to_str().unwrap();
        println!("{msg}");
    }
}

struct DeviceShared {
    vbos: Vec<u32>,
    ibo: Option<u32>,
    _instance: Rc<InstanceShared>,
}

pub struct Device<'a>(Rc<RefCell<DeviceShared>>, PhantomData<&'a ()>);

impl<'a> Device<'a> {
    pub fn new_buffer<T, const R: bool, const W: bool>(&self, b: BufferInit<T>) -> Buffer<T, R, W> {
        let mut id = 0;
        unsafe { gl::CreateBuffers(1, &mut id) };

        let mut flags = if R { gl::MAP_READ_BIT } else { 0 };
        W.then(|| flags |= gl::MAP_WRITE_BIT);

        let (capacity, data) = match b {
            BufferInit::Data(data) => (data.len(), data.as_ptr() as *const _),
            BufferInit::Capacity(capacity) => (capacity, std::ptr::null()),
        };

        let size = (std::mem::size_of::<T>() * capacity) as isize;
        unsafe { gl!(gl::NamedBufferStorage(id, size, data, flags)) }.unwrap();

        Buffer {
            id,
            capacity,
            len: 0,
            _device: Rc::clone(&self.0),
            _marker: PhantomData,
        }
    }

    pub fn new_shader<S: Stage>(&self, stage: S, src: &str) -> Shader<S> {
        let stage = match S::STAGE_TYPE {
            StageType::VertexStage => gl::VERTEX_SHADER,
            StageType::PixelStage => gl::FRAGMENT_SHADER,
        };

        let id = unsafe { gl!(gl::CreateShader(stage)) }.unwrap();

        let string = &(src.as_ptr() as *const _);
        let length = src.len() as _;
        unsafe { gl!(gl::ShaderSource(id, 1, string, &length)) }.unwrap();

        unsafe { gl!(gl::CompileShader(id)) }.unwrap();

        let mut success = 0;
        unsafe { gl!(gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success)) }.unwrap();
        if success != 1 {
            let mut msg: [u8; 512] = [0; 512];
            unsafe {
                gl::GetShaderInfoLog(
                    id,
                    msg.len() as _,
                    std::ptr::null_mut(),
                    msg.as_mut_ptr() as *mut _,
                )
            };

            let s = std::str::from_utf8(msg.as_slice()).unwrap();
            panic!("{s}");
        }

        Shader {
            id,
            _marker: PhantomData,
        }
    }

    pub fn set_vertex_buffers<T, const R: bool, const W: bool>(
        &self,
        bufs: &[&'a Buffer<T, R, W>],
    ) {
        let mut device = self.0.borrow_mut();
        device.vbos.clear();
        device.vbos.extend(bufs.into_iter().map(|b| b.id));
    }

    pub fn set_index_buffer<const R: bool, const W: bool>(&self, buf: &'a Buffer<u32, R, W>) {
        let mut device = self.0.borrow_mut();
        device.ibo = Some(buf.id);
    }

    pub fn draw(&self) {
        unsafe { gl!(gl::DrawArrays(gl::TRIANGLES, 0, 3)) }.unwrap();
    }

    pub fn draw_indexed(&self) {}
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
    id: u32,
    capacity: usize,
    len: usize,
    _device: Rc<RefCell<DeviceShared>>,
    _marker: PhantomData<T>,
}

impl<T, const R: bool, const W: bool> Drop for Buffer<T, R, W> {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.id) }
    }
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

pub enum StageType {
    VertexStage,
    PixelStage,
}

pub trait Stage {
    const STAGE_TYPE: StageType;
}

pub struct VertexStage;
impl Stage for VertexStage {
    const STAGE_TYPE: StageType = StageType::VertexStage;
}

pub struct PixelStage;
impl Stage for PixelStage {
    const STAGE_TYPE: StageType = StageType::PixelStage;
}

pub struct Shader<S: Stage> {
    id: u32,
    _marker: PhantomData<S>,
}

impl<S: Stage> Drop for Shader<S> {
    fn drop(&mut self) {
        unsafe { gl::DeleteShader(self.id) }
    }
}

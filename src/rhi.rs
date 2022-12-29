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
    pub fn new(window: &Window, debug: bool) -> Self {
        let _context = window.gl_create_context().unwrap();
        gl::load_with(|s| window.subsystem().gl_get_proc_address(s) as *const _);

        if debug {
            unsafe { gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS) }
            unsafe { gl::DebugMessageCallback(Some(Self::debug_callback), std::ptr::null()) };
        }

        Self(Rc::new(InstanceShared {
            window_context: window.context(),
            _context,
        }))
    }

    pub fn new_device<'a>(&self) -> Device<'a> {
        let mut vao = 0;
        unsafe { gl!(gl::CreateVertexArrays(1, &mut vao)) }.unwrap();

        let shared = DeviceShared {
            vao,
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
    vao: u32,
    _instance: Rc<InstanceShared>,
}

pub struct Device<'a>(Rc<RefCell<DeviceShared>>, PhantomData<&'a ()>);

impl<'a> Device<'a> {
    pub fn new_buffer<T, const R: bool, const W: bool>(&self, b: BufferInit<T>) -> Buffer<T, R, W>
    where
        T: BufferLayout,
    {
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

    pub fn new_shader<S: Stage>(&self, _stage: S, src: &str) -> Shader<S> {
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
                let _ = gl!(gl::GetShaderInfoLog(
                    id,
                    msg.len() as _,
                    std::ptr::null_mut(),
                    msg.as_mut_ptr() as *mut _,
                ));
            };

            let s = std::str::from_utf8(msg.as_slice()).unwrap();
            panic!("{s}");
        }

        Shader(Rc::new(ShaderShared {
            id,
            _marker: PhantomData,
            _device: Rc::clone(&self.0),
        }))
    }

    pub fn new_shader_program(&self, vs: &VertexShader, ps: &PixelShader) -> ShaderProgram {
        let id = unsafe { gl::CreateProgram() };
        unsafe { gl!(gl::AttachShader(id, vs.0.id)) }.unwrap();
        unsafe { gl!(gl::AttachShader(id, ps.0.id)) }.unwrap();
        unsafe { gl!(gl::LinkProgram(id)) }.unwrap();

        let mut success = 0;
        unsafe { gl!(gl::GetProgramiv(id, gl::LINK_STATUS, &mut success)) }.unwrap();
        if success != 1 {
            let mut msg: [u8; 512] = [0; 512];
            unsafe {
                let _ = gl!(gl::GetProgramInfoLog(
                    id,
                    msg.len() as _,
                    std::ptr::null_mut(),
                    msg.as_mut_ptr() as *mut _,
                ));
            };

            let s = std::str::from_utf8(msg.as_slice()).unwrap();
            panic!("{s}");
        }

        ShaderProgram { id }
    }

    // TODO(Bech): Allow for multiple buffers with differen layouts.
    pub fn set_vertex_buffers<T, const R: bool, const W: bool>(&self, bufs: &[&'a Buffer<T, R, W>])
    where
        T: BufferLayout,
    {
        let device = self.0.borrow();

        let ids: Vec<u32> = bufs.iter().map(|buf| buf.id).collect();
        let strides = vec![std::mem::size_of::<T>() as _; ids.len()];
        let offsets = vec![0; ids.len()];

        unsafe {
            gl!(gl::VertexArrayVertexBuffers(
                device.vao,
                0,
                ids.len() as _,
                ids.as_ptr(),
                offsets.as_ptr(),
                strides.as_ptr()
            ))
            .unwrap();
        }

        unsafe {
            gl!(gl::EnableVertexArrayAttrib(device.vao, 0)).unwrap();
            gl!(gl::EnableVertexArrayAttrib(device.vao, 1)).unwrap();

            gl!(gl::VertexArrayAttribFormat(
                device.vao,
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                0
            ))
            .unwrap();

            gl!(gl::VertexArrayAttribFormat(
                device.vao,
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                (3 * std::mem::size_of::<f32>()) as _,
            ))
            .unwrap();

            gl!(gl::VertexArrayAttribBinding(device.vao, 0, 0)).unwrap();
            gl!(gl::VertexArrayAttribBinding(device.vao, 1, 0)).unwrap();
        }
    }

    pub fn set_index_buffer<const R: bool, const W: bool>(&self, buf: &'a Buffer<u32, R, W>) {
        let device = self.0.borrow();
        unsafe { gl!(gl::VertexArrayElementBuffer(device.vao, buf.id)) }.unwrap();
    }

    pub fn set_shaders(&self, program: &'a ShaderProgram) {
        let device = self.0.borrow();
        unsafe { gl!(gl::UseProgram(program.id)) }.unwrap();
    }

    pub fn draw(&self, vertices: usize) {
        let device = self.0.borrow();
        unsafe { gl!(gl::BindVertexArray(device.vao)) }.unwrap();
        unsafe { gl!(gl::DrawArrays(gl::TRIANGLES, 0, vertices as _)) }.unwrap();
    }

    pub fn draw_indexed(&self, indices: usize) {
        let device = self.0.borrow();
        unsafe { gl!(gl::BindVertexArray(device.vao)) }.unwrap();

        unsafe {
            gl!(gl::DrawElements(
                gl::TRIANGLES,
                indices as _,
                gl::UNSIGNED_INT,
                std::ptr::null()
            ))
        }
        .unwrap();
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

pub unsafe trait BufferLayout: Sized {
    const LAYOUT: &'static [(usize, gl::types::GLenum)];

    // fn to_bytes(&self) -> Vec<u8>;
}

unsafe impl BufferLayout for u32 {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(0, gl::UNSIGNED_INT)];
}

pub trait BufferApi {
    fn len(&self) -> usize;
}

pub enum BufferInit<'a, T> {
    Data(&'a [T]),
    Capacity(usize),
}

pub struct Buffer<T: BufferLayout, const R: bool, const W: bool> {
    id: u32,
    capacity: usize,
    len: usize,
    _device: Rc<RefCell<DeviceShared>>,
    _marker: PhantomData<T>,
}

impl<T, const R: bool, const W: bool> Drop for Buffer<T, R, W>
where
    T: BufferLayout,
{
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.id) }
    }
}

impl<T, const W: bool> Buffer<T, true, W>
where
    T: BufferLayout,
{
    pub fn map_read(&self) -> MapRead<Self> {
        MapRead(self)
    }
}

impl<T, const R: bool> Buffer<T, R, true>
where
    T: BufferLayout,
{
    pub fn map_write(&mut self) -> MapWrite<Self> {
        MapWrite(self)
    }
}

impl<T, const R: bool, const W: bool> BufferApi for Buffer<T, R, W>
where
    T: BufferLayout,
{
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

pub struct ShaderShared<S: Stage> {
    id: u32,
    _device: Rc<RefCell<DeviceShared>>,
    _marker: PhantomData<S>,
}

impl<S: Stage> Drop for ShaderShared<S> {
    fn drop(&mut self) {
        let _ = unsafe { gl!(gl::DeleteShader(self.id)) };
    }
}

pub struct Shader<S: Stage>(Rc<ShaderShared<S>>);

pub type VertexShader = Shader<VertexStage>;
pub type PixelShader = Shader<PixelStage>;

pub struct ShaderProgram {
    id: u32,
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        let _ = unsafe { gl!(gl::DeleteProgram(self.id)) };
    }
}

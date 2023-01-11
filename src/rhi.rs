use std::cell::*;
use std::ffi::*;
use std::marker::*;
use std::rc::*;

use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};
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

pub use gl;

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

        unsafe { gl!(gl::Enable(gl::DEPTH_TEST)) }.unwrap();
        unsafe { gl!(gl::DepthFunc(gl::LEQUAL)) }.unwrap();

        let shared = DeviceShared {
            vao,
            _instance: Rc::clone(&self.0),
        };

        Device(Rc::new(RefCell::new(shared)), PhantomData)
    }

    pub fn new_swapchain(&self, nframebuffers: usize, vsync: bool) -> Swapchain {
        let interval = if vsync {
            SwapInterval::VSync
        } else {
            SwapInterval::Immediate
        };

        let window = unsafe { Window::from_ref(Rc::clone(&self.0.window_context)) };
        let _ = window.subsystem().gl_set_swap_interval(interval);

        let mut framebuffers = vec![0; nframebuffers];
        let n = nframebuffers as i32;
        unsafe { gl!(gl::CreateFramebuffers(n, framebuffers.as_mut_ptr())) }.unwrap();

        let framebuffers = framebuffers.into_iter().map(Framebuffer).collect();

        Swapchain {
            _instance: Rc::clone(&self.0),
            window,
            framebuffers,
        }
    }

    extern "system" fn debug_callback(
        _src: u32,
        _type: u32,
        _id: u32,
        _sev: u32,
        _len: i32,
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

        let bytes;
        let (size, capacity, data, len) = match b {
            BufferInit::Data(data) => {
                if T::COPYABLE {
                    (
                        data.len() * std::mem::size_of::<T>(),
                        data.len(),
                        data.as_ptr() as *const _,
                        data.len(),
                    )
                } else {
                    bytes = T::to_bytes(data);
                    (bytes.len(), data.len(), bytes.as_ptr(), data.len())
                }
            }
            BufferInit::Capacity(capacity) => (
                capacity * (std::mem::size_of::<T>() + T::padding()),
                capacity,
                std::ptr::null(),
                0,
            ),
        };

        unsafe {
            gl!(gl::NamedBufferStorage(
                id,
                size as isize,
                data as *const _,
                flags
            ))
        }
        .unwrap();

        Buffer {
            id,
            capacity,
            len,
            _device: Rc::clone(&self.0),
            _marker: PhantomData,
        }
    }

    pub fn new_shader<S: Stage>(&self, _stage: S, src: &str) -> Shader<S> {
        let stage = match S::STAGE_TYPE {
            StageType::Vertex => gl::VERTEX_SHADER,
            StageType::Geometry => gl::GEOMETRY_SHADER,
            StageType::Pixel => gl::FRAGMENT_SHADER,
        };

        let id = unsafe { gl!(gl::CreateShader(stage)) }.unwrap();

        let string = &(src.as_ptr() as *const _);
        unsafe { gl!(gl::ShaderSource(id, 1, string, [src.len() as _].as_ptr())) }.unwrap();

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

    /// Binds vertex buffers to the device.
    pub fn bind_vertex_buffer<T, const R: bool, const W: bool>(&mut self, props: BindProps<T, R, W>)
    where
        T: BufferLayout,
    {
        let DeviceShared { vao, .. } = &*self.0.borrow();
        let binding = props.binding as _;
        let id = props.buffer.id;
        let stride = T::stride() as _;
        unsafe {
            gl!(gl::VertexArrayVertexBuffer(*vao, binding, id, 0, stride)).unwrap();
        }

        for (i, attrib) in props.attributes.iter().enumerate() {
            let format = &T::LAYOUT[i];
            let attrib = *attrib as _;
            unsafe {
                gl!(gl::EnableVertexArrayAttrib(*vao, attrib)).unwrap();
                gl!(gl::VertexArrayAttribBinding(*vao, attrib, binding)).unwrap();
            }

            let (size, type_, normalized) = match format {
                Format::F32 => (1, gl::FLOAT, gl::FALSE),
                Format::Vec2 => (2, gl::FLOAT, gl::FALSE),
                Format::Vec3 => (3, gl::FLOAT, gl::FALSE),
                Format::Vec4 => (4, gl::FLOAT, gl::FALSE),
                Format::Mat3 => (12, gl::FLOAT, gl::FALSE),
                Format::Mat4 => (16, gl::FLOAT, gl::FALSE),
                Format::U32 => (4, gl::UNSIGNED_INT, gl::FALSE),
            };

            if type_ == gl::UNSIGNED_INT {
                unsafe {
                    gl!(gl::VertexArrayAttribIFormat(*vao, attrib, size, type_, 0)).unwrap();
                }
            } else {
                unsafe {
                    gl!(gl::VertexArrayAttribFormat(
                        *vao, attrib, size, type_, normalized, 0
                    ))
                    .unwrap();
                }
            }

            if props.instanced {
                unsafe {
                    gl!(gl::VertexArrayBindingDivisor(*vao, binding, 1)).unwrap();
                }
            }
        }
    }

    pub fn bind_index_buffer<const R: bool, const W: bool>(&self, buf: &'a Buffer<u32, R, W>) {
        let device = self.0.borrow();
        unsafe { gl!(gl::VertexArrayElementBuffer(device.vao, buf.id)) }.unwrap();
    }

    pub fn bind_shader_program(&self, program: &'a ShaderProgram) {
        let _device = self.0.borrow();
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

    pub fn draw_instanced(&self, vertices: usize, instances: usize) {
        let device = self.0.borrow();

        unsafe {
            gl!(gl::BindVertexArray(device.vao)).unwrap();

            gl!(gl::DrawArraysInstanced(
                gl::TRIANGLES,
                0,
                vertices as _,
                instances as _
            ))
            .unwrap()
        }
    }

    pub fn draw_indexed_instanced(&self, indices: usize, instances: usize) {
        let device = self.0.borrow();

        unsafe {
            gl!(gl::BindVertexArray(device.vao)).unwrap();

            gl!(gl::DrawElementsInstanced(
                gl::TRIANGLES,
                indices as _,
                gl::UNSIGNED_INT,
                std::ptr::null(),
                instances as _
            ))
            .unwrap()
        }
    }
}

pub struct BindProps<'a, T: BufferLayout, const R: bool, const W: bool> {
    pub binding: usize,
    pub attributes: &'a [usize],
    pub buffer: &'a Buffer<T, R, W>,
    pub instanced: bool,
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

pub enum Format {
    F32,
    Vec2,
    Vec3,
    Vec4,
    Mat3,
    Mat4,
    U32,
}

///
///
/// # Safety
pub unsafe trait BufferLayout: Sized {
    const LAYOUT: &'static [Format];
    const PADDING: &'static [usize];
    const COPYABLE: bool = false;

    /// Computes the amount of bytes (stride) of each element in the buffer
    ///
    /// Includes the size of the padding.
    fn stride() -> usize {
        let format_to_size = |format: &Format| match format {
            Format::F32 => 4,
            Format::Vec2 => 8,
            Format::Vec3 => 12,
            Format::Vec4 => 16,
            Format::Mat3 => 32,
            Format::Mat4 => 48,
            Format::U32 => 4,
        };

        let size: usize = Self::LAYOUT.iter().map(format_to_size).sum();
        size + Self::padding()
    }

    fn padding() -> usize {
        Self::PADDING.iter().sum()
    }

    /// Computes the offset between elements for the attribute located at
    /// `index` in bytes
    ///
    /// Includes the size of the padding up to `index`, but not after.
    // TODO(Bech): Probably not working...
    fn offset(index: usize) -> usize {
        let format_to_size = |format: &Format| match format {
            Format::F32 => 4,
            Format::Vec2 => 8,
            Format::Vec3 => 12,
            Format::Vec4 => 16,
            Format::Mat3 => 32,
            Format::Mat4 => 48,
            Format::U32 => 4,
        };

        let size: usize = Self::LAYOUT[0..index + 1].iter().map(format_to_size).sum();
        let padding: usize = Self::PADDING[0..index].iter().sum();
        Self::stride() - size + padding
    }

    // TODO: Refactor to Box<[]> avoid heap allocations yes yes
    fn to_bytes(items: &[Self]) -> Vec<u8>;
}

macro_rules! generate_layouts {
    ([$($layout:ident => $format:ident),+]) => {
        $(
            unsafe impl BufferLayout for $layout {
                const LAYOUT: &'static [Format] = &[Format::$format];
                const PADDING: &'static [usize] = &[0];
                const COPYABLE: bool = true;

                fn to_bytes(_items: &[Self]) -> Vec<u8> {
                    unimplemented!()
                }
            }
        )+
    };
}

generate_layouts!([
    f32 => F32,
    Vec2 => Vec2,
    Vec3 => Vec3,
    Vec4 => Vec4,
    Mat3 => Mat3,
    Mat4 => Mat4,
    u32 => U32
]);

pub enum BufferInit<'a, T: BufferLayout> {
    Data(&'a [T]),
    Capacity(usize),
}

pub struct Buffer<T: BufferLayout, const R: bool = false, const W: bool = false> {
    pub id: u32,
    capacity: usize,
    len: usize,
    _device: Rc<RefCell<DeviceShared>>,
    _marker: PhantomData<T>,
}

impl<T: BufferLayout, const R: bool, const W: bool> Buffer<T, R, W> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T: BufferLayout, const R: bool, const W: bool> Drop for Buffer<T, R, W> {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.id) }
    }
}

impl<T: BufferLayout, const W: bool> Buffer<T, true, W> {
    pub fn map_read(&self) -> MapRead<T, W> {
        MapRead(self)
    }
}

impl<T: BufferLayout, const R: bool> Buffer<T, R, true> {
    pub fn map_write(&mut self) -> MapWrite<T, R> {
        MapWrite(self)
    }
}

pub struct MapRead<'a, T: BufferLayout, const W: bool>(&'a Buffer<T, true, W>);

impl<'a, T: BufferLayout + Default + Clone, const W: bool> MapRead<'a, T, W> {
    pub fn read(&self) -> Vec<T> {
        let mapped = unsafe { gl!(gl::MapNamedBuffer(self.0.id, gl::READ_ONLY)) }.unwrap();
        if T::COPYABLE {
            let mut storage = vec![T::default(); self.0.len()];
            unsafe { std::ptr::copy(mapped as *const _, storage.as_mut_ptr(), storage.len()) };
            storage
        } else {
            todo!()
        }
    }
}

impl<'a, T: BufferLayout, const W: bool> Drop for MapRead<'a, T, W> {
    fn drop(&mut self) {
        unsafe { gl!(gl::UnmapNamedBuffer(self.0.id)) }.unwrap();
    }
}

pub struct MapWrite<'a, T: BufferLayout, const R: bool>(&'a mut Buffer<T, R, true>);

impl<'a, T: BufferLayout, const R: bool> MapWrite<'a, T, R> {
    pub fn write(&self, items: &[T]) {
        let buffer = &self.0;
        assert!(buffer.capacity() >= items.len());

        let mapped = unsafe { gl!(gl::MapNamedBuffer(buffer.id, gl::WRITE_ONLY)) }.unwrap();
        if T::COPYABLE {
            let count = items.len() * std::mem::size_of::<T>();
            unsafe { std::ptr::copy(items.as_ptr() as *const _, mapped, count) };
        } else {
            let bytes = T::to_bytes(items);
            unsafe { std::ptr::copy(bytes.as_ptr() as *const _, mapped, bytes.len()) };
        }
    }
}

impl<'a, T: BufferLayout, const R: bool> Drop for MapWrite<'a, T, R> {
    fn drop(&mut self) {
        unsafe { gl!(gl::UnmapNamedBuffer(self.0.id)) }.unwrap();
    }
}

pub enum StageType {
    Vertex,
    Geometry,
    Pixel,
}

pub trait Stage {
    const STAGE_TYPE: StageType;
}

pub struct VertexStage;
impl Stage for VertexStage {
    const STAGE_TYPE: StageType = StageType::Vertex;
}

pub struct GeometryStage;
impl Stage for GeometryStage {
    const STAGE_TYPE: StageType = StageType::Geometry;
}

pub struct PixelStage;
impl Stage for PixelStage {
    const STAGE_TYPE: StageType = StageType::Pixel;
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
pub type GeometryShader = Shader<GeometryStage>;
pub type PixelShader = Shader<PixelStage>;

pub struct ShaderProgram {
    pub id: u32,
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        let _ = unsafe { gl!(gl::DeleteProgram(self.id)) };
    }
}

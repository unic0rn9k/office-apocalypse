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
            program: 0,
            _instance: Rc::clone(&self.0),
        };

        Device(Rc::new(RefCell::new(shared)), PhantomData)
    }

    pub fn new_swapchain(&self, vsync: bool) -> Swapchain {
        let interval = if vsync {
            SwapInterval::VSync
        } else {
            SwapInterval::Immediate
        };

        let window = unsafe { Window::from_ref(Rc::clone(&self.0.window_context)) };
        let _ = window.subsystem().gl_set_swap_interval(interval);

        Swapchain {
            _instance: Rc::clone(&self.0),
            window,
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
    program: u32,
    _instance: Rc<InstanceShared>,
}

#[derive(Clone)]
pub struct Device<'a>(Rc<RefCell<DeviceShared>>, PhantomData<&'a ()>);

impl<'a> Device<'a> {
    pub fn default_framebuffer(&self) -> Framebuffer {
        let mut _device = self.0.borrow_mut();

        Framebuffer {
            id: 0,
            textures: Vec::default(),
            depth: None,
            default: true,
        }
    }

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
            BufferInit::Capacity(capacity) => {
                (capacity * (T::stride()), capacity, std::ptr::null(), 0)
            }
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

    pub fn new_texture_2d(&self, width: usize, height: usize, format: Format) -> Texture2D {
        let mut id = u32::MAX;

        let internal = match format {
            Format::R8G8B8A8 => gl::RGBA8,
            Format::D24 => gl::DEPTH_COMPONENT24,
            Format::R32G32B32A32Float => gl::RGBA32F,
            Format::R32G32Float => gl::RG32F,
            Format::R32Uint => gl::R32UI,
            _ => panic!("Textures can only be created with texture compatible formats!"),
        };

        unsafe {
            gl!(gl::CreateTextures(gl::TEXTURE_2D, 1, &mut id)).unwrap();

            gl!(gl::TextureStorage2D(
                id,
                1,
                internal,
                width as _,
                height as _
            ))
            .unwrap();
        }

        Texture2D {
            id,
            width,
            height,
            format,
            _device: Rc::clone(&self.0),
        }
    }

    pub fn new_framebuffer<const N: usize>(&self, attachments: [Attachment; N]) -> Framebuffer {
        let mut id = u32::MAX;
        unsafe { gl!(gl::CreateFramebuffers(1, &mut id)).unwrap() };

        let mut textures = Vec::from_iter((0..attachments.len()).map(|_| None));
        let mut depth = None;
        for attachment in attachments {
            let (texture, attachment) = match attachment {
                Attachment::Color(texture, index) => {
                    assert!(Format::TEXTURE_COMPATIBLE.contains(&texture.format));
                    let texture_id = texture.id;
                    textures[index] = Some(texture);
                    (texture_id, gl::COLOR_ATTACHMENT0 + index as u32)
                }
                Attachment::Depth(texture) => {
                    assert!(Format::DEPTH_COMPATIBLE.contains(&texture.format));
                    assert!(depth.is_none());
                    let texture_id = texture.id;
                    depth = Some(texture);
                    (texture_id, gl::DEPTH_ATTACHMENT)
                }
            };

            unsafe { gl!(gl::NamedFramebufferTexture(id, attachment, texture, 0)) }.unwrap();
        }

        if unsafe {
            gl!(gl::CheckNamedFramebufferStatus(id, gl::FRAMEBUFFER)).unwrap()
                != gl::FRAMEBUFFER_COMPLETE
        } {
            panic!("Framebuffer is not complete");
        }

        let points: Vec<_> = textures
            .iter()
            .enumerate()
            .filter_map(|(index, texture)| texture.as_ref().map(|_| index))
            .map(|index| gl::COLOR_ATTACHMENT0 + index as u32)
            .collect();

        // println!("{points:?}");

        unsafe {
            gl!(gl::NamedFramebufferDrawBuffers(
                id,
                points.len() as _,
                points.as_ptr()
            ))
        }
        .unwrap();

        Framebuffer {
            id,
            textures,
            depth,
            default: false,
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
        unsafe {
            gl!(gl::AttachShader(id, vs.0.id)).unwrap();
            gl!(gl::AttachShader(id, ps.0.id)).unwrap();
            gl!(gl::LinkProgram(id)).unwrap();
        }

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
        let DeviceShared { vao, program, .. } = &*self.0.borrow();
        let binding = props.binding as _;
        let id = props.buffer.id;
        let stride = T::stride() as _;
        unsafe {
            gl!(gl::VertexArrayVertexBuffer(*vao, binding, id, 0, stride)).unwrap();
        }

        for (i, attrib) in props.attributes.iter().enumerate() {
            let format = &T::LAYOUT[i];

            let name = CString::new(*attrib).unwrap();
            let location = unsafe { gl!(gl::GetAttribLocation(*program, name.as_ptr())) }.unwrap();

            unsafe {
                gl!(gl::EnableVertexArrayAttrib(*vao, location as _)).unwrap();
                gl!(gl::VertexArrayAttribBinding(*vao, location as _, binding)).unwrap();
            }

            let offset = T::offset(i);

            let (size, type_, normalized) = match format {
                Format::F32 => (1, gl::FLOAT, gl::FALSE),
                Format::Vec2 => (2, gl::FLOAT, gl::FALSE),
                Format::UVec2 => (2, gl::UNSIGNED_INT, gl::FALSE),
                Format::IVec2 => (2, gl::INT, gl::FALSE),
                Format::Vec3 => (3, gl::FLOAT, gl::FALSE),
                Format::Vec4 => (4, gl::FLOAT, gl::FALSE),
                Format::Mat3 => (12, gl::FLOAT, gl::FALSE),
                Format::Mat4 => (16, gl::FLOAT, gl::FALSE),
                Format::U32 => (1, gl::UNSIGNED_INT, gl::FALSE),
                Format::U16 => (1, gl::UNSIGNED_SHORT, gl::FALSE),
                _ => panic!("Format is not supported in vertex buffer"),
            };

            unsafe {
                if [gl::UNSIGNED_INT, gl::INT, gl::UNSIGNED_SHORT].contains(&type_) {
                    gl!(gl::VertexArrayAttribIFormat(
                        *vao,
                        location as _,
                        size,
                        type_,
                        offset as _
                    ))
                    .unwrap();
                } else {
                    gl!(gl::VertexArrayAttribFormat(
                        *vao,
                        location as _,
                        size,
                        type_,
                        normalized,
                        offset as _
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
        let mut device = self.0.borrow_mut();
        device.program = program.id;

        unsafe { gl!(gl::UseProgram(program.id)) }.unwrap();
    }

    pub fn bind_uniform_buffer<T, const R: bool, const W: bool>(
        &self,
        buf: &'a Buffer<T, R, W>,
        binding: usize,
    ) where
        T: BufferLayout,
    {
        let device = self.0.borrow_mut();
        unsafe { gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, binding as _, buf.id)) }.unwrap();
    }

    pub fn bind_texture_2d(&self, texture: &'a Texture2D, name: &str, location: usize) {
        let device = self.0.borrow_mut();
        let name = CString::new(name).unwrap();
        unsafe {
            gl!(gl::ActiveTexture(gl::TEXTURE0 + location as u32)).unwrap();
            gl!(gl::BindTexture(gl::TEXTURE_2D, texture.id)).unwrap();
            let uniform = gl::GetUniformLocation(device.program, name.as_ptr());
            gl!(gl::Uniform1i(uniform, location.try_into().unwrap())).unwrap();
        }
    }

    pub fn bind_framebuffer(&self, framebuffer: &'a mut Framebuffer) {
        let _device = self.0.borrow();
        unsafe { gl!(gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer.id)) }.unwrap();
    }

    pub fn unbind_framebuffer(&mut self) {
        let _device = self.0.borrow_mut();
        unsafe { gl!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0)) }.unwrap();
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
            .unwrap();

            gl!(gl::BindVertexArray(0)).unwrap();
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

    pub fn blit(&self, src: (&Framebuffer, usize), dst: (&mut Framebuffer, usize), depth: bool) {
        let _device = self.0.borrow();

        // If `src` is the default framebuffer we must query the size of the
        // viewport using glGetIntegerV. The same holds true for `dst`.
        //     let [src_width, src_height] = if let Some(texture) = &src.texture
        // {         [texture.width as _, texture.height as _]
        //     } else {
        //         let mut buf = [0, 0, 0, 0];
        //         unsafe { gl!(gl::GetIntegerv(gl::VIEWPORT, buf.as_mut_ptr()))
        // }.unwrap();         [buf[2], buf[3]]
        //     };

        //     let [dst_width, dst_height] = if let Some(texture) = &dst.texture
        // {         [texture.width as _, texture.height as _]
        //     } else {
        //         let mut buf = [0, 0, 0, 0];
        //         unsafe { gl!(gl::GetIntegerv(gl::VIEWPORT, buf.as_mut_ptr()))
        // }.unwrap();         [buf[2], buf[3]]
        //     };

        //     assert!(src_width <= dst_width);
        //     assert!(src_height <= dst_height);

        //     println!("src: {src_width},{src_height}");
        //     println!("dst: {dst_width},{dst_height}");

        //     unsafe {
        //         gl!(gl::BlitNamedFramebuffer(
        //             src.id,
        //             dst.id,
        //             0,
        //             0,
        //             src_width,
        //             src_height,
        //             0,
        //             0,
        //             dst_height,
        //             dst_width,
        //             gl::COLOR_BUFFER_BIT | if depth { gl::DEPTH_BUFFER_BIT }
        // else { 0 },             gl::NEAREST
        //         ))
        //     }
        //     .unwrap();
    }
}

pub struct Texture2D {
    pub id: u32,
    width: usize,
    height: usize,
    format: Format,
    _device: Rc<RefCell<DeviceShared>>,
}

impl Texture2D {
    pub fn write(&mut self, bytes: &[u8]) {
        assert_eq!(
            bytes.len(),
            self.width * self.height * std::mem::size_of::<u8>() * 4
        );

        unsafe {
            gl!(gl::TextureSubImage2D(
                self.id,
                0,
                0,
                0,
                self.width as _,
                self.height as _,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                bytes.as_ptr() as *const _
            ))
            .unwrap()
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn format(&self) -> Format {
        self.format
    }
}

impl Drop for Texture2D {
    fn drop(&mut self) {
        unsafe { gl!(gl::DeleteTextures(1, &mut self.id)).unwrap() };
    }
}

pub struct BindProps<'a, T: BufferLayout, const R: bool, const W: bool> {
    pub binding: usize,
    pub attributes: &'a [&'static str],
    pub buffer: &'a Buffer<T, R, W>,
    pub instanced: bool,
}

pub struct Swapchain {
    _instance: Rc<InstanceShared>,
    window: Window,
}

impl Swapchain {
    pub fn present(&mut self) {
        self.window.gl_swap_window();
    }
}

pub enum Attachment {
    Color(Texture2D, usize),
    Depth(Texture2D),
}

pub struct Framebuffer {
    pub id: u32,
    textures: Vec<Option<Texture2D>>,
    depth: Option<Texture2D>,
    default: bool,
}

impl Framebuffer {
    pub fn clear(&mut self, color: Vec4, depth: bool) {
        if self.default {
            unsafe {
                gl!(gl::ClearNamedFramebufferfv(
                    self.id,
                    gl::COLOR,
                    0 as i32,
                    color.as_ref().as_ptr()
                ))
            }
            .unwrap()
        }

        for (i, texture) in self.textures.iter().enumerate() {
            if texture.is_none() {
                continue;
            }

            unsafe {
                gl!(gl::ClearNamedFramebufferfv(
                    self.id,
                    gl::COLOR,
                    i as i32,
                    color.as_ref().as_ptr()
                ))
            }
            .unwrap()
        }

        if depth {
            unsafe {
                gl!(gl::ClearNamedFramebufferfv(
                    self.id,
                    gl::DEPTH,
                    0,
                    [1.0].as_ptr()
                ))
            }
            .unwrap();
        }
    }

    pub fn color(&self, index: usize) -> &Texture2D {
        let Self { id, textures, .. } = self;
        assert!(
            *id != 0,
            "Tried to access a color attachment for the default framebuffer"
        );

        textures[index].as_ref().unwrap()
    }

    pub fn color_mut(&mut self, index: usize) -> &mut Texture2D {
        let Self { id, textures, .. } = self;
        assert!(
            self.id != 0,
            "Tried to access a color attachment for the default framebuffer"
        );

        textures[index].as_mut().unwrap()
    }

    pub fn depth(&self) -> &Texture2D {
        assert!(
            self.id != 0,
            "Tried to access depth attachment for default framebuffer"
        );

        todo!()
    }

    pub fn depth_mut(&self) -> &mut Texture2D {
        assert!(
            self.id != 0,
            "Tried to access depth attachment for default framebuffer"
        );

        todo!()
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if !self.default {
            let _ = unsafe { gl!(gl::DeleteFramebuffers(1, &self.id)) };
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    R8G8B8A8,
    R32G32B32A32Float,
    R32Uint,
    R32G32Float,
    D24,

    F32,

    Vec2,
    UVec2,
    IVec2,

    Vec3,
    Vec4,
    Mat3,
    Mat4,
    U32,
    U16,
}

impl Format {
    const TEXTURE_COMPATIBLE: &[Self] = &[
        Self::R8G8B8A8,
        Self::R32G32B32A32Float,
        Self::R32G32Float,
        Self::R32Uint,
    ];

    const DEPTH_COMPATIBLE: &[Self] = &[Self::D24];
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
            Format::U32 => 4,
            Format::U16 => 2,

            Format::Vec2 => 8,
            Format::UVec2 => 8,
            Format::IVec2 => 8,

            Format::Vec3 => 12,
            Format::Vec4 => 16,
            Format::Mat3 => 32,
            Format::Mat4 => 48,

            Format::R8G8B8A8
            | Format::R32G32B32A32Float
            | Format::R32G32Float
            | Format::D24
            | Format::R32Uint => {
                panic!("{format:?} can't be used in buffers.")
            }
        };

        let size: usize = Self::LAYOUT.iter().map(format_to_size).sum();
        size + Self::padding()
    }

    fn padding() -> usize {
        Self::PADDING.iter().sum()
    }

    /// Computes the offset from the start of the buffer to the attribute
    /// located at `index`.
    // TODO(Bech): Probably not working...
    fn offset(index: usize) -> usize {
        if index == 0 {
            return 0;
        }

        let format_to_size = |format: &Format| match format {
            Format::F32 => 4,
            Format::U32 => 4,
            Format::U16 => 2,

            Format::Vec2 => 8,
            Format::UVec2 => 8,
            Format::IVec2 => 8,

            Format::Vec3 => 12,
            Format::Vec4 => 16,
            Format::Mat3 => 32,
            Format::Mat4 => 48,

            Format::R8G8B8A8
            | Format::R32G32B32A32Float
            | Format::R32G32Float
            | Format::D24
            | Format::R32Uint => {
                panic!("{format:?} can't be used in buffers.")
            }
        };

        let size: usize = Self::LAYOUT[0..index].iter().map(format_to_size).sum();
        let padding: usize = Self::PADDING[0..index].iter().sum();
        size + padding
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
    u32 => U32,
    u16 => U16
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
    /// Returns the amount of elements in the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the maximum amount of elements there is space for in the buffer
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
    pub fn write(&mut self, items: &[T]) {
        let buffer = &mut self.0;
        assert!(buffer.capacity() >= items.len());

        buffer.len = items.len();

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

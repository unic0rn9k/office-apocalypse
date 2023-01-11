use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Instant;

use glam::*;
use sdl2::video::*;

use crate::rhi::*;
use crate::scene::*;

struct Cache {
    vertices: Buffer<Vertex>,
    matrices: Buffer<Mat4, false, true>,
    materials: Option<Buffer<Material>>,
    buffers: Vec<Buffer<Vec3, false, true>>,
}

struct Profiler {
    task: Option<&'static str>,
    cpu_profiler: (Option<Instant>, Option<Instant>),
    gpu_profiler: (u32, u32),
    sender: Option<Sender<(&'static str, f64, f64)>>,
    thread: Option<JoinHandle<()>>,
}

impl Profiler {
    pub fn new(print: bool) -> Self {
        let cpu_profiler = (None, None);

        let gpu_profiler = unsafe {
            let mut queries = [u32::MAX, u32::MAX];
            gl!(gl::CreateQueries(gl::TIMESTAMP, 2, queries.as_mut_ptr())).unwrap();
            (queries[0], queries[1])
        };

        let (sender, thread) = if print {
            let (sender, receiver) = channel();
            let thread = std::thread::spawn(move || {
                while let Ok((task, cpu_time, gpu_time)) = receiver.recv() {
                    println!("Task {task}:");
                    println!("    CPU: {cpu_time}ms");
                    println!("    GPU: {gpu_time}ms");
                }
            });

            (Some(sender), Some(thread))
        } else {
            (None, None)
        };

        Self {
            task: None,
            cpu_profiler,
            gpu_profiler,
            sender,
            thread,
        }
    }

    pub fn begin_profile(&mut self, task: &'static str) {
        let _ = self.task.insert(task);
        let (cpu_start, _) = &mut self.cpu_profiler;
        let (gpu_start, _) = &mut self.gpu_profiler;

        if cpu_start.is_none() {
            let _ = cpu_start.insert(Instant::now());

            unsafe { gl!(gl::QueryCounter(*gpu_start, gl::TIMESTAMP)).unwrap() };
        }
    }

    pub fn end_profile(&mut self, task: &'static str) -> Option<(f64, f64)> {
        const RESULT: gl::types::GLenum = gl::QUERY_RESULT;
        const AVAILABLE: gl::types::GLenum = gl::QUERY_RESULT_AVAILABLE;

        let (cpu_start, cpu_end) = &mut self.cpu_profiler;
        let (gpu_start, gpu_end) = &mut self.gpu_profiler;

        if cpu_end.is_none() {
            let _ = cpu_end.insert(Instant::now());
            unsafe { gl!(gl::QueryCounter(*gpu_end, gl::TIMESTAMP)).unwrap() };
        }

        let mut completed = gl::FALSE as _;
        unsafe { gl!(gl::GetQueryObjectiv(*gpu_end, AVAILABLE, &mut completed)).unwrap() };
        if completed as u8 == gl::TRUE {
            let gpu_time = {
                let mut start = 0;
                unsafe { gl!(gl::GetQueryObjectui64v(*gpu_start, RESULT, &mut start)).unwrap() };

                let mut end = 0;
                unsafe { gl!(gl::GetQueryObjectui64v(*gpu_end, RESULT, &mut end)).unwrap() };

                (end - start) as f64 / 1_000_000.0
            };

            let cpu_time = {
                let start = cpu_start.expect("Measurement hasn't been started yet");
                let end = cpu_end.unwrap();
                end.duration_since(start).as_secs_f64() * 1000.0
            };

            if let Some(sender) = &self.sender {
                sender.send((task, cpu_time, gpu_time)).unwrap();
            }

            self.task = None;
            self.cpu_profiler = (None, None);

            Some((cpu_time, gpu_time))
        } else {
            None
        }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        drop(self.sender.take());

        let mut queries = [self.gpu_profiler.0, self.gpu_profiler.1];
        unsafe { gl!(gl::DeleteQueries(2, queries.as_mut_ptr())).unwrap() };

        if let Some(handle) = self.thread.take() {
            handle.join().unwrap();
        }
    }
}

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    shaders: ShaderProgram,
    cache: Cache,
    profiler: Profiler,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/shader.vert");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/shader.frag");

    #[rustfmt::skip]
    const VERTICES: [Vertex; 36] = [
        Vertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
        Vertex(Vec3::new( 0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
        Vertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
        Vertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
        Vertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
        Vertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),

        Vertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
        Vertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
        Vertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
        Vertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
        Vertex(Vec3::new(-0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
        Vertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),

        Vertex(Vec3::new(-0.5,  0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),
        Vertex(Vec3::new(-0.5,  0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
        Vertex(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
        Vertex(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
        Vertex(Vec3::new(-0.5, -0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),
        Vertex(Vec3::new(-0.5,  0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),

        Vertex(Vec3::new(0.5,  0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),
        Vertex(Vec3::new(0.5,  0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
        Vertex(Vec3::new(0.5, -0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
        Vertex(Vec3::new(0.5, -0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
        Vertex(Vec3::new(0.5, -0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),
        Vertex(Vec3::new(0.5,  0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),

        Vertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),
        Vertex(Vec3::new( 0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),
        Vertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
        Vertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
        Vertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
        Vertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),

        Vertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0)),
        Vertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0)),
        Vertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
        Vertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
        Vertex(Vec3::new(-0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
        Vertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0))
    ];

    pub fn new(window: &Window, vsync: bool, profile: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(1, vsync);

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);
        let shaders = device.new_shader_program(&vertex_shader, &pixel_shader);

        let vertices = device.new_buffer(BufferInit::Data(&Self::VERTICES));
        let matrices = device.new_buffer(BufferInit::Capacity(2));

        Self {
            _instance,
            device,
            swapchain,
            shaders,
            cache: Cache {
                vertices,
                matrices,
                materials: None,
                buffers: Vec::default(),
            },
            profiler: Profiler::new(profile),
        }
    }

    /// Renders a single frame
    pub fn run(&mut self, scene: &mut Scene) -> Option<f64> {
        let Self { device, cache, .. } = self;

        self.profiler.begin_profile("Renderer");

        unsafe { gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT)) }.unwrap();

        if cache.materials.is_none() {
            cache.materials = Some(device.new_buffer(BufferInit::Data(scene.materials())));
        }

        let view_projection = scene.camera.view_projection();

        // TODO: Batch multiple chunks into a single drawcall.
        for chunk in scene.terrain() {
            self.render_chunk(view_projection, chunk);
        }

        self.swapchain.present();

        let measurement = self.profiler.end_profile("Renderer");
        measurement.map(|(cpu, gpu)| cpu + gpu)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        unsafe { gl!(gl::Viewport(0, 0, width as _, height as _)) }.unwrap();
    }

    fn render_chunk(&mut self, view_projection: Mat4, chunk: &Chunk) {
        let Self { device, cache, .. } = self;

        let materials = cache
            .materials
            .as_ref()
            .expect("Materials haven't been uploaded to the GPU");

        let mvp = view_projection * chunk.transform;
        cache.matrices.map_write().write(&[chunk.transform, mvp]);

        let offsets: Vec<_> = chunk.positions.iter().map(|(offset, _)| *offset).collect();
        let offsets: Buffer<_> = device.new_buffer(BufferInit::Data(&offsets));

        let material_ids: Vec<_> = chunk.positions.iter().map(|(_, id)| id.0 as u32).collect();
        let material_ids: Buffer<_, false> = device.new_buffer(BufferInit::Data(&material_ids));

        assert_eq!(offsets.len(), material_ids.len());

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &[0, 1],
            buffer: &cache.vertices,
            instanced: false,
        });

        device.bind_vertex_buffer(BindProps {
            binding: 1,
            attributes: &[2],
            buffer: &offsets,
            instanced: true,
        });

        device.bind_vertex_buffer(BindProps {
            binding: 2,
            attributes: &[3],
            buffer: &material_ids,
            instanced: true,
        });

        device.bind_shader_program(&self.shaders);

        unsafe {
            gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, cache.matrices.id)).unwrap();
            gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 1, materials.id)).unwrap();
        }

        device.draw_instanced(cache.vertices.len(), offsets.len());
    }
}

unsafe impl BufferLayout for Material {
    const LAYOUT: &'static [Format] = &[Format::Vec4, Format::F32, Format::F32];
    const PADDING: &'static [usize] = &[0, 0, 8];
    const COPYABLE: bool = false;

    // OpenGL require that arrays are aligned to a multiple of 16.
    // Since the material contains a total of 24 bytes, the next multiple is 32.
    // Because of that we must add 8 empty bytes at the end of our material.
    fn to_bytes(items: &[Self]) -> Vec<u8> {
        // assert_eq!(std::mem::size_of::<Self>(), 24);

        let mut bytes: Vec<u8> = Vec::with_capacity(items.len() * std::mem::size_of::<Self>());
        for item in items {
            let albedo = {
                let [r, g, b, a] = item.albedo;
                Vec4::new(r as _, g as _, b as _, a as _) / 255.0
            };

            for component in albedo.as_ref() {
                bytes.extend_from_slice(&component.to_ne_bytes()) // 4 bytes * 4
            }

            bytes.extend_from_slice(&item.roughness.to_ne_bytes()); // 4 bytes
            bytes.extend_from_slice(&item.metalness.to_ne_bytes()); // 4 bytes
            bytes.extend_from_slice(&[0; 8]); // 8 bytes
        }

        bytes
    }
}

unsafe impl BufferLayout for Vertex {
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::Vec3];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

#[repr(C)]
struct Vertex(Vec3, Vec3);

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Self(position, normal)
    }
}

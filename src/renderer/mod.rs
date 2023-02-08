use glam::*;
use sdl2::video::*;

use self::profiler::*;
use crate::rhi::*;
use crate::scene::*;

mod profiler;

#[repr(C)]
struct Vertex(Vec3, Vec3);

unsafe impl BufferLayout for Vertex {
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::Vec3];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

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

struct Cache {
    vertex_buffer: Buffer<Vertex, false, false>,
    matrix_buffer: Buffer<Mat4, false, true>,
    material_buffer: Option<Buffer<Material, false, false>>,
}

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    framebuffer: Option<Framebuffer>,
    shaders: ShaderProgram,

    window_size: UVec2,

    cache: Cache,
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

        let cache = Cache {
            vertex_buffer: device.new_buffer(BufferInit::Data(&VERTICES)),
            matrix_buffer: device.new_buffer(BufferInit::Capacity(2)),
            material_buffer: None,
        };

        let profiler = Profiler::new(false);

        Self {
            _instance,
            device,
            swapchain,
            framebuffer,
            shaders,
            window_size,
            cache,
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
        default_framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0), true);

        self.geometry_pass(scene, &mut framebuffer);
        // self.postprocess_pass(&mut framebuffer);
        // self.text_pass(&mut framebuffer);

        // Copy the image from the framebuffer to the default framebuffer and present
        // the image.
        let Self { device, .. } = self;
        // device.blit(&framebuffer, &mut default_framebuffer, false);

        self.swapchain.present();

        self.framebuffer = Some(framebuffer);

        Some(1.0)
    }

    pub fn resize(&mut self, window_size: UVec2) {
        let Self { device, .. } = self;
        self.window_size = window_size;

        unsafe { gl::Viewport(0, 0, window_size.x as _, window_size.y as _) };

        self.framebuffer = {
            let window_width = window_size.x as _;
            let window_height = window_size.y as _;
            let texture = device.new_texture_2d(window_width, window_height, Format::R8G8B8A8);
            let depth = device.new_texture_2d(window_width, window_height, Format::D24);
            Some(device.new_framebuffer(texture, Some(depth)))
        };
    }

    fn geometry_pass(&mut self, scene: &mut Scene, framebuffer: &mut Framebuffer) {
        let Self { device, cache, .. } = self;
        let Cache {
            vertex_buffer,
            matrix_buffer,
            material_buffer,
        } = cache;

        scene.entities.evaluate_all();

        let material_buffer = if let Some(material_buffer) = material_buffer.take() {
            material_buffer
        } else {
            device.new_buffer(BufferInit::Data(scene.materials()))
        };

        for entity in scene.entities.mutated_entities() {
            if let Entity::Object(object) = entity {
                let model = &object.model;

                let entity: Buffer<_, false, false> =
                    device.new_buffer(BufferInit::Data(&model.positions));

                device.bind_vertex_buffer(BindProps {
                    binding: 0,
                    attributes: &[0, 1],
                    buffer: vertex_buffer,
                    instanced: false,
                });

                device.bind_vertex_buffer(BindProps {
                    binding: 1,
                    attributes: &[2, 3],
                    buffer: &entity,
                    instanced: true,
                });

                device.bind_shader_program(&self.shaders);

                let model_matrix = model.transform;
                let mvp_matrix = scene.camera.view_projection() * (object.transform * model_matrix);

                matrix_buffer.map_write().write(&[model_matrix, mvp_matrix]);

                unsafe {
                    gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, matrix_buffer.id)).unwrap();
                    gl!(gl::BindBufferBase(
                        gl::UNIFORM_BUFFER,
                        1,
                        material_buffer.id
                    ))
                    .unwrap();
                }

                device.unbind_framebuffer();
                device.draw_instanced(VERTICES.len(), model.positions.len());
            }
        }

        cache.material_buffer = Some(material_buffer)
    }

    fn postprocess_pass(&mut self, framebuffer: &mut Framebuffer) {}

    fn text_pass(&mut self, framebuffer: &mut Framebuffer) {}
}

unsafe impl BufferLayout for (Vec3, MaterialId) {
    const COPYABLE: bool = false;
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::U32];
    const PADDING: &'static [usize] = &[0, 0];

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(items.len() * (12 + 4));
        for (offset, material) in items {
            let offset: [u8; 12] = unsafe { std::mem::transmute(*offset) };
            bytes.extend(offset);
            bytes.extend((material.0 as u32).to_ne_bytes())
        }

        bytes
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

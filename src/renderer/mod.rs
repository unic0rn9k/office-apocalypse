use glam::*;
use sdl2::video::*;

use self::profiler::*;
use crate::rhi::*;
use crate::scene::*;

mod deferred_renderer;
mod profiler;
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
struct CubeVertex(Vec3, Vec3);

unsafe impl BufferLayout for CubeVertex {
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::Vec3];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

#[rustfmt::skip]
const CUBE: [CubeVertex; 36] = [
    CubeVertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
    CubeVertex(Vec3::new( 0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
    CubeVertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
    CubeVertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
    CubeVertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),
    CubeVertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0,  0.0, -1.0)),

    CubeVertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
    CubeVertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
    CubeVertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
    CubeVertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
    CubeVertex(Vec3::new(-0.5,  0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),
    CubeVertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0,  0.0,  1.0)),

    CubeVertex(Vec3::new(-0.5,  0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(-0.5,  0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(-1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(-0.5, -0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(-0.5,  0.5,  0.5), Vec3::new(-1.0,  0.0,  0.0)),

    CubeVertex(Vec3::new(0.5,  0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(0.5,  0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(0.5, -0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(0.5, -0.5, -0.5),  Vec3::new(1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(0.5, -0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),
    CubeVertex(Vec3::new(0.5,  0.5,  0.5),  Vec3::new(1.0,  0.0,  0.0)),

    CubeVertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
    CubeVertex(Vec3::new(-0.5, -0.5,  0.5),  Vec3::new(0.0, -1.0,  0.0)),
    CubeVertex(Vec3::new(-0.5, -0.5, -0.5),  Vec3::new(0.0, -1.0,  0.0)),

    CubeVertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
    CubeVertex(Vec3::new( 0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
    CubeVertex(Vec3::new(-0.5,  0.5,  0.5),  Vec3::new(0.0,  1.0,  0.0)),
    CubeVertex(Vec3::new(-0.5,  0.5, -0.5),  Vec3::new(0.0,  1.0,  0.0))
];

#[derive(Debug)]
struct Voxel {
    pub offset: Vec3,
    pub chunk_id: u32,
    pub material_id: u32,
}

unsafe impl BufferLayout for Voxel {
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::U32, Format::U32];
    const PADDING: &'static [usize] = &[0, 0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

struct GBuffer {
    position: Texture2D,
    normal: Texture2D,
    albedo: Texture2D,
    roughness_and_metalness: Texture2D,
}

struct Cache {
    vertex_buffer: Buffer<CubeVertex, false, false>,
    quad_buffer: Buffer<QuadVertex, false, false>,
    matrix_buffer: Buffer<Matrices, false, true>,
    // light_buffer: Option<Buffer<Light, false, true>>,
    material_buffer: Option<Buffer<Material, false, false>>,
}

pub struct Renderer<'a> {
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    framebuffer: Option<Framebuffer>,
    geometry_shaders: ShaderProgram,
    lighting_shaders: ShaderProgram,

    window_size: UVec2,

    cache: Cache,
    profiler: Profiler,
}

impl Renderer<'_> {
    const GEOMETRY_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds.vert");
    const GEOMETRY_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds.frag");

    const LIGHTING_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.vert");
    const LIGHTING_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.frag");

    const MAX_LIGHTS: usize = 256;

    // OpenGL is required to support at least 16384 bytes for uniform buffers.
    // 170 * (2 * std::mem::size_of::<Mat4>()) < 16384
    const MAX_CHUNKS: usize = 170;

    pub fn new(window: &Window, vsync: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(vsync);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let window_size = UVec2::from(window.size());

        let framebuffer = {
            let window_width = window_size.x as _;
            let window_height = window_size.y as _;
            let texture = device.new_texture_2d(window_width, window_height, Format::R8G8B8A8);
            let depth = device.new_texture_2d(window_width, window_height, Format::D24);
            Some(device.new_framebuffer(texture, Some(depth)))
        };

        let geometry_shaders = {
            let vertex_shader = device.new_shader(VertexStage, Self::GEOMETRY_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::GEOMETRY_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let lighting_shaders = {
            let vertex_shader = device.new_shader(VertexStage, Self::LIGHTING_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::LIGHTING_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let cache = Cache {
            vertex_buffer: device.new_buffer(BufferInit::Data(&CUBE)),
            quad_buffer: device.new_buffer(BufferInit::Data(&QUAD)),
            matrix_buffer: device.new_buffer(BufferInit::Capacity(Self::MAX_CHUNKS)),
            material_buffer: None,
        };

        let profiler = Profiler::new(false);

        Self {
            _instance,
            device,
            swapchain,
            framebuffer,
            geometry_shaders,
            lighting_shaders,
            window_size,
            cache,
            profiler,
        }
    }

    pub fn render(&mut self, scene: &mut Scene) -> Option<f64> {
        // Compute the transformations applied by the scene graph.
        scene.scene_graph.evaluate_all();

        let mut default_framebuffer = self.device.default_framebuffer();
        default_framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0), true);

        self.g_pass(scene, &mut default_framebuffer);

        self.swapchain.present();

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

    /// Renders the geometry without lighting
    ///
    /// This pass renders all the necessary information to `framebuffer` for
    /// doing the lighting calculations. The lighting is essentially
    /// postponed to another pass, so we don't waste compute-power on
    /// occluded voxels.
    fn g_pass(&mut self, scene: &mut Scene, framebuffer: &mut Framebuffer) {
        let Self { device, cache, .. } = self;
        let Cache {
            vertex_buffer,
            quad_buffer,
            matrix_buffer,
            material_buffer,
        } = cache;

        let (matrices, voxels) = Self::extract_matrices_and_voxels(scene);
        matrix_buffer.map_write().write(&matrices);

        // println!("{voxels:?}");

        let voxel_buffer: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&voxels));

        let (
            fbo,
            position_texture,
            normal_texture,
            albedo_texture,
            roughness_and_metalness_texture,
        ) = unsafe {
            let position_texture = device.new_texture_2d(
                self.window_size.x as _,
                self.window_size.y as _,
                Format::R32G32B32A32Float,
            );

            let normal_texture = device.new_texture_2d(
                self.window_size.x as _,
                self.window_size.y as _,
                Format::R32G32B32A32Float,
            );

            let albedo_texture = device.new_texture_2d(
                self.window_size.x as _,
                self.window_size.y as _,
                Format::R32G32B32A32Float,
            );

            let roughness_and_metalness_texture = device.new_texture_2d(
                self.window_size.x as _,
                self.window_size.y as _,
                Format::R32G32Float,
            );

            let depth_buffer = device.new_texture_2d(
                self.window_size.x as _,
                self.window_size.y as _,
                Format::D24,
            );

            let mut fbo = u32::MAX;
            gl!(gl::CreateFramebuffers(1, &mut fbo)).unwrap();
            gl!(gl::NamedFramebufferTexture(
                fbo,
                gl::COLOR_ATTACHMENT0,
                position_texture.id,
                0
            ))
            .unwrap();

            gl!(gl::NamedFramebufferTexture(
                fbo,
                gl::COLOR_ATTACHMENT1,
                normal_texture.id,
                0
            ))
            .unwrap();

            gl!(gl::NamedFramebufferTexture(
                fbo,
                gl::COLOR_ATTACHMENT2,
                albedo_texture.id,
                0
            ))
            .unwrap();

            gl!(gl::NamedFramebufferTexture(
                fbo,
                gl::COLOR_ATTACHMENT3,
                roughness_and_metalness_texture.id,
                0
            ))
            .unwrap();

            gl!(gl::NamedFramebufferTexture(
                fbo,
                gl::DEPTH_ATTACHMENT,
                depth_buffer.id,
                0
            ))
            .unwrap();

            gl!(gl::NamedFramebufferDrawBuffers(
                fbo,
                4,
                [
                    gl::COLOR_ATTACHMENT0,
                    gl::COLOR_ATTACHMENT1,
                    gl::COLOR_ATTACHMENT2,
                    gl::COLOR_ATTACHMENT3
                ]
                .as_ptr(),
            ))
            .unwrap();

            gl!(gl::ClearNamedFramebufferfv(
                fbo,
                gl::COLOR,
                0,
                [0.0, 0.0, 0.0, 0.0].as_ptr()
            ))
            .unwrap();

            gl!(gl::ClearNamedFramebufferfv(
                fbo,
                gl::COLOR,
                1,
                [0.0, 0.0, 0.0, 0.0].as_ptr()
            ))
            .unwrap();

            gl!(gl::ClearNamedFramebufferfv(
                fbo,
                gl::COLOR,
                2,
                [0.0, 0.0, 0.0, 0.0].as_ptr()
            ))
            .unwrap();

            gl!(gl::ClearNamedFramebufferfv(
                fbo,
                gl::COLOR,
                3,
                [0.0, 0.0, 0.0, 0.0].as_ptr()
            ))
            .unwrap();

            gl!(gl::ClearNamedFramebufferfv(
                fbo,
                gl::DEPTH,
                0,
                [1.0].as_ptr()
            ))
            .unwrap();

            (
                fbo,
                position_texture,
                normal_texture,
                albedo_texture,
                roughness_and_metalness_texture,
            )
        };

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &[0, 1],
            buffer: vertex_buffer,
            instanced: false,
        });

        device.bind_vertex_buffer(BindProps {
            binding: 1,
            attributes: &[2, 3, 4],
            buffer: &voxel_buffer,
            instanced: true,
        });

        device.bind_shader_program(&self.geometry_shaders);

        unsafe {
            gl!(gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, matrix_buffer.id)).unwrap();
            gl!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo)).unwrap();
            // gl!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
            // gl!(gl::Clear(gl::COLOR_BUFFER_BIT |
            // gl::DEPTH_BUFFER_BIT)).unwrap();
        }

        let material_buffer = if let Some(material_buffer) = material_buffer.take() {
            material_buffer
        } else {
            device.new_buffer(BufferInit::Data(scene.materials()))
        };

        unsafe {
            gl!(gl::BindBufferBase(
                gl::UNIFORM_BUFFER,
                1,
                material_buffer.id
            ))
            .unwrap();
        }

        device.draw_instanced(vertex_buffer.len(), voxel_buffer.len());

        cache.material_buffer = Some(material_buffer);

        // lighting

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &[0, 1],
            buffer: quad_buffer,
            instanced: false,
        });

        device.bind_shader_program(&self.lighting_shaders);

        device.bind_texture_2d(&position_texture, "gPosition", 0);
        device.bind_texture_2d(&normal_texture, "gNormal", 1);
        device.bind_texture_2d(&albedo_texture, "gAlbedo", 2);
        device.bind_texture_2d(
            &roughness_and_metalness_texture,
            "gRoughnessAndMetalness",
            3,
        );

        device.unbind_framebuffer();

        device.draw(quad_buffer.len());

        unsafe { gl::DeleteFramebuffers(1, [fbo].as_ptr()) };
    }

    fn extract_matrices_and_voxels(scene: &mut Scene) -> (Vec<Matrices>, Vec<Voxel>) {
        let entities = scene.scene_graph.mutated_entities();
        let camera = scene.camera();

        let objects = |entity: &Entity| {
            if let Entity::Object(o) = entity.clone() {
                Some(o)
            } else {
                None
            }
        };

        let mut matrices = Vec::with_capacity(Self::MAX_CHUNKS);
        let mut voxels = Vec::with_capacity(256 * 256 * 256); // 16 Mib
        for (i, object) in entities.filter_map(objects).enumerate() {
            let model = object.transform * object.model.transform;
            matrices.push(Matrices(model, camera.view_projection() * model));
            voxels.extend(
                object
                    .model
                    .positions
                    .iter()
                    .map(|&(offset, material_id)| Voxel {
                        offset,
                        chunk_id: i as _,
                        material_id: material_id.0 as _,
                    }),
            );
        }

        assert!(matrices.len() <= Self::MAX_CHUNKS);
        (matrices, voxels)
    }

    fn lighting_pass(&mut self, framebuffer: &mut Framebuffer, gbuffer: GBuffer) {}

    fn postprocess_pass(&mut self, framebuffer: &mut Framebuffer) {}

    fn text_pass(&mut self, scene: &Scene, framebuffer: &mut Framebuffer) {}
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

#[repr(C)]
struct Matrices(Mat4, Mat4);

unsafe impl BufferLayout for Matrices {
    const LAYOUT: &'static [Format] = &[Format::Mat4, Format::Mat4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

// unsafe impl BufferLayout for Light {
// }

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

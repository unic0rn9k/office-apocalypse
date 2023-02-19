use glam::*;

use super::{CubeVertex, QuadVertex, CUBE, QUAD};
use crate::rhi::*;
use crate::scene::*;

struct Voxel {
    position: Vec3,
    chunk_id: u16, // Since we only allow 256 chunks in a drawcall a u16 saves us some bandwidth.
    material_id: u16, // Since we only allow 256 materials a u16 saves us some bandwidth.
}

unsafe impl BufferLayout for Voxel {
    const LAYOUT: &'static [Format] = &[Format::Vec3, Format::U16, Format::U16];
    const PADDING: &'static [usize] = &[0, 0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

unsafe impl BufferLayout for [Mat4; 2] {
    const LAYOUT: &'static [Format] = &[Format::Mat4, Format::Mat4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(_items: &[Self]) -> Vec<u8> {
        unimplemented!()
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

unsafe impl BufferLayout for Light {
    const LAYOUT: &'static [Format] = &[Format::Vec4, Format::Vec4];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = false;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(items.len() * Self::stride());

        for item in items {
            let position: [u8; 12] = unsafe { std::mem::transmute(item.position) };
            bytes.extend(position);
            bytes.extend(1.0f32.to_ne_bytes());

            let color: [u8; 12] = unsafe { std::mem::transmute(item.color) };
            bytes.extend(color);
            bytes.extend(1.0f32.to_ne_bytes());
        }

        bytes
    }
}

pub struct DeferredRenderer<'a> {
    device: Device<'a>,
    cube_buffer: Buffer<CubeVertex, false, false>,
    quad_buffer: Buffer<QuadVertex, false, false>,
    matrix_buffer: Buffer<[Mat4; 2], false, true>,
    material_buffer: Buffer<Material, false, true>,
    light_buffer: Buffer<Light, false, true>,
    camera_buffer: Buffer<Vec4, false, true>,
    program: ShaderProgram,
    lighting_program: ShaderProgram,
    framebuffer: Framebuffer,
}

impl<'a> DeferredRenderer<'a> {
    const DS_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds.vert");
    const DS_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds.frag");
    const DS_LIGHTING_VERTEX_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.vert");
    const DS_LIGHTING_PIXEL_SHADER_SRC: &str = include_str!("./shaders/ds_lighting.frag");

    // The maximum amount of chunks that can be grouped into a single drawcall.
    //
    // OpenGL is required to support at least 16384 bytes for uniform buffers.
    // MAX_CHUNKS * (2 * std::mem::size_of::<Mat4>()) < 16384
    const MAX_CHUNKS: usize = 170;

    // The maximum amount of materials that can be used at any given time.
    const MAX_MATERIALS: usize = 256;

    // The maximum amount of lights that can be used at any given time.
    const MAX_LIGHTS: usize = 256;

    pub fn new(device: Device<'a>, window_size: UVec2) -> Self {
        // The cube buffer is static since we use instanced rendering, so it is uploaded
        // once at the creation of the renderer.
        let cube_buffer = device.new_buffer(BufferInit::Data(&CUBE));
        let quad_buffer = device.new_buffer(BufferInit::Data(&QUAD));

        // We preallocate space for the various kinds of uniform buffers.
        let matrix_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_CHUNKS));
        let material_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_MATERIALS));
        let light_buffer = device.new_buffer(BufferInit::Capacity(Self::MAX_LIGHTS));
        let camera_buffer = device.new_buffer(BufferInit::Capacity(1));

        let program = {
            let vertex_shader = device.new_shader(VertexStage, Self::DS_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::DS_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let lighting_program = {
            let vertex_shader = device.new_shader(VertexStage, Self::DS_LIGHTING_VERTEX_SHADER_SRC);
            let pixel_shader = device.new_shader(PixelStage, Self::DS_LIGHTING_PIXEL_SHADER_SRC);
            device.new_shader_program(&vertex_shader, &pixel_shader)
        };

        let framebuffer = Self::setup_framebuffer(&device, window_size);

        Self {
            device,
            cube_buffer,
            quad_buffer,
            matrix_buffer,
            material_buffer,
            light_buffer,
            camera_buffer,
            program,
            lighting_program,
            framebuffer,
        }
    }

    pub fn render(&mut self, scene: &mut Scene) {
        let Self {
            device,
            cube_buffer,
            quad_buffer,
            matrix_buffer,
            material_buffer,
            light_buffer,
            camera_buffer,
            program,
            lighting_program,
            framebuffer,
        } = self;

        framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0), true);

        // Write matrices and upload voxels
        let (matrices, voxels) = Self::extract_matrices_and_voxels(scene);
        matrix_buffer.map_write().write(&matrices);
        let voxel_buffer: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&voxels));

        // Write materials
        material_buffer.map_write().write(scene.materials());

        device.bind_shader_program(program);

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &["a_position", "a_normal"],
            buffer: cube_buffer,
            instanced: false,
        });

        device.bind_vertex_buffer(BindProps {
            binding: 1,
            attributes: &["a_offset", "a_chunkId", "a_materialId"],
            buffer: &voxel_buffer,
            instanced: true,
        });

        device.bind_framebuffer(framebuffer);

        device.bind_uniform_buffer(matrix_buffer, 0);
        device.bind_uniform_buffer(material_buffer, 1);

        device.draw_instanced(cube_buffer.len(), voxel_buffer.len());

        // Write lights
        let lights = Self::extract_lights(scene);
        light_buffer.map_write().write(&lights);

        let position = scene.camera().translation();
        let position = vec4(position.x, position.y, position.z, 1.0);
        camera_buffer.map_write().write(&[position]);

        device.bind_shader_program(&lighting_program);

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &["a_position", "a_texcoord"],
            buffer: quad_buffer,
            instanced: false,
        });

        device.bind_framebuffer(&mut device.default_framebuffer());

        device.bind_uniform_buffer(light_buffer, 0);
        device.bind_uniform_buffer(camera_buffer, 1);
        device.bind_texture_2d(framebuffer.color(0), "gWorldPosition", 0);
        device.bind_texture_2d(framebuffer.color(1), "gNormal", 1);
        device.bind_texture_2d(framebuffer.color(2), "gAlbedo", 2);
        device.bind_texture_2d(framebuffer.color(3), "gRoughnessAndMetalness", 3);

        device.draw(quad_buffer.len());
    }

    pub fn resize(&mut self, window_size: UVec2) {
        self.framebuffer = Self::setup_framebuffer(&self.device, window_size);
    }

    fn extract_matrices_and_voxels(scene: &mut Scene) -> (Vec<[Mat4; 2]>, Vec<Voxel>) {
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
            matrices.push([model, camera.view_projection() * model]);
            voxels.extend(
                object
                    .model
                    .positions
                    .iter()
                    .map(|&(position, material_id)| Voxel {
                        position,
                        chunk_id: i as _,
                        material_id: material_id.0 as _,
                    }),
            );
        }

        // We handle the terrain geometry here
        let offset = matrices.len();
        for (i, chunk) in scene.terrain.iter().enumerate() {
            matrices.push([chunk.transform, camera.view_projection() * chunk.transform]);
            voxels.extend(chunk.into_iter().map(|(position, material_id)| Voxel {
                position: position.as_vec3(),
                chunk_id: (i + offset) as _,
                material_id: material_id.0 as _,
            }));
        }

        assert!(matrices.len() <= Self::MAX_CHUNKS);
        (matrices, voxels)
    }

    fn extract_lights(scene: &mut Scene) -> Vec<Light> {
        let entities = scene.scene_graph.mutated_entities();

        let lights = |entity: &Entity| {
            if let Entity::Light(l) = entity.clone() {
                Some(l)
            } else {
                None
            }
        };

        entities.filter_map(lights).collect()
    }

    fn setup_framebuffer(device: &Device<'a>, window_size: UVec2) -> Framebuffer {
        let [width, height] = window_size.to_array().map(|v| v as _);
        let positions = device.new_texture_2d(width, height, Format::R32G32B32A32Float);
        let normals = device.new_texture_2d(width, height, Format::R32G32B32A32Float);
        let albedo = device.new_texture_2d(width, height, Format::R32G32B32A32Float);
        let roughness_and_metalness = device.new_texture_2d(width, height, Format::R32G32Float);

        let depth = device.new_texture_2d(width, height, Format::D24);

        let attachments = [
            Attachment::Color(positions, 0),
            Attachment::Color(normals, 1),
            Attachment::Color(albedo, 2),
            Attachment::Color(roughness_and_metalness, 3),
            Attachment::Depth(depth),
        ];

        device.new_framebuffer(attachments)
    }
}

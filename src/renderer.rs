use glam::*;
use sdl2::video::*;

use crate::rhi::*;
use crate::scene::*;

pub struct Renderer<'a> {
    instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    shaders: ShaderProgram,

    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    texture: image::Rgb32FImage,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/vertex_shader.glsl");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/pixel_shader.glsl");

    // #[rustfmt::skip]
    // const VERTICES: [Vertex; 8] = [
    //     Vertex::new(Vec3::new(-1.0, -1.0,  1.0)),
    //     Vertex::new(Vec3::new( 1.0, -1.0,  1.0)),
    //     Vertex::new(Vec3::new( 1.0,  1.0,  1.0)),
    //     Vertex::new(Vec3::new(-1.0,  1.0,  1.0)),
    //     Vertex::new(Vec3::new(-1.0, -1.0, -1.0)),
    //     Vertex::new(Vec3::new( 1.0, -1.0, -1.0)),
    //     Vertex::new(Vec3::new( 1.0,  1.0, -1.0)),
    //     Vertex::new(Vec3::new(-1.0,  1.0, -1.0)),
    // ];

    // #[rustfmt::skip]
    // const INDICES: [u32; 36] = [
    //     0, 1, 2,
    // 	2, 3, 0,
    // 	1, 5, 6,
    // 	6, 2, 1,
    // 	7, 6, 5,
    // 	5, 4, 7,
    // 	4, 0, 3,
    // 	3, 7, 4,
    // 	4, 5, 1,
    // 	1, 0, 4,
    // 	3, 2, 6,
    // 	6, 7, 3
    // ];

    pub fn new(window: &Window) -> Self {
        let instance = Instance::new(window, true);
        let device = instance.new_device();
        let swapchain = instance.new_swapchain(1);

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);
        let shaders = device.new_shader_program(&vertex_shader, &pixel_shader);

        let (models, mats) = tobj::load_obj("./assets/plant.obj", &tobj::GPU_LOAD_OPTIONS).unwrap();
        let mats = mats.unwrap();

        let model = &models[0];

        let positions: Vec<_> = model
            .mesh
            .positions
            .chunks_exact(3)
            .map(|position| Vec3::new(position[0], position[1], position[2]))
            .collect();

        let texcoords: Vec<_> = model
            .mesh
            .texcoords
            .chunks_exact(2)
            .map(|texcoord| Vec2::new(texcoord[0], texcoord[0]))
            .collect();

        let mut vertices = Vec::with_capacity(positions.len() + texcoords.len());
        for i in 0..positions.len() {
            let position = positions[i];
            let texcoord = texcoords[i];
            vertices.push(Vertex::new(position, texcoord));
        }

        let indices: Vec<_> = model.mesh.indices.clone();

        let id = model.mesh.material_id.unwrap();

        let texture_path = format!("./assets/{}", &mats[id].diffuse_texture);
        let texture = image::open(texture_path).unwrap().to_rgb32f();

        Self {
            instance,
            device,
            swapchain,
            shaders,

            vertices,
            indices,
            texture,
        }
    }

    pub fn run(&mut self, scene: &Scene) {
        let Self { device, .. } = self;

        unsafe { gl::ClearColor(0.0, 0.0, 0.0, 1.0) };
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT) };

        let vb: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&self.vertices));
        let ib: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&self.indices));

        let vp = scene.camera.projection().clone() * scene.camera.view().clone();
        let ub: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&[vp]));

        let mut texture = u32::MAX;
        unsafe { gl::CreateTextures(gl::TEXTURE_2D, 1, &mut texture) };
        unsafe { gl::TextureStorage2D(texture, 1, gl::RGB32F, self.texture.width() as _, 1) }

        unsafe {
            gl::TextureSubImage2D(
                texture,
                0,
                0,
                0,
                self.texture.width() as _,
                1,
                gl::RGB,
                gl::FLOAT,
                self.texture.as_ptr() as _,
            )
        }

        device.set_vertex_buffers(&[&vb]);
        device.set_index_buffer(&ib);

        device.set_shader_program(&self.shaders);

        let block = b"Matrix\0".as_ptr() as _;
        let block_index = unsafe { gl::GetUniformBlockIndex(self.shaders.id, block) };

        unsafe { gl::BindBufferBase(gl::UNIFORM_BUFFER, block_index, ub.id) };
        unsafe { gl::BindTextureUnit(0, texture) };

        device.draw_indexed(self.indices.len());

        self.swapchain.present();
    }
}

unsafe impl BufferLayout for Vec2 {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(2, gl::FLOAT)];
}

unsafe impl BufferLayout for Vec3 {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(3, gl::FLOAT)];
}

unsafe impl BufferLayout for Vec4 {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(4, gl::FLOAT)];
}

unsafe impl BufferLayout for Mat4 {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(16, gl::FLOAT)];
}

#[repr(C)]
struct Vertex {
    position: Vec3,
    texcoord: Vec2,
}

impl Vertex {
    pub const fn new(position: Vec3, texcoord: Vec2 /* color: Vec3 */) -> Self {
        Self { position, texcoord }
    }
}

unsafe impl BufferLayout for Vertex {
    const LAYOUT: &'static [(usize, gl::types::GLenum)] = &[(0, gl::FLOAT), (1, gl::FLOAT)];
}

// unsafe impl Layout for Vertex {}

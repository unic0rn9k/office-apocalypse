use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Instant;

use glam::*;
use image::EncodableLayout;
use sdl2::video::*;

use crate::rhi::*;
use crate::scene::*;

struct TextRenderer<'a> {
    device: Device<'a>,
    shaders: ShaderProgram,
    font_face: FontFace,
    atlas: Texture2D,
    matrix_buffer: Buffer<Mat4, false, true>,
}

impl<'a> TextRenderer<'a> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/text.vert");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/text.frag");

    const FONT_FACE: &'static [u8] = include_bytes!("../assets/fonts/sans-serif/sans-serif.fnt");
    const FONT_IMAGE: &'static [u8] = include_bytes!("../assets/fonts/sans-serif/sans-serif.png");

    pub fn new(device: Device<'a>, window_size: (u32, u32)) -> Self {
        let shaders = {
            let vs = device.new_shader(VertexStage, Self::VERTEX_SHADER);
            let ps = device.new_shader(PixelStage, Self::PIXEL_SHADER);
            device.new_shader_program(&vs, &ps)
        };

        let font_face = Self::parse_fnt(Self::FONT_FACE);
        let mut atlas = device.new_texture_2d(font_face.width, font_face.height, Format::R8G8B8A8);
        let font_image = image::load_from_memory(Self::FONT_IMAGE).unwrap();
        atlas.write(font_image.flipv().as_rgba8().as_ref().unwrap());

        let (width, height) = window_size;
        let projection = Mat4::orthographic_rh_gl(0.0, width as _, 0.0, height as _, 0.0, 1.0);
        let matrix_buffer = device.new_buffer(BufferInit::Data(&[projection]));

        Self {
            device,
            shaders,
            font_face,
            atlas,
            matrix_buffer,
        }
    }

    pub fn render(&mut self, scene: &Scene, framebuffer: &mut Framebuffer) {
        let Self { device, .. } = self;

        let Text {
            position,
            text,
            color,
            scale,
        } = &scene.text[0];

        let position = vec2(position.x as _, position.y as _);

        let mut vertices = Vec::with_capacity(6 * text.chars().count());
        let mut advance = Vec2::default();
        for c in text.chars() {
            if c.is_whitespace() {
                advance += vec2(38.0, 0.0);
                continue;
            }

            let glyph = self
                .font_face
                .glyphs
                .iter()
                .find(|glyph| glyph.id == c)
                .unwrap();

            let glyph_size = vec2(glyph.size.x as _, glyph.size.y as _);
            let glyph_position = vec2(glyph.position.x as _, glyph.position.y as _);
            let glyph_offset = vec2(glyph.offset.x as _, glyph.offset.y as _);
            let glyph_height = vec2(0.0, glyph_size.y);
            let glyph_width = vec2(glyph_size.x, 0.0);

            // (font_face_width, 0) -> (1, 0)
            // (0, font_face_height) -> (0, 0)
            let to_opengl = |texcoord: Vec2| {
                let x = texcoord.x / self.font_face.width as f32;
                let y = 1.0 - (texcoord.y / self.font_face.height as f32);
                vec2(x, y)
            };

            vertices.extend_from_slice(&[
                // top left -> top right -> bottom left
                TextVertex {
                    position: position - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position),
                },
                TextVertex {
                    position: position + glyph_width - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position + glyph_width),
                },
                TextVertex {
                    position: position - glyph_height - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position + glyph_height),
                },
                // top right -> bottom right -> bottom left
                TextVertex {
                    position: position + glyph_width - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position + glyph_width),
                },
                TextVertex {
                    position: position + glyph_width - glyph_height - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position + glyph_size),
                },
                TextVertex {
                    position: position - glyph_height - glyph_offset + advance,
                    texcoord: to_opengl(glyph_position + glyph_height),
                },
            ]);

            advance += glyph_width
        }

        let vertex_buffer: Buffer<_, false, false> = device.new_buffer(BufferInit::Data(&vertices));

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        device.bind_vertex_buffer(BindProps {
            binding: 0,
            attributes: &[0, 1],
            buffer: &vertex_buffer,
            instanced: false,
        });

        device.bind_shader_program(&self.shaders);

        unsafe {
            gl::TextureParameteri(self.atlas.id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl::TextureParameteri(self.atlas.id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl!(gl::BindTexture(gl::TEXTURE_2D, self.atlas.id)).unwrap();

            gl!(gl::BindBufferBase(
                gl::UNIFORM_BUFFER,
                0,
                self.matrix_buffer.id
            ))
            .unwrap();
        }

        device.bind_framebuffer(framebuffer);
        device.draw(vertices.len());
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let projection = Mat4::orthographic_rh_gl(0.0, width as _, 0.0, height as _, 0.0, 1.0);
        self.matrix_buffer = self.device.new_buffer(BufferInit::Data(&[projection]));
    }

    fn parse_fnt(bytes: &[u8]) -> FontFace {
        let ident = |s: &str| {
            s.chars()
                .take_while(|c| c.is_alphabetic())
                .collect::<String>()
        };

        let kv = |s: &str| {
            let key = ident(s);
            assert_eq!(&s[key.len()..key.len() + 1], "=");
            let value: String = s[key.len() + 1..]
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();

            (key, value)
        };

        let mut width = None;
        let mut height = None;
        let mut line_height = None;
        let mut base = None;

        let mut glyphs = Vec::default();

        for line in std::str::from_utf8(bytes).unwrap().lines() {
            match line {
                line if line.starts_with("info") => {}
                line if line.starts_with("common") => {
                    for (key, value) in line.split_whitespace().skip(1).map(kv) {
                        match key.as_str() {
                            "lineHeight" => line_height = value.parse().ok(),
                            "base" => base = value.parse().ok(),
                            "scaleW" => width = value.parse().ok(),
                            "scaleH" => height = value.parse().ok(),
                            _ => {}
                        }
                    }
                }
                line if line.starts_with("chars") => {
                    let (_, value) = line
                        .split_whitespace()
                        .skip(1)
                        .map(kv)
                        .find(|(key, _)| key == "count")
                        .unwrap();

                    glyphs.reserve_exact(value.parse().unwrap());
                }
                line if line.starts_with("char") => {
                    let mut id = None;
                    let mut x = None;
                    let mut y = None;
                    let mut width = None;
                    let mut height = None;
                    let mut xoffset = None;
                    let mut yoffset = None;

                    for (key, value) in line.split_whitespace().skip(1).map(kv) {
                        match key.as_str() {
                            "id" => {
                                id = value
                                    .parse::<u32>()
                                    .map(|c| char::from_u32(c).unwrap())
                                    .ok()
                            }
                            "x" => x = value.parse::<u32>().ok(),
                            "y" => y = value.parse::<u32>().ok(),
                            "width" => width = value.parse::<u32>().ok(),
                            "height" => height = value.parse::<u32>().ok(),
                            "xoffset" => xoffset = value.parse::<i32>().ok(),
                            "yoffset" => yoffset = value.parse::<i32>().ok(),
                            _ => {}
                        }
                    }

                    glyphs.push(FontGlyph {
                        id: id.unwrap(),
                        position: uvec2(x.unwrap(), y.unwrap()),
                        size: uvec2(width.unwrap(), height.unwrap()),
                        offset: ivec2(xoffset.unwrap(), yoffset.unwrap()),
                    });
                }
                _ => {}
            }
        }

        FontFace {
            width: width.unwrap(),
            height: height.unwrap(),
            line_height: line_height.unwrap(),
            base: base.unwrap(),
            glyphs,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct TextVertex {
    position: Vec2,
    texcoord: Vec2,
}

unsafe impl BufferLayout for TextVertex {
    const LAYOUT: &'static [Format] = &[Format::Vec2, Format::Vec2];
    const PADDING: &'static [usize] = &[0, 0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

struct Cache {
    vertices: Buffer<Vertex>,
    matrices: Buffer<Mat4, false, true>,
    materials: Option<Buffer<Material>>,
    buffers: Vec<Buffer<Vec3, false, true>>,
}

pub struct Renderer<'a> {
    window_size: (u32, u32),
    _instance: Instance,
    device: Device<'a>,
    swapchain: Swapchain,
    framebuffer: Framebuffer,
    shaders: ShaderProgram,
    cache: Cache,
    text_renderer: TextRenderer<'a>,
    profiler: Profiler,
}

impl Renderer<'_> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/shader.vert");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/shader.frag");

    pub fn new(window: &Window, vsync: bool, profile: bool) -> Self {
        let _instance = Instance::new(window, true);
        let device = _instance.new_device();
        let swapchain = _instance.new_swapchain(vsync);

        let window_size = window.size();
        let frame = device.new_texture_2d(window_size.0 as _, window_size.1 as _, Format::R8G8B8A8);
        let depth = device.new_texture_2d(window_size.0 as _, window_size.1 as _, Format::D24);
        let framebuffer = device.new_framebuffer(frame, Some(depth));

        let vertex_shader = device.new_shader(VertexStage, Self::VERTEX_SHADER);
        let pixel_shader = device.new_shader(PixelStage, Self::PIXEL_SHADER);
        let shaders = device.new_shader_program(&vertex_shader, &pixel_shader);

        let vertices = device.new_buffer(BufferInit::Data(&Self::VERTICES));
        let matrices = device.new_buffer(BufferInit::Capacity(2));

        Self {
            _instance,
            window_size,
            text_renderer: TextRenderer::new(device.clone(), window_size),
            device,
            swapchain,
            framebuffer,
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
        self.profiler.begin_profile("Renderer");

        self.device
            .default_framebuffer()
            .clear(Vec4::new(0.0, 0.0, 0.0, 0.0), true);

        self.framebuffer.clear(Vec4::new(0.0, 0.0, 0.0, 1.0), true);

        self.geometry_pass(scene);
        self.text_renderer.render(scene, &mut self.framebuffer);

        self.blit();
        self.swapchain.present();

        let measurement = self.profiler.end_profile("Renderer");
        measurement.map(|(cpu, gpu)| cpu + gpu)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let Self { device, .. } = self;

        unsafe { gl!(gl::Viewport(0, 0, width as _, height as _)) }.unwrap();

        self.window_size = (width, height);

        let frame = device.new_texture_2d(width as _, height as _, Format::R8G8B8A8);
        let depth = device.new_texture_2d(width as _, height as _, Format::D24);
        self.framebuffer = device.new_framebuffer(frame, Some(depth));

        self.text_renderer.resize(width, height);
    }

    fn geometry_pass(&mut self, scene: &mut Scene) {
        let Self { device, cache, .. } = self;

        if cache.materials.is_none() {
            cache.materials = Some(device.new_buffer(BufferInit::Data(scene.materials())));
        }

        let view_projection = scene.camera.view_projection();

        // TODO: Batch multiple chunks into a single drawcall.
        for chunk in scene.terrain() {
            self.render_chunk(view_projection, chunk);
        }
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

        device.bind_framebuffer(&self.framebuffer);
        device.draw_instanced(cache.vertices.len(), offsets.len());
    }

    fn postprocess_pass(&mut self) {}

    /// This function blits (copies) all values in `self.framebuffer` to the
    /// default framebuffer
    fn blit(&mut self) {
        unsafe {
            gl!(gl::BlitNamedFramebuffer(
                self.framebuffer.id,
                0,
                0,
                0,
                self.window_size.0 as _,
                self.window_size.1 as _,
                0,
                0,
                self.window_size.0 as _,
                self.window_size.1 as _,
                gl::COLOR_BUFFER_BIT,
                gl::NEAREST,
            ))
        }
        .unwrap();
    }
}

unsafe impl<const N: usize> BufferLayout for [UVec2; N] {
    const LAYOUT: &'static [Format] = &[Format::UVec2; N];
    const PADDING: &'static [usize] = &[0; N];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

unsafe impl BufferLayout for IVec2 {
    const LAYOUT: &'static [Format] = &[Format::IVec2];
    const PADDING: &'static [usize] = &[0];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

unsafe impl<const N: usize> BufferLayout for [IVec2; N] {
    const LAYOUT: &'static [Format] = &[Format::IVec2; N];
    const PADDING: &'static [usize] = &[0; N];
    const COPYABLE: bool = true;

    fn to_bytes(items: &[Self]) -> Vec<u8> {
        unimplemented!()
    }
}

unsafe impl BufferLayout for [Vec2; 2] {
    const LAYOUT: &'static [Format] = &[Format::Vec2, Format::Vec2];
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

use glam::*;

use crate::format::fnt::*;
use crate::rhi::*;
use crate::scene::*;

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

pub struct TextRenderer<'a> {
    device: Device<'a>,
    shaders: ShaderProgram,
    font_face: FontFace,
    atlas: Texture2D,
    matrix_buffer: Buffer<Mat4, false, true>,
}

impl<'a> TextRenderer<'a> {
    const VERTEX_SHADER: &'static str = include_str!("./shaders/text.vert");
    const PIXEL_SHADER: &'static str = include_str!("./shaders/text.frag");

    const FONT_FACE: &'static [u8] = include_bytes!("../../assets/fonts/sans-serif/sans-serif.fnt");
    const FONT_IMAGE: &'static [u8] =
        include_bytes!("../../assets/fonts/sans-serif/sans-serif.png");

    pub fn new(device: Device<'a>, window_size: UVec2) -> Self {
        let shaders = {
            let vs = device.new_shader(VertexStage, Self::VERTEX_SHADER);
            let ps = device.new_shader(PixelStage, Self::PIXEL_SHADER);
            device.new_shader_program(&vs, &ps)
        };

        let font_face = parse(Self::FONT_FACE);
        let mut atlas = device.new_texture_2d(font_face.width, font_face.height, Format::R8G8B8A8);
        let font_image = image::load_from_memory(Self::FONT_IMAGE).unwrap();
        atlas.write(font_image.flipv().as_rgba8().as_ref().unwrap());

        let [width, height] = window_size.to_array().map(|v| v as _);
        let projection = Mat4::orthographic_rh_gl(0.0, width, 0.0, height, 0.0, 1.0);
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
            attributes: &["a_position", "a_texcoord"],
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

    pub fn resize(&mut self, window_size: UVec2) {
        let [width, height] = window_size.to_array().map(|v| v as _);
        let projection = Mat4::orthographic_rh_gl(0.0, width, 0.0, height, 0.0, 1.0);
        self.matrix_buffer = self.device.new_buffer(BufferInit::Data(&[projection]));
    }
}

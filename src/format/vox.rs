// https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt
// https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox-extension.txt

use std::fs::*;
use std::io::*;
use std::mem::*;
use std::path::*;

use byteorder::*;
use glam::*;

type VoxEndian = LittleEndian;

#[derive(Clone, PartialEq, Eq)]
struct VoxChunk {
    id: String,
    content: Vec<u8>,
    chunks: Vec<VoxChunk>,
}

impl std::fmt::Debug for VoxChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("id", &self.id)
            .field("chunks", &self.chunks)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxModel {
    pub transform: Mat4,
    pub size: (usize, usize, usize),
    pub positions: Vec<(Vec3, VoxMaterialId)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxMaterial {
    pub albedo: [u8; 4],
    pub roughness: f32,
    pub metalness: f32,
    pub transparency: f32,
    pub specular: Option<f32>,
    pub ior: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxMaterialId(pub usize);

fn parse_header(input: &mut impl ReadBytesExt) -> ([u8; 4], i32) {
    let signature = {
        let mut buf = [0; 4];
        input.read_exact(&mut buf).unwrap();
        buf
    };

    let version = input.read_i32::<VoxEndian>().unwrap();
    (signature, version)
}

fn parse_chunk(input: &mut impl ReadBytesExt) -> Option<VoxChunk> {
    let mut id = String::from("    ");
    if input.read_exact(unsafe { id.as_bytes_mut() }).is_err() {
        return None;
    };

    let n = input.read_u32::<VoxEndian>().unwrap();
    let m = input.read_u32::<VoxEndian>().unwrap();

    let mut content = vec![0; n as _];
    assert_eq!(input.read(&mut content).unwrap(), n as _);

    let mut children = {
        let mut children = vec![0; m as _];
        assert_eq!(input.read(&mut children).unwrap(), m as _);
        Cursor::new(children)
    };

    let mut chunks = Vec::new();
    while let Some(chunk) = parse_chunk(&mut children) {
        chunks.push(chunk);
    }

    Some(VoxChunk {
        id,
        content,
        chunks,
    })
}

fn parse_model(size: &VoxChunk, positions: &VoxChunk) -> VoxModel {
    assert_eq!(&size.id, "SIZE");
    assert_eq!(&positions.id, "XYZI");

    let size = {
        let mut content = Cursor::new(&size.content);
        let x = content.read_u32::<VoxEndian>().unwrap() as _;
        let y = content.read_u32::<VoxEndian>().unwrap() as _;
        let z = content.read_u32::<VoxEndian>().unwrap() as _;
        (x, y, z)
    };

    let positions = {
        let mut content = Cursor::new(&positions.content);
        let n = content.read_u32::<VoxEndian>().unwrap();
        let mut buf: Vec<u8> = vec![0; n as usize * std::mem::size_of::<u8>() * 4];
        content.read_exact(buf.as_mut_slice()).unwrap();

        buf.array_chunks::<4>()
            .map(|&[x, y, z, i]| (Vec3::new(x as _, y as _, z as _), VoxMaterialId(i as _)))
            .collect()
    };
    VoxModel {
        transform: Mat4::IDENTITY,
        size,
        positions,
    }
}

fn parse_models(chunks: &[VoxChunk]) -> Vec<VoxModel> {
    let mut models = Vec::with_capacity(1);

    let pack = chunks.iter().find(|VoxChunk { id, .. }| id == "PACK");
    if let Some(pack) = pack {
        let mut cursor = Cursor::new(&pack.content);
        let nmodels = cursor.read_u32::<VoxEndian>().unwrap() as _;
        models = Vec::with_capacity(nmodels);
    }

    let iter = chunks.iter().filter(|c| c.id == "SIZE" || c.id == "XYZI");
    for [size, positions] in iter.array_chunks::<2>() {
        models.push(parse_model(size, positions));
    }

    models
}

fn parse_materials(chunks: &[VoxChunk]) -> Box<[VoxMaterial; 256]> {
    let palette: Vec<[u8; 4]> = {
        let chunk = chunks.iter().find(|c| c.id == "RGBA").unwrap();
        let mut content = Cursor::new(&chunk.content);

        let mut buf = Box::new([0; 256 * std::mem::size_of::<u8>() * 4]);
        content.read_exact(buf.as_mut_slice()).unwrap();

        buf.into_iter().array_chunks::<4>().collect()
    };

    let mut materials = Box::new([MaybeUninit::<VoxMaterial>::uninit(); 256]);
    for (i, chunk) in chunks.iter().filter(|c| c.id == "MATL").enumerate() {
        let mut content = Cursor::new(&chunk.content);
        let id = content.read_u32::<VoxEndian>().unwrap() as usize;
        let dict = parse_dict(&mut content);

        let mut roughness = 1.0;
        let mut transparency = 0.0;
        let mut specular = None;
        let mut ior = None;
        for (key, value) in dict {
            match key.as_str() {
                "_rough" => roughness = value.parse().unwrap(),
                "_trans" => transparency = value.parse().unwrap(),
                "_sp" => specular = Some(value.parse().unwrap()),
                "_ior" => ior = Some(value.parse().unwrap()),
                _ => {}
            }
        }

        let material = VoxMaterial {
            albedo: palette[id - 1],
            roughness,
            metalness: 0.0,
            transparency,
            specular,
            ior,
        };

        materials[i] = MaybeUninit::new(material);
    }

    // SAFETY:
    unsafe { std::mem::transmute(materials) }
}

fn parse_string(input: &mut impl ReadBytesExt) -> String {
    let len = input.read_u32::<VoxEndian>().unwrap() as _;
    let mut buf = vec![0; len];
    input.read_exact(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn parse_dict(input: &mut impl ReadBytesExt) -> Vec<(String, String)> {
    let n = input.read_u32::<VoxEndian>().unwrap();

    let mut dict = Vec::new();
    for _ in 0..n {
        let key = parse_string(input);
        let value = parse_string(input);

        dict.push((key, value));
    }

    dict
}

pub fn parse(input: &mut impl ReadBytesExt) -> (Vec<VoxModel>, Box<[VoxMaterial; 256]>) {
    let (signature, version) = parse_header(input);
    assert_eq!((&signature, version), (b"VOX ", 150));

    let main = parse_chunk(input).unwrap();
    assert_eq!(main.id, "MAIN");

    let models = parse_models(&main.chunks);
    let materials = parse_materials(&main.chunks);

    (models, materials)
}

pub fn open(path: impl AsRef<Path>) -> (Vec<VoxModel>, Box<[VoxMaterial; 256]>) {
    let mut file = File::open(path).unwrap();
    parse(&mut file)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[test]
    fn test_parse() {
        let input = include_bytes!("../../assets/knife.vox");
        let mut cursor = Cursor::new(input);
        super::parse(&mut cursor);
    }
}

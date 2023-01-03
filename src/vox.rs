use std::fs::*;
use std::io::*;
use std::path::*;

use byteorder::*;
use glam::*;
use image::EncodableLayout;

use crate::scene::*;

type VoxEndian = LittleEndian;

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

struct VoxModel {
    transform: Mat4,
    size: (usize, usize, usize),
    positions: Vec<[u8; 4]>,
}

// https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt
// https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox-extension.txt
// pub fn open(p: impl AsRef<Path>) {
//     let mut file = File::open(p).unwrap();

//     let mut header = [0; 4];
//     file.read_exact(&mut header).unwrap();
//     let s = unsafe { std::str::from_utf8_unchecked(&header) };
//     assert_eq!(s, "VOX ");

//     let version = file.read_i32::<VoxEndian>().unwrap();
//     println!("{version}");

//     let main = read_chunk(&mut file).unwrap();
//     assert_eq!(main.id, "MAIN");
//     assert_eq!(main.content.len(), 0);

//     // By default the file only contains a single model, but it might contain
// more.     let mut models = Vec::with_capacity(1);
//     let mut positions: Option<Vec<_>> = None;
//     let mut materials: Option<Vec<_>> = None;
//     let mut transform: Option<Mat4> = None;

//     for chunk in main.children {
//         let Chunk { id, .. } = &chunk;
//         let mut content = Cursor::new(&chunk.content);

//         println!("{id}");

//         match id.as_str() {
//             // The "PACK" chunk is guaranteed to come before any models
// inside the file.             "PACK" => {
//                 models =
// Vec::with_capacity(content.read_u32::<VoxEndian>().unwrap() as _);
//             }
//             "SIZE" => {
//                 let mut size = [0, 0, 0];
//                 content.read_i32_into::<VoxEndian>(&mut size).unwrap();
//             }
//             "XYZI" => {
//                 let n = content.read_u32::<VoxEndian>().unwrap();
//                 let mut buf = vec![0; n as usize * (std::mem::size_of::<u8>()
// * 4)];                 content.read(&mut buf).unwrap();

//                 let it = buf.array_chunks::<4>().map(|&[x, y, z, i]| ([x, y,
// z], i));                 positions = Some(it.clone().map(|(xyz, _)|
// xyz).collect());                 materials = Some(it.map(|(_, i)|
// Material(i)).collect());             }
//             "LAYR" => {
//                 let id = content.read_i32::<VoxEndian>().unwrap();
//                 let nkeys = content.read_i32::<VoxEndian>().unwrap();

//                 let key = read_string(&mut content);
//                 let value = read_string(&mut content);
//                 println!("{key} : {value}");
//                 let _reserved = content.read_i32::<VoxEndian>().unwrap();
//             }
//             "nTRN" => {
//                 let id = content.read_i32::<VoxEndian>().unwrap();

//                 let _ = content.read_i32::<VoxEndian>().unwrap();
//                 let name = (read_string(&mut content), read_string(&mut
// content));                 let _hidden = (read_string(&mut content),
// read_string(&mut content));

//                 let _ = content.read_i32::<VoxEndian>().unwrap();
//                 let _ = content.read_i32::<VoxEndian>().unwrap();
//                 let _ = content.read_i32::<VoxEndian>().unwrap();
//                 let n = content.read_i32::<VoxEndian>().unwrap();
//                 (0..n).for_each(|_| {
//                     let j = content.read_i32::<VoxEndian>().unwrap();
//                     let _ = (read_string(&mut content),
// content.read_i8().unwrap());

//                     let mut x = [0, 0, 0];
//                     let _ = (
//                         read_string(&mut content),
//                         content.read_i32_into::<VoxEndian>(&mut x).unwrap(),
//                     );

//                     let _ = (
//                         read_string(&mut content),
//                         content.read_i32::<VoxEndian>().unwrap(),
//                     );
//                 })
//             }
//             "nGRP" => {}
//             "nSHP" => {}
//             _ => {}
//         }

//         match (positions.take(), materials.take(), transform.take()) {
//             (Some(positions), Some(materials), transform) => {
//                 let model = Model {
//                     positions,
//                     materials,
//                     transform: transform.unwrap_or(Mat4::IDENTITY),
//                 };

//                 models.push(model);
//             }
//             _ => {}
//         }
//     }
// }

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

fn parse_model(input: &mut impl ReadBytesExt) -> VoxModel {
    let size = {
        let chunk = parse_chunk(input).unwrap();
        let mut content = Cursor::new(chunk.content);
        assert_eq!(&chunk.id, "SIZE");

        let x = content.read_u32::<VoxEndian>().unwrap() as _;
        let y = content.read_u32::<VoxEndian>().unwrap() as _;
        let z = content.read_u32::<VoxEndian>().unwrap() as _;
        (x, y, z)
    };

    let positions = {
        let chunk = parse_chunk(input).unwrap();
        let mut content = Cursor::new(chunk.content);

        let n = content.read_u32::<VoxEndian>().unwrap();
        let mut positions: Vec<u8> = vec![0; n as usize * std::mem::size_of::<u8>() * 4];
        content.read_exact(positions.as_mut_slice()).unwrap();
        Vec::from(positions.as_chunks::<4>().0)
    };

    VoxModel {
        transform: Mat4::IDENTITY,
        size,
        positions,
    }
}

pub fn parse(input: &mut impl ReadBytesExt) -> Vec<Chunk> {
    let header = {
        let mut buf = [0; 4];
        input.read_exact(&mut buf).unwrap();
        buf
    };
    assert_eq!(&header, b"VOX ");

    let version = input.read_i32::<VoxEndian>().unwrap();
    assert_eq!(version, 150);

    let main = parse_chunk(input).unwrap();
    assert_eq!(main.id, "MAIN");
    assert!(main.content.is_empty());

    let pack = main.chunks.iter().find(|VoxChunk { id, .. }| id == "PACK");
    let nmodels = if let Some(pack) = pack {
        let mut cursor = Cursor::new(&pack.content);
        cursor.read_u32::<VoxEndian>().unwrap() as _
    } else {
        1
    };

    let mut models: Vec<VoxModel> = Vec::with_capacity(nmodels);
    main.chunks
        .iter()
        .filter(|VoxChunk { id, .. }| id == "SIZE" || id == "XYZI");

    main.chunks
        .iter()
        .for_each(|chunk| println!("{}", chunk.id));

    Vec::default()
}

// TODO: Avoid allocations with lifetimes. (?)

// fn read_chunk(reader: &mut impl ReadBytesExt) -> Option<Chunk> {
//     let mut id = [0; 4];
//     let id = if let Ok(n) = reader.read(&mut id) {
//         if n != id.len() {
//             return None;
//         };

//         unsafe { std::str::from_utf8_unchecked(&id) }.to_string()
//     } else {
//         return None;
//     };

//     let len = reader.read_i32::<VoxEndian>().unwrap();
//     let mut content = vec![0; len as _];

//     let len = reader.read_i32::<VoxEndian>().unwrap();
//     let mut buf = vec![0; len as _];

//     reader.read(&mut content).unwrap();
//     reader.read(&mut buf).unwrap();

//     let mut cursor = Cursor::new(buf);
//     let mut children = Vec::with_capacity(16);
//     while let Some(chunk) = read_chunk(&mut cursor) {
//         children.push(chunk);
//     }

//     Some(Chunk {
//         id,
//         content,
//         children,
//     })
// }

// fn read_dict(reader: &mut impl ReadBytesExt) -> (String, String) {
//     let n = reader.read_i32::<VoxEndian>().unwrap();
//     (read_string(reader), read_string(reader))
// }

// fn read_string(reader: &mut impl ReadBytesExt) -> String {
//     let size = reader.read_u32::<VoxEndian>().unwrap();
//     let mut buf = vec![0; (size) as _];
//     reader.read(&mut buf).unwrap();

//     String::from_utf8(buf).unwrap()
// }

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[test]
    fn test_parse() {
        let input = include_bytes!("../assets/plant.vox");
        let mut cursor = Cursor::new(input);
        super::parse(&mut cursor);
    }
}

use std::fs::*;
use std::io::*;
use std::path::*;

use byteorder::*;

type VoxEndian = LittleEndian;

pub fn open(p: impl AsRef<Path>) {
    let mut file = File::open(p).unwrap();

    let mut header = [0; 4];
    file.read_exact(&mut header).unwrap();
    let s = unsafe { std::str::from_utf8_unchecked(&header) };
    assert_eq!(s, "VOX ");

    let version = file.read_i32::<VoxEndian>().unwrap();
    println!("{version}");

    let main = read_chunk(&mut file);
    // assert_eq!(main.id, "MAIN");
    println!("{main:?}");
}

struct Chunk {
    id: String,
    content: Vec<u8>,
    children: Vec<Chunk>,
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("id", &self.id)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

fn read_chunk(reader: &mut impl ReadBytesExt) -> Option<Chunk> {
    let mut id = [0; 4];
    let id = if let Ok(n) = reader.read(&mut id) {
        if n != id.len() {
            return None;
        };

        unsafe { std::str::from_utf8_unchecked(&id) }.to_string()
    } else {
        return None;
    };

    println!("{id}");

    let len = reader.read_i32::<VoxEndian>().unwrap();
    let mut content = vec![0; len as _];

    let len = reader.read_i32::<VoxEndian>().unwrap();
    let mut buf = vec![0; len as _];

    reader.read(&mut content).unwrap();
    reader.read(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);

    let mut children = Vec::with_capacity(16);
    while let Some(chunk) = read_chunk(&mut cursor) {
        children.push(chunk);
    }

    Some(Chunk {
        id,
        content,
        children,
    })
}

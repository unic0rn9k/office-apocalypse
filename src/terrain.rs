use glam::{Mat4, Vec3};

use crate::tensor::SparseTensorChunk;

const FOV: usize = 10;

struct TerrainCache {
    player_position_on_last_udpate: Vec3,
}

enum Asset {
    Kitchen,
    Door,
}

impl Asset {
    fn chunk(transform: Mat4) -> SparseTensorChunk {
        // It would be nice to have some way of merging chunks.
        // It will be bad if chunks are merged after there compressed.
        // The whole tensor implementation is a premature optimization.
        // Should just switch to HashMap for now

        todo!()
    }
}

struct Map {
    data: [[Asset; FOV]; FOV],
}

/// # Notes
/// - Either only generate the new delta terrain that needs to be generated
/// - Only generate new tarrain on a 'room' basis
fn gen_terrain(src: &mut Vec<SparseTensorChunk>, player_location: Vec3) {
    src.clear();
    todo!()
}

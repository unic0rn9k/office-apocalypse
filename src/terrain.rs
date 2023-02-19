use std::ops::{Range, RangeBounds};

use glam::{Mat4, Vec3};

use crate::tensor::SparseTensorChunk;

const FOV: usize = 10;

fn random(v: Vec3, r: Range<usize>) -> usize {
    let a: usize = match r.start_bound() {
        std::ops::Bound::Included(a) => *a,
        _ => panic!("invalid bound for random number generation"),
    };
    let b: usize = match r.end_bound() {
        std::ops::Bound::Excluded(a) => *a,
        _ => panic!("invalid bound for random number generation"),
    } - a;
    (((v.x + v.y + v.z) * 100.).abs() as usize % b) + a
}

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
        // It will be bad if chunks are merged after they're compressed.
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

#[test]
fn random_test() {
    assert_eq!(
        random(
            Vec3 {
                x: 3000.,
                y: 3000.,
                z: 3000.
            },
            0..1
        ),
        0
    )
}

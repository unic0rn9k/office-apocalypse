use std::ops::{Range, RangeBounds};

use glam::{vec3, Mat4, UVec3, Vec3};

use crate::format::vox::{self, VoxModel};
use crate::scene::Model;
use crate::tensor::SparseTensorChunk;

const FOV: usize = 10; // Must be even
const CUBICAL_SIZE: u32 = 40;
const SEED: f32 = 123.4;

fn random(v: Vec3, r: Range<usize>) -> usize {
    let a: usize = match r.start_bound() {
        std::ops::Bound::Included(a) => *a,
        _ => panic!("invalid bound for random number generation"),
    };
    let b: usize = match r.end_bound() {
        std::ops::Bound::Excluded(b) => *b,
        _ => panic!("invalid bound for random number generation"),
    } - a;
    (((v.x + v.y + v.z) * SEED).abs() as usize % b) + a
}

#[derive(Clone, Copy, Debug)]
enum Asset {
    Kitchen,
    Door,
    Nil,
}
const ASSETS: &[Asset] = &[Asset::Kitchen, Asset::Door];

impl Asset {
    fn path(&self) -> String {
        use Asset::*;
        format!(
            "assets/{}.vox",
            match self {
                Kitchen => "kitchen",
                Door => "door",
                Nil => panic!("Tried to load nil-asset"),
            }
        )
    }

    fn chunk(&self, map_pos: UVec3) -> SparseTensorChunk {
        let mut ret = SparseTensorChunk::nothing(UVec3::ZERO);
        let translation = (map_pos * CUBICAL_SIZE).as_vec3();

        let rotate_90 = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let transform = Mat4::from_translation(translation);

        let path = self.path();
        let chunk = SparseTensorChunk::from(Model::from(vox::open(path).0[0].clone()));

        //ret
        todo!()
    }
}

fn blk_pos(x: usize, y: usize, center: Vec3) -> Vec3 {
    let min = FOV as f32 / -2.;
    let min = vec3(min, center.y, min);
    let p = vec3(x as f32 * FOV as f32, center.y, y as f32 * FOV as f32);

    center + min + p
}

struct TerrainMask([[bool; FOV]; FOV]);

struct MapBlock {
    center: Vec3,
    data: [[Asset; FOV]; FOV],
}

impl MapBlock {
    fn from_scratch(pos: Vec3) -> Self {
        let mut data = [[Asset::Nil; FOV]; FOV];

        for y in 0..FOV {
            for x in 0..FOV {
                let blk_pos = blk_pos(x, y, pos);
                data[y][x] = ASSETS[random(blk_pos, 0..ASSETS.len())]
            }
        }

        MapBlock { center: pos, data }
    }

    /// A mask of elements that needs to be added to the terrain,
    /// for the new self map, based on the old_pos
    fn mask(&self, old_pos: Vec3) -> TerrainMask {
        let mut tmp = TerrainMask([[true; FOV]; FOV]);

        for y in 0..FOV {
            for x in 0..FOV {
                let new_pos = blk_pos(x, y, self.center);
                if old_pos.abs_diff_eq(new_pos, FOV as f32) {
                    tmp.0[y][x] = false;
                }
            }
        }

        tmp
    }

    fn gen_terrain(&self, mask: TerrainMask) -> SparseTensorChunk {
        todo!()
    }
}

impl std::fmt::Debug for MapBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = ASSETS.iter().map(|a| format!("{a:?}").len()).max().unwrap() + 3;
        for y in 0..FOV {
            for x in 0..FOV {
                let lbl = format!("{:?}", self.data[y][x]);
                write!(f, "{lbl}{}", " ".repeat(len - lbl.len()))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[test]
fn map() {
    let a = vec3(1., 2., 3.);
    let b = vec3(2., 0., 1.);
    let c = a + b;

    let a_map = MapBlock::from_scratch(a);
    let b_map = MapBlock::from_scratch(c);

    println!("{a_map:?}");
    println!();
    println!("{b_map:?}");
    println!();

    let mask = b_map.mask(a);

    for y in 0..FOV {
        for x in 0..FOV {
            print!("{}", if mask.0[y][x] { "." } else { "#" });
        }
        println!()
    }
}

#[test]
fn _random() {
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

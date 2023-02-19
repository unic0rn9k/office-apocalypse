use std::ops::{Range, RangeBounds};

use glam::{vec3, Mat4, UVec3, Vec3};

use crate::format::vox::{self, VoxModel};
use crate::scene::Model;
use crate::tensor::{self, SparseTensorChunk};

const FOV: usize = 6; // Must be even
const CUBICAL_SIZE: u32 = 40;
const SEED: f32 = 123.4;

fn random(v: Vec3, r: Range<usize>, variant: usize) -> usize {
    let a: usize = match r.start_bound() {
        std::ops::Bound::Included(a) => *a,
        _ => panic!("invalid bound for random number generation"),
    };
    let b: usize = match r.end_bound() {
        std::ops::Bound::Excluded(b) => *b,
        _ => panic!("invalid bound for random number generation"),
    } - a;

    let x = (v.x * SEED).abs() as usize;
    let y = (v.y * SEED).abs() as usize + variant;
    let z = (v.z * SEED).abs() as usize;

    let r = (x | z) & y;

    r % b + a
}

macro_rules! assets {
    ($($asset: ident),*) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug)]
        enum Asset {
            $($asset,)*
            Nil,
        }
        const ASSETS: &[Asset] = &[$(Asset::$asset),*];

        impl Asset {
            fn path(&self) -> String {
                use Asset::*;
                format!(
                    "assets/{}.vox",
                    match self {
                        $($asset => stringify!($asset),)*
                        Nil => panic!("Tried to load nil-asset"),
                    }
                )
            }
        }
}}

assets!(
    kitchen,
    chair,
    desk,
    doorframe,
    floor,
    kitchen_island,
    laptop,
    plant,
    wall
);

impl Asset {
    fn chunk(&self, map_pos: UVec3) -> SparseTensorChunk {
        let translation = (map_pos * CUBICAL_SIZE).as_vec3();

        let rotate_90 = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let transform = Mat4::from_translation(translation);

        let path = self.path();
        let mut chunk = SparseTensorChunk::from(Model::from(vox::open(path).0[0].clone()));

        chunk.transform *= transform;
        chunk
    }
}

fn blk_pos(x: usize, y: usize, center: Vec3) -> Vec3 {
    let min = (FOV as f32 / -2.) * CUBICAL_SIZE as f32;
    let min = vec3(min, 0., min);
    let p = vec3(
        x as f32 * CUBICAL_SIZE as f32,
        0.,
        y as f32 * CUBICAL_SIZE as f32,
    );

    center + min + p
}

pub struct TerrainMask([[bool; FOV]; FOV]);
pub const EMPTY_MASK: TerrainMask = TerrainMask([[true; FOV]; FOV]);

pub struct MapBlock {
    center: Vec3,
    data: [[Asset; FOV]; FOV],
}

impl MapBlock {
    pub fn from_scratch(pos: Vec3) -> Self {
        let mut data = [[Asset::Nil; FOV]; FOV];

        for y in 0..FOV {
            for x in 0..FOV {
                let blk_pos = blk_pos(x, y, pos);
                data[y][x] = ASSETS[random(blk_pos, 0..ASSETS.len(), 0)]
            }
        }

        MapBlock { center: pos, data }
    }

    /// A mask of elements that needs to be added to the terrain,
    /// for the new self map, based on the old_pos
    pub fn mask(&self, old_pos: Vec3) -> TerrainMask {
        let mut tmp = TerrainMask([[true; FOV]; FOV]);

        for y in 0..FOV {
            for x in 0..FOV {
                let new_pos = blk_pos(x, y, self.center);
                if old_pos.abs_diff_eq(new_pos, CUBICAL_SIZE as f32) {
                    tmp.0[y][x] = false;
                }
            }
        }

        tmp
    }

    pub fn gen_terrain(&self, mask: TerrainMask) -> SparseTensorChunk {
        let mut ret = SparseTensorChunk::nothing(UVec3::ZERO);

        for y in 0..FOV {
            for x in 0..FOV {
                if mask.0[y][x] {
                    let pos = blk_pos(x, y, self.center);
                    ret = tensor::combine(ret, self.data[y][x].chunk(pos.as_uvec3()));
                }
            }
        }

        ret
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

pub fn closest_block(p: Vec3) -> Vec3 {
    let mut tmp = (p.as_uvec3() / CUBICAL_SIZE).as_vec3() * CUBICAL_SIZE as f32;
    tmp.y = 31.;
    tmp
}

#[test]
fn map() {
    let a = closest_block(vec3(1., 2., 3.));
    let b = closest_block(vec3(80., 2., 45.));

    let a_map = MapBlock::from_scratch(a);
    let b_map = MapBlock::from_scratch(b);

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
fn block_coordinates() {
    assert_eq!(blk_pos(5, 5, vec3(0., 0., 0.)), vec3(0., 0., 0.));
    assert_eq!(blk_pos(5, 5, vec3(1., 1., 1.)), vec3(1., 1., 1.));
    assert_eq!(
        blk_pos(7, 6, vec3(0., 1., 40.)),
        vec3(2. * CUBICAL_SIZE as f32, 1., 2. * CUBICAL_SIZE as f32)
    );
}

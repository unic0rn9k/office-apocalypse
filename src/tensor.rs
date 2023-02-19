use std::collections::HashMap;
use std::iter::FilterMap;

use glam::{Mat4, UVec3, Vec3, Vec4};

use crate::scene::{MaterialId, Model};

/// # Notes
/// The tensor functionality will be used for:
/// - collision detection
/// - destructible terrain
#[derive(Debug, Clone)]
pub struct SparseTensorChunk {
    pub data: HashMap<UVec3, MaterialId>,
    pub transform: Mat4,
    pub dim: UVec3,
}

impl SparseTensorChunk {
    pub fn remove(&mut self, i: UVec3) {
        self.data.remove(&i);
    }

    pub fn insert(&mut self, i: UVec3, vox: Option<MaterialId>) {
        if let Some(vox) = vox {
            self.data.insert(i, vox);
        }
    }

    pub fn voxel(&self, i: UVec3) -> Option<&MaterialId> {
        self.data.get(&i)
    }
    pub fn voxel_mut(&mut self, i: UVec3) -> Option<&mut MaterialId> {
        self.data.get_mut(&i)
    }

    pub fn nothing() -> Self {
        Self {
            dim: UVec3::ZERO,
            data: HashMap::new(),
            transform: Mat4::IDENTITY, //lower_bound: UVec3::ZERO,
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&UVec3, &MaterialId)> {
        self.data.iter()
    }

    // pub fn from_model(model: &[(UVec3, MaterialId)], dim: UVec3) -> Self {
    //     //let mut min_bound = model[0].0;
    //     //let mut max_bound = model[0].0;

    //     //for (p, _) in model {
    //     //    min_bound = p.min(min_bound);
    //     //    max_bound = p.max(max_bound);
    //     //}

    //     //let dim = max_bound - min_bound;
    //     let mut tmp = Self::nothing(dim);
    //     //tmp.lower_bound = min_bound;

    //     for (p, m) in model {
    //         tmp.insert(*p, Some(*m))
    //     }
    //     tmp.compress();
    //     tmp
    // }
}

impl From<Model> for SparseTensorChunk {
    fn from(value: Model) -> Self {
        let mut temp = Self::nothing();
        temp.transform *= value.transform;

        let mut dim = UVec3::ZERO;

        for (position, material_id) in value.positions {
            let index = UVec3::from_array(position.to_array().map(|v| v as _));
            temp.insert(index, Some(material_id));
        }
        temp
    }
}

impl<'a> IntoIterator for &'a SparseTensorChunk {
    type IntoIter = <HashMap<UVec3, MaterialId> as IntoIterator>::IntoIter;
    type Item = <HashMap<UVec3, MaterialId> as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// *DOES NOT AUTO-COMPRESS*
pub fn combine(a: SparseTensorChunk, b: SparseTensorChunk) -> SparseTensorChunk {
    let v4 = |v: Vec3| Vec4::from_array([v.x, v.y, v.z, 1.]);
    let v3 = |v: Vec4| Vec3::from_slice(&v.to_array()[0..3]).as_uvec3();

    let mut c = SparseTensorChunk::nothing();

    let map = |t: Mat4| move |(a, b): (UVec3, MaterialId)| (v3(t * v4(a.as_vec3())), b);

    for (position, material_id) in a
        .into_iter()
        .map(map(a.transform))
        .chain(b.into_iter().map(map(b.transform)))
    {
        let index = UVec3::from_array(position.to_array().map(|v| v as _));
        c.insert(index, Some(material_id));
    }

    c
}

//fn combine_many(t: &[SparseTensorChunk]) -> SparseTensorChunk

/*
#[cfg(test)]
mod test {
    extern crate test;
    use std::collections::HashMap;

    use test::{black_box, Bencher};

    use crate::scene::MaterialId;
    use crate::tensor::SparseTensorChunk;

    #[test]
    fn same_same() {
        let m = MaterialId(0);
        let model = [
            ((0, 0, 0).into(), m),
            ((0, 1, 0).into(), m),
            ((1, 0, 0).into(), m),
            ((1, 0, 1).into(), m),
            ((1, 1, 1).into(), m),
            ((0, 0, 2).into(), m),
        ];
        let t = SparseTensorChunk::from(model);
        let mut t2 = HashMap::new();

        for (p, m) in &model {
            t2.insert(p.to_array(), *m);
        }

        for z in 0..3 {
            for x in 0..2 {
                for y in 0..2 {
                    assert_eq!(t.voxel((x, y, z).into()).map(|v| &v.1), t2.get(&[x, y, z]))
                }
            }
        }

        /*
        println!("{:?}", t.nodes);
        println!("----");
        for z in 0..3 {
            for x in 0..2 {
                print!("|");
                for y in 0..2 {
                    match t.voxel((x, y, z).into()) {
                        Some(_) => print!("#"),
                        None => print!("."),
                    }
                }
                println!("|");
            }
            println!("----");
        }
        println!();
        println!("----");
        for z in 0..3 {
            for x in 0..2 {
                print!("|");
                for y in 0..2 {
                    match t2.get(&[x, y, z]) {
                        Some(_) => print!("#"),
                        None => print!("."),
                    }
                }
                println!("|");
            }
            println!("----");
        }
        */
    }

    #[bench]
    fn weird(b: &mut Bencher) {
        let m = black_box(MaterialId(0));
        let model = [
            black_box(((0, 0, 0).into(), m)),
            black_box(((0, 1, 0).into(), m)),
            black_box(((1, 0, 0).into(), m)),
            black_box(((1, 0, 1).into(), m)),
            black_box(((1, 1, 1).into(), m)),
            black_box(((0, 0, 2).into(), m)),
        ];
        let t = SparseTensorChunk::from_model(&model, (2, 2, 3).into());
        b.iter(|| black_box(t.voxel(black_box((1, 1, 1).into()))))
    }

    #[bench]
    fn hashmap(b: &mut Bencher) {
        let m = black_box(MaterialId(0));
        let model = [
            black_box(([0, 0, 0], m)),
            black_box(([0, 1, 0], m)),
            black_box(([1, 0, 0], m)),
            black_box(([1, 0, 1], m)),
            black_box(([1, 1, 1], m)),
            black_box(([0, 0, 2], m)),
        ];

        let mut t = HashMap::new();
        for (p, m) in &model {
            t.insert(p, m);
        }

        b.iter(|| black_box(t.get(black_box(&[1, 1, 1]))))
    }
}
*/

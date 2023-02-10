use std::iter::FilterMap;

use glam::UVec3;

use crate::scene::MaterialId;

#[derive(Clone, Debug)]
pub enum SparseNode {
    Nothing(u32),
    Voxel((UVec3, MaterialId)),
}
use SparseNode::*;

impl SparseNode {
    fn voxel(&self) -> &(UVec3, MaterialId) {
        match self {
            Nothing(_) => panic!("Called voxel on `SparseNode::Nothing`"),
            Voxel(v) => v,
        }
    }
    fn voxel_mut(&mut self) -> &mut (UVec3, MaterialId) {
        match self {
            Nothing(_) => panic!("Called voxel_mut on `SparseNode::Nothing`"),
            Voxel(v) => v,
        }
    }

    fn nils(&self) -> u32 {
        // Hej Nils :)
        match self {
            Nothing(n) => *n,
            Voxel(_) => 0,
        }
    }

    fn add_nils(&mut self, i: u32) {
        match self {
            Nothing(n) => *n += i,
            Voxel(_) => panic!("Called add_nil on `SparseNode::Voxel`"),
        }
    }
}

pub struct SparseTensorChunk {
    nodes: Vec<SparseNode>,
    pub dim: UVec3,
}

impl SparseTensorChunk {
    pub fn compress(&mut self) {
        let mut prev_was_nil = false;
        let mut n = 0;

        while n < self.nodes.len() {
            let nils = self.nodes[n].nils();
            if prev_was_nil && nils != 0 {
                self.nodes.remove(n);
                self.nodes[n - 1].add_nils(nils);
                prev_was_nil = true;
                continue;
            }
            n += 1;
        }
    }

    fn idx(&self, i: UVec3) -> Option<usize> {
        let i = self.near_idx(i);
        match self.nodes[i] {
            Nothing(_) => None,
            Voxel(_) => Some(i),
        }
    }

    pub fn remove(&mut self, i: UVec3) {
        if let Some(i) = self.idx(i) {
            let mut has_neighbor = false;
            if let Some(Nothing(n)) = self.nodes.get_mut(i + 1) {
                *n += 1;
                has_neighbor = true;
            } else if let Some(Nothing(n)) = self.nodes.get_mut(i - 1) {
                *n += 1;
                has_neighbor = true;
            }
            if has_neighbor {
                self.nodes.remove(i);
            } else {
                self.nodes[i] = Nothing(1);
            }
        }
    }

    pub fn near_idx(&self, i_: UVec3) -> usize {
        let i = i_[0] + i_[1] * self.dim[0] + i_[2] * self.dim[1] * self.dim[0];

        if i > self.dim.to_array().iter().product() {
            panic!("index {i_:?} out of bounds {:?}", self.dim);
        }

        let mut j = 0;

        for (n, node) in self.nodes.iter().enumerate() {
            if j > i {
                return n;
            }
            if j == i {
                return n;
            }
            match node {
                SparseNode::Nothing(nils) => j += nils,
                SparseNode::Voxel(_) => j += 1,
            }
        }

        self.nodes.len() - 1
    }

    pub fn insert(&mut self, i: UVec3, vox: Option<MaterialId>) {
        let node = if let Some(v) = vox {
            Voxel((i, v))
        } else {
            Nothing(1)
        };
        let i = self.near_idx(i);

        match self.nodes[i] {
            Nothing(1) | Voxel(_) => self.nodes[i] = node,
            _ => panic!("Cannot insert space slot containing multiple empty voxels"),
        }
    }

    pub fn voxel(&self, i: UVec3) -> Option<&(UVec3, MaterialId)> {
        assert!(
            i.x < self.dim.x && i.y < self.dim.y && i.z < self.dim.z,
            "{i:?} out of bounds {:?}",
            self.dim
        );
        self.idx(i).map(|i| self.nodes[i].voxel())
    }
    pub fn voxel_mut(&mut self, i: UVec3) -> Option<&mut (UVec3, MaterialId)> {
        assert!(
            i.x < self.dim.x && i.y < self.dim.y && i.z < self.dim.z,
            "{i:?} out of bounds {:?}",
            self.dim
        );
        self.idx(i).map(|i| self.nodes[i].voxel_mut())
    }

    pub fn nothing(dim: UVec3) -> Self {
        Self {
            nodes: vec![Nothing(1); dim.to_array().iter().product::<u32>() as usize],
            dim,
        }
    }

    pub fn from_model(model: &[(UVec3, MaterialId)], dim: UVec3) -> Self {
        // TODO: do this
        //let mut min_bound = model[0].0;
        //let mut max_bound = model[0].0;

        //for (p, _) in model {
        //    min_bound = p.min(min_bound);
        //    max_bound = p.min(max_bound);
        //}

        //let dim = max_bound - min_bound;
        let mut tmp = Self::nothing(dim);

        for (p, m) in model {
            tmp.insert(*p, Some(*m))
        }
        tmp.compress();
        tmp
    }
}

impl<'a> IntoIterator for &'a SparseTensorChunk {
    type Item = &'a (UVec3, MaterialId);

    type IntoIter = FilterMap<
        std::slice::Iter<'a, SparseNode>,
        fn(&SparseNode) -> Option<&(UVec3, MaterialId)>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter().filter_map(|node| match node {
            Nothing(_) => None,
            Voxel(some) => Some(some),
        })
    }
}

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
        let t = SparseTensorChunk::from_model(&model, (2, 2, 3).into());
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
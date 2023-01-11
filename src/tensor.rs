use glam::{UVec3, Vec3Swizzles};

use crate::scene::MaterialId;

pub enum SparseNode {
    Nothing(u32),
    Voxel(MaterialId),
}
use SparseNode::*;

impl SparseNode {
    fn voxel(&self) -> &MaterialId {
        match self {
            Nothing(_) => panic!("Called voxel on `SparseNode::Nothing`"),
            Voxel(v) => v,
        }
    }
    fn voxel_mut(&mut self) -> &mut MaterialId {
        match self {
            Nothing(_) => panic!("Called voxel_mut on `SparseNode::Nothing`"),
            Voxel(v) => v,
        }
    }

    fn is_nil(&self) -> bool {
        match self {
            Nothing(_) => true,
            Voxel(_) => false,
        }
    }

    fn add_nil(&mut self) {
        match self {
            Nothing(n) => *n += 1,
            Voxel(_) => panic!("Called add_nil on `SparseNode::Voxel`"),
        }
    }
}

pub struct SparseTensorChunk {
    nodes: Vec<SparseNode>,
    dim: UVec3,
}

impl SparseTensorChunk {
    pub fn compress(&mut self) {
        let mut prev_was_nil = false;
        let mut n = 0;

        while n < self.nodes.len() {
            let is_nil = self.nodes[n].is_nil();
            if prev_was_nil && is_nil {
                self.nodes.remove(n);
                self.nodes[n - 1].add_nil();
                continue;
            }
            prev_was_nil = is_nil;
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
        let i = i_[0] + i_[1] * self.dim[0] + i_[2] * self.dim[1];

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
                SparseNode::Nothing(nils) => j += nils, // Hej Nils :)
                SparseNode::Voxel(_) => j += 1,
            }
        }

        self.nodes.len() - 1
    }

    pub fn insert(&mut self, i: UVec3, vox: Option<MaterialId>) {
        todo!() // Cant do this, without making it a graph :/
    }

    pub fn voxel(&self, i: UVec3) -> Option<&MaterialId> {
        self.idx(i).map(|i| self.nodes[i].voxel())
    }
    pub fn voxel_mut(&mut self, i: UVec3) -> Option<&mut MaterialId> {
        self.idx(i).map(|i| self.nodes[i].voxel_mut())
    }

    pub fn nothing(dim: UVec3) -> Self {
        Self { nodes: vec![], dim }
    }

    pub fn from_model(model: Vec<(UVec3, MaterialId)>, size: UVec3) -> Self {
        todo!()
    }
}

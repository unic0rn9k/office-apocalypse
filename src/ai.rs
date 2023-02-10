use std::collections::{vec_deque, HashMap, HashSet, VecDeque};

use glam::{IVec3, UVec3};

use crate::scene::MaterialId;
use crate::tensor::SparseTensorChunk;

struct Thing {
    position: UVec3,
    route: Vec<UVec3>,
}

impl Thing {
    // Dejstra path finding (breadth first search)
    // The algorithm will spin forever, if there is no path.
    fn destination(&mut self, dest: UVec3, scene: &SparseTensorChunk) -> Vec<UVec3> {
        let mut reached = HashMap::<IVec3, Vec<IVec3>>::new();
        reached.insert(self.position.as_ivec3(), vec![]);

        let dirs = [
            IVec3::from([1, 0, 0]),
            IVec3::from([-1, 0, 0]),
            IVec3::from([0, 0, 1]),
            IVec3::from([0, 0, -1]),
        ];

        let mut path_by_points = vec![];

        let mut next_up = VecDeque::from([self.position.as_ivec3()]);
        let mut p;
        loop {
            match next_up.pop_front() {
                Some(next) => p = next,
                None => unreachable!(),
            }

            if p.as_uvec3() == dest {
                let mut ret = reached.get(&p).unwrap().clone();
                ret.push(dest.as_ivec3());
                path_by_points = ret;
                break;
            }

            for n in dirs {
                if reached.contains_key(&(p + n))
                    || (p + n).clamp(
                        IVec3::ZERO,
                        scene.dim.as_ivec3() - IVec3 { x: 1, y: 1, z: 1 },
                    ) != (p + n)
                    || scene.voxel((p + n).as_uvec3()).is_some()
                {
                    continue;
                }

                let mut tmp = match reached.get(&p) {
                    Some(some) => some.clone(),
                    None => unreachable!(),
                };
                tmp.push(p);
                reached.insert(p + n, tmp);
                next_up.push_back(p + n);
            }
        }

        path_by_points.iter().map(IVec3::as_uvec3).collect()
    }
}

#[test]
fn straight() {
    let mut thing = Thing {
        position: UVec3 { x: 0, y: 0, z: 0 },
        route: vec![],
    };

    let mut env = SparseTensorChunk::nothing(UVec3 { x: 4, y: 4, z: 4 });

    env.insert(UVec3 { x: 1, y: 0, z: 0 }, Some(MaterialId(0)));

    let path = thing.destination(UVec3 { x: 3, y: 0, z: 0 }, &env);

    assert_eq!(
        path,
        vec![
            UVec3 { x: 0, y: 0, z: 0 },
            UVec3 { x: 1, y: 0, z: 0 },
            UVec3 { x: 2, y: 0, z: 0 },
        ]
    )
}

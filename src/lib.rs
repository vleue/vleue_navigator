use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        pipeline::PrimitiveTopology,
    },
    utils::{HashMap, HashSet},
};
use petgraph::{algo::connected_components, graphmap::UnGraphMap};

#[derive(Default)]
pub struct NavMesh {
    triangles: Vec<((Vec3, usize), (Vec3, usize), (Vec3, usize))>,
    graph: UnGraphMap<usize, f32>,
    vertices: Vec<Vec3>,
}
impl NavMesh {
    pub fn from_mesh(mesh: &Mesh) -> NavMesh {
        info!("creating navmesh");
        fn mesh_to_option(mesh: &Mesh) -> Option<NavMesh> {
            let indices = match mesh.primitive_topology() {
                PrimitiveTopology::TriangleList => mesh.indices()?,
                PrimitiveTopology::TriangleStrip => mesh.indices()?,
                _ => return None,
            };

            let indices: Vec<usize> = match indices {
                Indices::U16(indices) => indices.iter().map(|v| *v as usize).collect(),
                Indices::U32(indices) => indices.iter().map(|v| *v as usize).collect(),
            };

            let grouped_indices = indices
                .iter()
                .fold((vec![], vec![]), |(mut triangles, mut buffer), i| {
                    buffer.push(*i);
                    if buffer.len() == 3 {
                        triangles.push(buffer.clone());
                        buffer = vec![];
                        if mesh.primitive_topology() == PrimitiveTopology::TriangleStrip {
                            buffer.push(*i);
                        }
                    }
                    (triangles, buffer)
                })
                .0;

            if let VertexAttributeValues::Float3(positions) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
            {
                let mut graph = UnGraphMap::with_capacity(0, 0);

                let vertices: Vec<Vec3> = positions
                    .iter()
                    .map(|p| Vec3::from_slice_unaligned(p))
                    .collect();

                let mut graph_connections: HashMap<IVec3, HashSet<usize>> = HashMap::default();

                let triangles = grouped_indices
                    .iter()
                    .map(|indices| {
                        let a = vertices[indices[0]];
                        let b = vertices[indices[1]];
                        let c = vertices[indices[2]];
                        graph_connections
                            .entry(IVec3::new(
                                (a.x * 1000.0) as i32,
                                (a.y * 1000.0) as i32,
                                (a.z * 1000.0) as i32,
                            ))
                            .or_insert_with(|| HashSet::default())
                            .insert(indices[0]);
                        graph_connections
                            .entry(IVec3::new(
                                (b.x * 1000.0) as i32,
                                (b.y * 1000.0) as i32,
                                (b.z * 1000.0) as i32,
                            ))
                            .or_insert_with(|| HashSet::default())
                            .insert(indices[1]);
                        graph_connections
                            .entry(IVec3::new(
                                (c.x * 1000.0) as i32,
                                (c.y * 1000.0) as i32,
                                (c.z * 1000.0) as i32,
                            ))
                            .or_insert_with(|| HashSet::default())
                            .insert(indices[2]);

                        graph.add_edge(indices[0], indices[1], a.distance(b));
                        graph.add_edge(indices[0], indices[2], a.distance(c));
                        graph.add_edge(indices[1], indices[2], b.distance(c));
                        ((a, indices[0]), (b, indices[1]), (c, indices[2]))
                    })
                    .collect();
                if connected_components(&graph) > 1 {
                    for vertices in graph_connections.values() {
                        if vertices.len() > 1 {
                            vertices.iter().fold(None, |last, current| {
                                if let Some(last) = last {
                                    let edges = graph
                                        .edges(*current)
                                        .map(|(a, b, w)| (a, b, *w))
                                        .collect::<Vec<_>>();
                                    for (from, to, weight) in edges.into_iter() {
                                        if from == *current {
                                            graph.add_edge(last, to, weight);
                                        } else {
                                            graph.add_edge(last, from, weight);
                                        }
                                    }
                                }
                                Some(*current)
                            });
                        }
                    }
                }
                return Some(NavMesh {
                    triangles,
                    graph,
                    vertices,
                });
            }
            None
        }
        mesh_to_option(mesh).unwrap_or_default()
    }

    pub fn point_in_mesh(&self, point: Vec3) -> bool {
        self.point_to_triangle(point).is_some()
    }

    fn point_to_triangle(
        &self,
        point: Vec3,
    ) -> Option<&((Vec3, usize), (Vec3, usize), (Vec3, usize))> {
        self.triangles
            .iter()
            .filter(|(a, b, c)| point_in_triangle(&point, (&a.0, &b.0, &c.0)))
            .next()
    }

    pub fn path_from_to(&self, from: Vec3, to: Vec3) -> Vec<Vec3> {
        let mut path = self.path_option(from, to).unwrap_or_default();
        if path.len() > 1 {
            self.smoothen_path(from, &mut path);
            path.push(to);
        }
        path
    }

    fn path_option(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        info!("searching for path");
        let start = self.point_to_triangle(from)?;
        let end = self.point_to_triangle(to)?;
        let mut graph = self.graph.clone();
        graph.add_edge(usize::MAX, start.0 .1, from.distance(start.0 .0));
        graph.add_edge(usize::MAX, start.1 .1, from.distance(start.1 .0));
        graph.add_edge(usize::MAX, start.2 .1, from.distance(start.2 .0));
        graph.add_edge(usize::MAX - 1, end.0 .1, from.distance(end.0 .0));
        graph.add_edge(usize::MAX - 1, end.1 .1, from.distance(end.1 .0));
        graph.add_edge(usize::MAX - 1, end.2 .1, from.distance(end.2 .0));
        let path = petgraph::algo::astar(
            &graph,
            usize::MAX,
            |n| n == usize::MAX - 1,
            |e| *e.2,
            |n| {
                if n == usize::MAX - 1 {
                    0.0
                } else if n == usize::MAX {
                    from.distance(to)
                } else {
                    self.vertices[n].distance(to) * 0.8
                }
            },
        )?
        .1;
        let mut path: Vec<Vec3> = path
            .iter()
            .filter(|n| **n != usize::MAX && **n != usize::MAX - 1)
            .map(|n| self.vertices[*n])
            .collect();
        path.push(to);
        Some(path)
    }

    fn smoothen_path(&self, from: Vec3, path: &mut Vec<Vec3>) {
        // eprintln!("{:?}", path);
        let mut new_path = vec![];
        let mut current = from;
        let mut last = from;
        let mut diff = 0;
        for point in path.iter() {
            diff += 1;
            let delta = *point - current;
            let mut is_in = true;
            // Check that line is in mesh
            // println!("    {:?}   ->    {:?}", current, point);
            for i in 1..(diff * 10) {
                let to_check = current + delta * (i as f32 / (diff as f32 * 10.0));
                // println!(
                //     "           {:?} - {:?}",
                //     to_check,
                //     self.point_in_mesh(to_check)
                // );
                if !self.point_in_mesh(to_check) {
                    is_in = false;
                    break;
                }
            }
            if !is_in {
                // eprintln!("+ {:?}", point);
                new_path.push(last);
                current = last;
                diff = 0;
            } else {
                // eprintln!("--- {:?}", point);
            }
            last = *point;
        }

        *path = new_path;
    }
}

fn point_in_triangle(point: &Vec3, (a, b, c): (&Vec3, &Vec3, &Vec3)) -> bool {
    let a = *a - *point;
    let b = *b - *point;
    let c = *c - *point;

    let u = b.cross(c);
    let v = c.cross(a);
    let w = a.cross(b);

    if u.dot(v) < 0.0 {
        false
    } else if u.dot(w) < 0.0 {
        false
    } else {
        true
    }
}

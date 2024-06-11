#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs
)]

use std::sync::Arc;

use bevy::math::Vec3Swizzles;
use bevy::reflect::TypePath;
use bevy::render::mesh::{MeshVertexAttributeId, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use itertools::Itertools;

pub mod asset_loaders;
mod obstacles;
pub mod updater;

pub use obstacles::{primitive::PrimitiveObstacle, ObstacleSource};

/// Bevy plugin to add support for the [`NavMesh`] asset type.
#[derive(Debug, Clone, Copy)]
pub struct VleueNavigatorPlugin;

impl Plugin for VleueNavigatorPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(asset_loaders::NavMeshPolyanyaLoader)
            .init_asset::<NavMesh>();
    }
}

/// A path between two points, in 3 dimensions using [`NavMesh::transform`].
#[derive(Debug, PartialEq)]
pub struct TransformedPath {
    /// Length of the path.
    pub length: f32,
    /// Coordinates for each step of the path. The destination is the last step.
    pub path: Vec<Vec3>,
}

use polyanya::Trimesh;
pub use polyanya::{Path, Triangulation};

/// A navigation mesh
#[derive(Debug, TypePath, Clone, Asset)]
pub struct NavMesh {
    mesh: Arc<polyanya::Mesh>,
    transform: Transform,
}

impl NavMesh {
    /// Builds a [`NavMesh`] from a Polyanya [`Mesh`](polyanya::Mesh)
    pub fn from_polyanya_mesh(mesh: polyanya::Mesh) -> NavMesh {
        NavMesh {
            mesh: Arc::new(mesh),
            transform: Transform::IDENTITY,
        }
    }

    /// Creates a [`NavMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    /// The [`polyanya::Mesh`] generated in the process can be modified via `callback`.
    ///
    /// Only supports meshes with the [`PrimitiveTopology::TriangleList`].
    pub fn from_bevy_mesh_and_then(mesh: &Mesh, callback: impl Fn(&mut polyanya::Mesh)) -> NavMesh {
        let normal = get_vectors(mesh, Mesh::ATTRIBUTE_NORMAL).next().unwrap();
        let rotation = Quat::from_rotation_arc(normal, Vec3::Z);

        let vertices = get_vectors(mesh, Mesh::ATTRIBUTE_POSITION)
            .map(|vertex| rotation.mul_vec3(vertex))
            .map(|coords| coords.xy())
            .collect();

        let triangles = mesh
            .indices()
            .expect("No polygon indices found in mesh")
            .iter()
            .tuples::<(_, _, _)>()
            .map(|(a, b, c)| [a, b, c])
            .collect();

        let mut polyanya_mesh = Trimesh {
            vertices,
            triangles,
        }
        .try_into()
        .unwrap();
        callback(&mut polyanya_mesh);

        let mut navmesh = Self::from_polyanya_mesh(polyanya_mesh);
        navmesh.transform = Transform::from_rotation(rotation);
        navmesh
    }

    /// Creates a [`NavMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    ///
    /// Only supports meshes with the [`PrimitiveTopology::TriangleList`].
    pub fn from_bevy_mesh(mesh: &Mesh) -> NavMesh {
        Self::from_bevy_mesh_and_then(mesh, |_| {})
    }

    /// Build a navmesh from its edges and obstacles.
    ///
    /// Obstacles will be merged in case some are overlapping, and mesh will be simplified to reduce the number of polygons.
    ///
    /// If you want more controls over the simplification process, you can use the [`from_polyanya_mesh`] method.
    ///
    /// Depending on the scale of your mesh, you should change the [`delta`](polyanya::Mesh::delta) value using [`set_delta`].
    pub fn from_edge_and_obstacles(edges: Vec<Vec2>, obstacles: Vec<Vec<Vec2>>) -> NavMesh {
        let mut triangulation = Triangulation::from_outer_edges(&edges);
        for obstacle in obstacles {
            triangulation.add_obstacle(obstacle);
        }

        triangulation.merge_overlapping_obstacles();

        let mut mesh: polyanya::Mesh = triangulation.as_navmesh().unwrap();
        triangulation.simplify(0.001);
        for _i in 0..3 {
            if mesh.merge_polygons() {
                break;
            }
        }
        mesh.set_delta(0.01);

        Self::from_polyanya_mesh(mesh)
    }

    /// Get the underlying Polyanya navigation mesh
    pub fn get(&self) -> Arc<polyanya::Mesh> {
        self.mesh.clone()
    }

    /// Set the [`delta`](polyanya::Mesh::delta) value of the navmesh.
    pub fn set_delta(&mut self, delta: f32) -> bool {
        if let Some(mesh) = Arc::get_mut(&mut self.mesh) {
            debug!("setting mesh delta to {}", delta);
            mesh.set_delta(delta);
            true
        } else {
            warn!("failed setting mesh delta to {}", delta);
            false
        }
    }

    /// Get the [`delta`](polyanya::Mesh::delta) value of the navmesh.
    pub fn delta(&self) -> f32 {
        self.mesh.delta()
    }

    /// Get a path between two points, in an async way
    #[inline]
    pub async fn get_path(&self, from: Vec2, to: Vec2) -> Option<Path> {
        self.mesh.get_path(from, to).await
    }

    /// Get a path between two points, in an async way.
    ///
    /// Inputs and results are transformed using the [`NavMesh::transform`]
    pub async fn get_transformed_path(&self, from: Vec3, to: Vec3) -> Option<TransformedPath> {
        let inner_from = self.transform.transform_point(from).xy();
        let inner_to = self.transform.transform_point(to).xy();
        let path = self.mesh.get_path(inner_from, inner_to).await;
        path.map(|path| self.transform_path(path, from, to))
    }

    /// Get a path between two points
    #[inline]
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<Path> {
        self.mesh.path(from, to)
    }

    /// Get a path between two points, in an async way.
    ///
    /// Inputs and results are transformed using the [`NavMesh::transform`]
    pub fn transformed_path(&self, from: Vec3, to: Vec3) -> Option<TransformedPath> {
        let inner_from = self.transform.transform_point(from).xy();
        let inner_to = self.transform.transform_point(to).xy();
        let path = self.mesh.path(inner_from, inner_to);
        path.map(|path| self.transform_path(path, from, to))
    }

    fn transform_path(&self, path: Path, from: Vec3, to: Vec3) -> TransformedPath {
        let inverse_transform = self.inverse_transform();
        TransformedPath {
            length: from.distance(to),
            path: path
                .path
                .into_iter()
                .map(|coords| inverse_transform.transform_point((coords, 0.).into()))
                .collect(),
        }
    }

    /// Check if a 3d point is in a navigationable part of the mesh, using the [`Mesh::transform`]
    pub fn transformed_is_in_mesh(&self, point: Vec3) -> bool {
        let point = self.transform.transform_point(point).xy();
        self.mesh.point_in_mesh(point)
    }

    /// Check if a point is in a navigationable part of the mesh
    pub fn is_in_mesh(&self, point: Vec2) -> bool {
        self.mesh.point_in_mesh(point)
    }

    /// The transform used to convert world coordinates into mesh coordinates.
    /// After applying this transform, the `z` coordinate is dropped because navmeshes are 2D.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Set the mesh transform
    ///
    /// It will be used to transform a 3d point to a 2d point where the `z` axis can be ignored
    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
    }

    /// Creates a [`Mesh`] from this [`NavMesh`], suitable for rendering the surface
    pub fn to_mesh(&self) -> Mesh {
        let mut new_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        let inverse_transform = self.inverse_transform();
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.mesh
                .vertices
                .iter()
                .map(|v| [v.coords.x, v.coords.y, 0.0])
                .map(|coords| inverse_transform.transform_point(coords.into()).into())
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.insert_indices(Indices::U32(
            self.mesh
                .polygons
                .iter()
                .flat_map(|p| {
                    (2..p.vertices.len())
                        .flat_map(|i| [p.vertices[0], p.vertices[i - 1], p.vertices[i]])
                })
                .collect(),
        ));
        new_mesh
    }

    /// Creates a [`Mesh`] from this [`NavMesh`], showing the wireframe of the polygons
    pub fn to_wireframe_mesh(&self) -> Mesh {
        let mut new_mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::all());
        let inverse_transform = self.inverse_transform();
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.mesh
                .vertices
                .iter()
                .map(|v| [v.coords.x, v.coords.y, 0.0])
                .map(|coords| inverse_transform.transform_point(coords.into()).into())
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.insert_indices(Indices::U32(
            self.mesh
                .polygons
                .iter()
                .flat_map(|p| {
                    (0..p.vertices.len())
                        .map(|i| [p.vertices[i], p.vertices[(i + 1) % p.vertices.len()]])
                })
                .unique_by(|[a, b]| if a < b { (*a, *b) } else { (*b, *a) })
                .flatten()
                .collect(),
        ));
        new_mesh
    }

    #[inline]
    fn inverse_transform(&self) -> Transform {
        Transform {
            translation: -self.transform.translation,
            rotation: self.transform.rotation.inverse(),
            scale: 1.0 / self.transform.scale,
        }
    }
}

fn get_vectors(
    mesh: &Mesh,
    id: impl Into<MeshVertexAttributeId>,
) -> impl Iterator<Item = Vec3> + '_ {
    let vectors = match mesh.attribute(id).unwrap() {
        VertexAttributeValues::Float32x3(values) => values,
        // Guaranteed by Bevy
        _ => unreachable!(),
    };
    vectors.iter().cloned().map(Vec3::from)
}

#[cfg(test)]
mod tests {
    use polyanya::Trimesh;

    use super::*;

    #[test]
    fn generating_from_existing_navmesh_results_in_same_navmesh() {
        let expected_navmesh = NavMesh::from_polyanya_mesh(
            Trimesh {
                vertices: vec![
                    Vec2::new(1., 1.),
                    Vec2::new(5., 1.),
                    Vec2::new(5., 4.),
                    Vec2::new(1., 4.),
                    Vec2::new(2., 2.),
                    Vec2::new(4., 3.),
                ],
                triangles: vec![[0, 1, 4], [1, 2, 5], [5, 2, 3], [1, 5, 3], [0, 4, 3]],
            }
            .try_into()
            .unwrap(),
        );
        let mut bevy_mesh = expected_navmesh.to_mesh();
        // Add back normals as they are used to determine where is up in the mesh
        bevy_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            (0..6).map(|_| [0.0, 0.0, 1.0]).collect::<Vec<_>>(),
        );
        let actual_navmesh = NavMesh::from_bevy_mesh(&bevy_mesh);

        assert_same_navmesh(expected_navmesh, actual_navmesh);
    }

    #[test]
    fn rotated_mesh_generates_expected_navmesh() {
        let expected_navmesh = NavMesh::from_polyanya_mesh(
            Trimesh {
                vertices: vec![
                    Vec2::new(-1., -1.),
                    Vec2::new(1., -1.),
                    Vec2::new(-1., 1.),
                    Vec2::new(1., 1.),
                ],
                triangles: vec![[0, 1, 3], [0, 3, 2]],
            }
            .try_into()
            .unwrap(),
        );
        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        bevy_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-1.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, -1.0],
                [1.0, 0.0, -1.0],
            ],
        );
        bevy_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![
                [0.0, 1.0, -0.0],
                [0.0, 1.0, -0.0],
                [0.0, 1.0, -0.0],
                [0.0, 1.0, -0.0],
            ],
        );
        bevy_mesh.insert_indices(Indices::U32(vec![0, 1, 3, 0, 3, 2]));

        let actual_navmesh = NavMesh::from_bevy_mesh(&bevy_mesh);

        assert_same_navmesh(expected_navmesh, actual_navmesh);
    }

    fn assert_same_navmesh(expected: NavMesh, actual: NavMesh) {
        let expected_mesh = expected.mesh;
        let actual_mesh = actual.mesh;

        assert_eq!(expected_mesh.polygons, actual_mesh.polygons);
        for (index, (expected_vertex, actual_vertex)) in expected_mesh
            .vertices
            .iter()
            .zip(actual_mesh.vertices.iter())
            .enumerate()
        {
            let nearly_same_coords =
                (expected_vertex.coords - actual_vertex.coords).length_squared() < 1e-8;
            assert!(nearly_same_coords
               ,
                "\nvertex {index} does not have the expected coords.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                expected_mesh.vertices, actual_mesh.vertices
            );

            let adjusted_actual = wrap_to_first(&actual_vertex.polygons, |index| *index != -1).unwrap_or_else(||
                panic!("vertex {index}: Found only surrounded by obstacles.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                       expected_mesh.vertices, actual_mesh.vertices));

            let adjusted_expectation= wrap_to_first(&expected_vertex.polygons, |polygon| {
                *polygon == adjusted_actual[0]
            })
                .unwrap_or_else(||
                    panic!("vertex {index}: Failed to expected polygons.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                           expected_mesh.vertices, actual_mesh.vertices));

            assert_eq!(
                adjusted_expectation, adjusted_actual,
                "\nvertex {index} does not have the expected polygons.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                expected_mesh.vertices, actual_mesh.vertices
            );
        }
    }

    fn wrap_to_first(polygons: &[isize], pred: impl Fn(&isize) -> bool) -> Option<Vec<isize>> {
        let offset = polygons.iter().position(pred)?;
        Some(
            polygons
                .iter()
                .skip(offset)
                .chain(polygons.iter().take(offset))
                .cloned()
                .collect(),
        )
    }
}

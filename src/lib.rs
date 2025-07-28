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
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::sync::Arc;

#[cfg(feature = "debug-with-gizmos")]
use bevy::{
    app::Update,
    asset::Assets,
    color::Color,
    prelude::{Component, Gizmos, Query, Res, Resource},
};
use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp},
    log::{debug, warn},
    math::{Affine3A, Quat, Vec2, Vec3, Vec3Swizzles},
    prelude::{Mesh, Transform, TransformPoint},
    reflect::TypePath,
    render::{
        mesh::{Indices, MeshVertexAttributeId, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};
use itertools::Itertools;

pub mod asset_loaders;
mod obstacles;
mod updater;

/// Prelude for imports
pub mod prelude {
    #[cfg(feature = "parry2d")]
    pub use crate::obstacles::parry2d::shape::SharedShapeStorage;
    pub use crate::obstacles::{
        ObstacleSource, cached::CachedObstacle, primitive::PrimitiveObstacle,
    };
    pub use crate::updater::{
        CachableObstacle, FilterObstaclesMode, ManagedNavMesh, NAVMESH_BUILD_DURATION,
        NavMeshSettings, NavMeshStatus, NavMeshUpdateMode, NavMeshUpdateModeBlocking,
        NavmeshUpdaterPlugin,
    };
    pub use crate::{NavMesh, Triangulation, VleueNavigatorPlugin};
    #[cfg(feature = "debug-with-gizmos")]
    pub use crate::{NavMeshDebug, NavMeshesDebug};
}

/// Bevy plugin to add support for the [`NavMesh`] asset type.
///
/// This plugin doesn't add support for updating them. See [`NavmeshUpdaterPlugin`] for that.
#[derive(Debug, Clone, Copy)]
pub struct VleueNavigatorPlugin;

/// Controls wether to display all [`NavMesh`]es with gizmos, and the color used.
///
/// When this resource is present, all [`NavMesh`]es will be visible.
#[cfg(feature = "debug-with-gizmos")]
#[derive(Resource, Clone, Copy, Debug)]
pub struct NavMeshesDebug(
    /// Color to display the [`NavMesh`] with
    pub Color,
);

/// Controls wether to display a specific [`NavMesh`] with gizmos, and the color used.
///
/// When this component is present on an entity, the [`NavMesh`] will be visible.
#[cfg(feature = "debug-with-gizmos")]
#[derive(Component, Clone, Copy, Debug)]
pub struct NavMeshDebug(
    /// Color to display the [`NavMesh`] with
    pub Color,
);

impl Plugin for VleueNavigatorPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(asset_loaders::NavMeshPolyanyaLoader)
            .init_asset::<NavMesh>();

        #[cfg(feature = "debug-with-gizmos")]
        app.add_systems(Update, display_navmesh);
    }
}

/// A path between two points, in 3D space, transformed using [`NavMesh::transform`].
#[derive(Debug, PartialEq)]
pub struct TransformedPath {
    /// Length of the path.
    pub length: f32,
    /// Coordinates for each step of the path. The destination is the last step.
    pub path: Vec<Vec3>,
    /// Coordinates for each step of the path. The destination is the last step.
    /// This path also contains the layer of each step, and steps when changing layers even on a straight line.
    #[cfg(feature = "detailed-layers")]
    #[cfg_attr(docsrs, doc(cfg(feature = "detailed-layers")))]
    pub path_with_layers: Vec<(Vec3, u8)>,
}

use polyanya::Trimesh;
pub use polyanya::{Path, Triangulation};

#[derive(Debug, Clone)]
pub(crate) struct BuildingMesh {
    pub(crate) mesh: polyanya::Mesh,
    pub(crate) failed_stitches: Vec<(u8, u8)>,
}

/// A navigation mesh
#[derive(Debug, TypePath, Clone, Asset)]
pub struct NavMesh {
    mesh: Arc<polyanya::Mesh>,
    building: Option<BuildingMesh>,
    transform: Transform,
}

impl NavMesh {
    /// Builds a [`NavMesh`] from a Polyanya [`Mesh`](polyanya::Mesh)
    pub fn from_polyanya_mesh(mesh: polyanya::Mesh) -> NavMesh {
        NavMesh {
            mesh: Arc::new(mesh),
            building: None,
            transform: Transform::IDENTITY,
        }
    }

    /// Creates a [`NavMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    /// The [`polyanya::Mesh`] generated in the process can be modified via `callback`.
    ///
    /// Only supports meshes with the [`PrimitiveTopology::TriangleList`].
    pub fn from_bevy_mesh_and_then(
        mesh: &Mesh,
        callback: impl Fn(&mut polyanya::Mesh),
    ) -> Option<NavMesh> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return None;
        }
        let normal = get_vectors(mesh, Mesh::ATTRIBUTE_NORMAL)
            .and_then(|mut i| i.next())
            .unwrap_or(Vec3::Z);
        let rotation = Quat::from_rotation_arc(normal, Vec3::Z);
        let rotation_reverse = rotation.inverse();

        let vertices = get_vectors(mesh, Mesh::ATTRIBUTE_POSITION)
            .expect("can't extract a navmesh from a mesh without `Mesh::ATTRIBUTE_POSITION`")
            .map(|vertex| rotation_reverse.mul_vec3(vertex))
            .map(|coords| coords.xy())
            .collect();

        let triangles = mesh
            .indices()
            .expect("No polygon indices found in mesh")
            .iter()
            .tuples::<(_, _, _)>()
            .map(|(a, b, c)| [c, b, a])
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
        Some(navmesh)
    }

    /// Creates a [`NavMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    ///
    /// Only supports meshes with the [`PrimitiveTopology::TriangleList`].
    pub fn from_bevy_mesh(mesh: &Mesh) -> Option<NavMesh> {
        Self::from_bevy_mesh_and_then(mesh, |_| {})
    }

    /// Builds a navmesh from its edges and obstacles.
    ///
    /// Obstacles will be merged in case some are overlapping, and mesh will be simplified to reduce the number of polygons.
    ///
    /// If you want more controls over the simplification process, you can use the [`Self::from_polyanya_mesh`] method.
    ///
    /// Depending on the scale of your mesh, you should change the [`Self::search_delta`] value using [`Self::set_search_delta`].
    pub fn from_edge_and_obstacles(edges: Vec<Vec2>, obstacles: Vec<Vec<Vec2>>) -> NavMesh {
        let mut triangulation = Triangulation::from_outer_edges(&edges);
        triangulation.add_obstacles(obstacles);

        let mut mesh: polyanya::Mesh = triangulation.as_navmesh();
        triangulation.simplify(0.001);
        for _i in 0..3 {
            if mesh.merge_polygons() {
                break;
            }
        }
        mesh.set_search_delta(0.01);

        Self::from_polyanya_mesh(mesh)
    }

    /// Retrieves the underlying Polyanya navigation mesh.
    pub fn get(&self) -> Arc<polyanya::Mesh> {
        self.mesh.clone()
    }

    /// Sets the [`search_delta`](polyanya::Mesh::search_delta) value of the navmesh.
    ///
    /// Returns `true` if the delta was successfully set, `false` otherwise. This can happens if the mesh is shared and already being modified.
    pub fn set_search_delta(&mut self, delta: f32) -> bool {
        if let Some(mesh) = Arc::get_mut(&mut self.mesh) {
            debug!("setting mesh delta to {}", delta);
            mesh.set_search_delta(delta);
            true
        } else {
            warn!("failed setting mesh delta to {}", delta);
            false
        }
    }

    /// Retrieves the [`search_delta`](polyanya::Mesh::search_delta) value of the navmesh.
    pub fn search_delta(&self) -> f32 {
        self.mesh.search_delta()
    }

    /// Sets the [`search_steps`](polyanya::Mesh::search_steps) value of the navmesh.
    ///
    /// Returns `true` if the steps value was successfully set, `false` otherwise. This can happens if the mesh is shared and already being modified.
    pub fn set_search_steps(&mut self, steps: u32) -> bool {
        if let Some(mesh) = Arc::get_mut(&mut self.mesh) {
            debug!("setting mesh steps to {}", steps);
            mesh.set_search_steps(steps);
            true
        } else {
            warn!("failed setting mesh steps to {}", steps);
            false
        }
    }

    /// Retrieves the [`search_steps`](polyanya::Mesh::search_steps) value of the navmesh.
    pub fn search_steps(&self) -> u32 {
        self.mesh.search_steps()
    }

    /// Asynchronously finds the shortest path between two points.
    #[inline]
    pub async fn get_path(&self, from: Vec2, to: Vec2) -> Option<Path> {
        self.mesh.get_path(from, to).await
    }

    /// Asynchronously finds the shortest path between two points.
    ///
    /// Inputs and results are transformed using the [`NavMesh::transform`].
    pub async fn get_transformed_path(&self, from: Vec3, to: Vec3) -> Option<TransformedPath> {
        let inner_from = self.world_to_mesh().transform_point(from).xy();
        let inner_to = self.world_to_mesh().transform_point(to).xy();
        let path = self.mesh.get_path(inner_from, inner_to).await;
        path.map(|path| self.transform_path(path))
    }

    /// Finds the shortest path between two points.
    #[inline]
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<Path> {
        self.mesh.path(from, to)
    }

    /// Finds the shortest path between two points.
    ///
    /// Inputs and results are transformed using the [`NavMesh::transform`]
    pub fn transformed_path(&self, from: Vec3, to: Vec3) -> Option<TransformedPath> {
        let inner_from = self.world_to_mesh().transform_point(from).xy();
        let inner_to = self.world_to_mesh().transform_point(to).xy();
        let path = self.mesh.path(inner_from, inner_to);
        path.map(|path| self.transform_path(path))
    }

    fn transform_path(&self, path: Path) -> TransformedPath {
        let transform = self.transform();
        TransformedPath {
            // TODO: recompute length
            length: path.length,
            path: path
                .path
                .into_iter()
                .map(|coords| transform.transform_point(coords.extend(0.0)))
                .collect(),
            #[cfg(feature = "detailed-layers")]
            path_with_layers: path
                .path_with_layers
                .into_iter()
                .map(|(coords, layer)| (transform.transform_point(coords.extend(0.0)), layer))
                .collect(),
        }
    }

    /// Checks if a 3D point is within a navigable part of the mesh, using the [`NavMesh::transform`].
    pub fn transformed_is_in_mesh(&self, point: Vec3) -> bool {
        let point_in_navmesh = self.world_to_mesh().transform_point(point).xy();
        self.mesh.point_in_mesh(point_in_navmesh)
    }

    /// Checks if a point is within a navigable part of the mesh.
    pub fn is_in_mesh(&self, point: Vec2) -> bool {
        self.mesh.point_in_mesh(point)
    }

    /// Retrieves the transform used to convert world coordinates into mesh coordinates.
    ///
    /// After applying this transform, the `z` coordinate is dropped because navmeshes are in 2D space.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Sets the mesh transform.
    ///
    /// It will be used to transform a point in 3D space to a point in the NavMesh 2D space by rotating it and ignoring the `z` axis.
    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
    }

    /// Creates a [`Mesh`] from this [`NavMesh`], suitable for debugging the surface.
    ///
    /// This mesh doesn't have normals.
    pub fn to_mesh(&self) -> Mesh {
        let mut new_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        let mesh_to_world = self.transform();
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.mesh.layers[0]
                .vertices
                .iter()
                .map(|v| v.coords.extend(0.0))
                .map(|coords| mesh_to_world.transform_point(coords).into())
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.insert_indices(Indices::U32(
            self.mesh.layers[0]
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

    /// Creates a [`Mesh`] from this [`NavMesh`], showing the wireframe of the polygons.
    pub fn to_wireframe_mesh(&self) -> Mesh {
        let mut new_mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::all());
        let mesh_to_world = self.transform();
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.mesh.layers[0]
                .vertices
                .iter()
                .map(|v| [v.coords.x, v.coords.y, 0.0])
                .map(|coords| mesh_to_world.transform_point(coords.into()).into())
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.insert_indices(Indices::U32(
            self.mesh.layers[0]
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

    /// Returns the transform that would convert world coordinates into mesh coordinates.
    #[inline]
    pub fn world_to_mesh(&self) -> Affine3A {
        world_to_mesh(&self.transform())
    }
}

pub(crate) fn world_to_mesh(navmesh_transform: &Transform) -> Affine3A {
    navmesh_transform.compute_affine().inverse()
}

fn get_vectors(
    mesh: &Mesh,
    id: impl Into<MeshVertexAttributeId>,
) -> Option<impl Iterator<Item = Vec3> + '_> {
    let vectors = match mesh.attribute(id) {
        Some(VertexAttributeValues::Float32x3(values)) => values,
        // Guaranteed by Bevy for the attributes requested in this context
        _ => return None,
    };
    Some(vectors.iter().cloned().map(Vec3::from))
}

#[cfg(feature = "debug-with-gizmos")]
/// System displaying navmeshes using gizmos for debug purposes.
pub fn display_navmesh(
    live_navmeshes: Query<(
        &updater::ManagedNavMesh,
        Option<&NavMeshDebug>,
        &bevy::prelude::GlobalTransform,
        &updater::NavMeshSettings,
    )>,
    mut gizmos: Gizmos,
    navmeshes: Res<Assets<NavMesh>>,
    controls: Option<Res<NavMeshesDebug>>,
) {
    for (mesh, debug, mesh_to_world, settings) in &live_navmeshes {
        let Some(color) = debug
            .map(|debug| debug.0)
            .or_else(|| controls.as_ref().map(|c| c.0))
        else {
            continue;
        };
        if let Some(navmesh) = navmeshes.get(mesh) {
            let navmesh = navmesh.get();
            let Some(layer) = &navmesh.layers.get(settings.layer.unwrap_or(0) as usize) else {
                continue;
            };
            #[cfg(feature = "detailed-layers")]
            let scale = layer.scale;
            #[cfg(not(feature = "detailed-layers"))]
            let scale = Vec2::ONE;
            for polygon in &layer.polygons {
                let mut v = polygon
                    .vertices
                    .iter()
                    .filter(|i| **i != u32::MAX)
                    .map(|i| layer.vertices[*i as usize].coords * scale)
                    .map(|v| mesh_to_world.transform_point(v.extend(0.0)))
                    .collect::<Vec<_>>();
                if !v.is_empty() {
                    let first = polygon.vertices[0];
                    let first = &layer.vertices[first as usize];
                    v.push(mesh_to_world.transform_point((first.coords * scale).extend(0.0)));
                    gizmos.linestrip(v, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use polyanya::Trimesh;

    use super::*;

    #[test]
    fn generating_from_existing_navmesh_results_in_same_navmesh() {
        // TODO: try and find why this is in CW instead of CCW
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
                triangles: vec![[4, 1, 0], [5, 2, 1], [3, 2, 5], [3, 5, 1], [3, 4, 0]],
            }
            .try_into()
            .unwrap(),
        );
        let initial_navmesh = NavMesh::from_polyanya_mesh(
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
        let mut bevy_mesh = initial_navmesh.to_mesh();
        // Add back normals as they are used to determine where is up in the mesh
        bevy_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            (0..6).map(|_| [0.0, 0.0, 1.0]).collect::<Vec<_>>(),
        );
        let actual_navmesh = NavMesh::from_bevy_mesh(&bevy_mesh).unwrap();

        assert_same_navmesh(expected_navmesh, actual_navmesh);
    }

    #[test]
    fn rotated_mesh_generates_expected_navmesh() {
        let expected_navmesh = NavMesh::from_polyanya_mesh(
            Trimesh {
                vertices: vec![
                    Vec2::new(-1., 1.),
                    Vec2::new(1., 1.),
                    Vec2::new(-1., -1.),
                    Vec2::new(1., -1.),
                ],
                triangles: vec![[3, 1, 0], [2, 3, 0]],
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

        let actual_navmesh = NavMesh::from_bevy_mesh(&bevy_mesh).unwrap();

        assert_same_navmesh(expected_navmesh, actual_navmesh);
    }

    fn assert_same_navmesh(expected: NavMesh, actual: NavMesh) {
        let expected_mesh = expected.mesh;
        let actual_mesh = actual.mesh;

        for i in 0..expected_mesh.layers.len() {
            assert_eq!(
                expected_mesh.layers[i].polygons,
                actual_mesh.layers[i].polygons
            );
            for (index, (expected_vertex, actual_vertex)) in expected_mesh.layers[i]
                .vertices
                .iter()
                .zip(actual_mesh.layers[i].vertices.iter())
                .enumerate()
            {
                let nearly_same_coords =
                    (expected_vertex.coords - actual_vertex.coords).length_squared() < 1e-8;
                assert!(
                    nearly_same_coords,
                    "\nvertex {index} does not have the expected coords.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                    expected_mesh.layers[i].vertices, actual_mesh.layers[i].vertices
                );

                let adjusted_actual = wrap_to_first(&actual_vertex.polygons, |index| *index != u32::MAX).unwrap_or_else(||
                panic!("vertex {index}: Found only surrounded by obstacles.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                       expected_mesh.layers[i].vertices, actual_mesh.layers[i].vertices));

                let adjusted_expectation= wrap_to_first(&expected_vertex.polygons, |polygon| {
                *polygon == adjusted_actual[0]
            })
                .unwrap_or_else(||
                    panic!("vertex {index}: Failed to expected polygons.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                           expected_mesh.layers[i].vertices, actual_mesh.layers[i].vertices));

                assert_eq!(
                    adjusted_expectation, adjusted_actual,
                    "\nvertex {index} does not have the expected polygons.\nExpected vertices: {0:?}\nGot vertices: {1:?}",
                    expected_mesh.layers[i].vertices, actual_mesh.layers[i].vertices
                );
            }
        }
    }

    fn wrap_to_first(polygons: &[u32], pred: impl Fn(&u32) -> bool) -> Option<Vec<u32>> {
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

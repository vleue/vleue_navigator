use std::sync::Arc;

use bevy::render::mesh::{MeshVertexAttributeId, VertexAttributeValues};
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    utils::BoxedFuture,
};
use itertools::Itertools;

pub struct PathMeshPlugin;

impl Plugin for PathMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<PathMesh>()
            .init_asset_loader::<PathMeshPolyanyaLoader>();
    }
}

#[derive(Debug, TypeUuid, Clone)]
#[uuid = "807C7A31-EA06-4A3B-821B-6E91ADB95734"]
pub struct PathMesh {
    mesh: Arc<polyanya::Mesh>,
}

impl PathMesh {
    pub fn from_polyanya_mesh(mut mesh: polyanya::Mesh) -> PathMesh {
        mesh.bake();
        PathMesh {
            mesh: Arc::new(mesh),
        }
    }

    /// Creates a [`PathMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    /// The [`polyanya::Mesh`] generated in the process can be modified via `callback`.
    ///
    /// Only supports triangle lists.
    pub fn from_bevy_mesh_and_then(
        mesh: &Mesh,
        callback: impl Fn(&mut polyanya::Mesh),
    ) -> PathMesh {
        let rotations = get_vectors(mesh, Mesh::ATTRIBUTE_NORMAL)
            .map(|normal| Quat::from_rotation_arc(normal, Vec3::Z));

        let vertices = get_vectors(mesh, Mesh::ATTRIBUTE_POSITION)
            .zip(rotations)
            .map(|(vertex, rotation)| rotation.mul_vec3(vertex))
            .map(|coords| Vec2::new(coords[0], coords[1]))
            .collect();

        let triangles = mesh
            .indices()
            .expect("No polygon indices found in mesh")
            .iter()
            .tuples::<(_, _, _)>()
            .map(polyanya::Triangle::from)
            .collect();

        let mut polyanya_mesh = polyanya::Mesh::from_trimesh(vertices, triangles);
        callback(&mut polyanya_mesh);

        Self::from_polyanya_mesh(polyanya_mesh)
    }

    /// Creates a [`PathMesh`] from a Bevy [`Mesh`], assuming it constructs a 2D structure.
    /// All triangle normals are aligned during the conversion, so the orientation of the [`Mesh`] does not matter.
    ///
    /// Only supports triangle lists.
    pub fn from_bevy_mesh(mesh: &Mesh) -> PathMesh {
        Self::from_bevy_mesh_and_then(mesh, |_| {})
    }

    pub fn get(&self) -> Arc<polyanya::Mesh> {
        self.mesh.clone()
    }

    #[inline]
    pub async fn get_path(&self, from: Vec2, to: Vec2) -> Option<polyanya::Path> {
        self.mesh.get_path(from, to).await
    }

    #[inline]
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<polyanya::Path> {
        self.mesh.path(from, to)
    }

    pub fn is_in_mesh(&self, point: Vec2) -> bool {
        self.mesh.point_in_mesh(point)
    }

    pub fn to_mesh(&self) -> Mesh {
        let mut new_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.mesh
                .vertices
                .iter()
                .map(|v| [v.coords.x, v.coords.y, 0.0])
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.set_indices(Some(Indices::U32(
            self.mesh
                .polygons
                .iter()
                .flat_map(|p| {
                    (2..p.vertices.len())
                        .flat_map(|i| [p.vertices[0], p.vertices[i - 1], p.vertices[i]])
                })
                .map(|v| v as u32)
                .collect(),
        )));
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            (0..self.mesh.vertices.len())
                .into_iter()
                .map(|_| [0.0, 0.0, 1.0])
                .collect::<Vec<[f32; 3]>>(),
        );
        new_mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            self.mesh
                .vertices
                .iter()
                .map(|v| [v.coords.x, v.coords.y])
                .collect::<Vec<[f32; 2]>>(),
        );
        new_mesh
    }
}

#[derive(Default)]
pub struct PathMeshPolyanyaLoader;

impl AssetLoader for PathMeshPolyanyaLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            load_context.set_default_asset(LoadedAsset::new(PathMesh {
                mesh: Arc::new(polyanya::Mesh::from_bytes(bytes)),
            }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["polyanya.mesh"]
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
    vectors.into_iter().cloned().map(Vec3::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generating_from_existing_path_mesh_results_in_same_path_mesh() {
        let expected_path_mesh = PathMesh::from_polyanya_mesh(polyanya::Mesh::from_trimesh(
            vec![
                Vec2::new(1., 1.),
                Vec2::new(5., 1.),
                Vec2::new(5., 4.),
                Vec2::new(1., 4.),
                Vec2::new(2., 2.),
                Vec2::new(4., 3.),
            ],
            vec![
                (0, 1, 4).into(),
                (1, 2, 5).into(),
                (5, 2, 3).into(),
                (1, 5, 3).into(),
                (0, 4, 3).into(),
            ],
        ));
        let bevy_mesh = expected_path_mesh.to_mesh();
        let actual_path_mesh = PathMesh::from_bevy_mesh(&bevy_mesh);

        assert_same_path_mesh(expected_path_mesh, actual_path_mesh);
    }

    #[test]
    fn rotated_mesh_generates_expected_path_mesh() {
        let expected_path_mesh = PathMesh::from_polyanya_mesh(polyanya::Mesh::from_trimesh(
            vec![
                Vec2::new(-1., -1.),
                Vec2::new(1., -1.),
                Vec2::new(-1., 1.),
                Vec2::new(1., 1.),
            ],
            vec![(0, 1, 3).into(), (0, 3, 2).into()],
        ));
        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);
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
        bevy_mesh.set_indices(Some(Indices::U32(vec![0, 1, 3, 0, 3, 2])));

        let actual_path_mesh = PathMesh::from_bevy_mesh(&bevy_mesh);

        assert_same_path_mesh(expected_path_mesh, actual_path_mesh);
    }

    fn assert_same_path_mesh(expected: PathMesh, actual: PathMesh) {
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

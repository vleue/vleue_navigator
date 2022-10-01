use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    utils::BoxedFuture,
};

pub struct PathmeshPlugin;

impl Plugin for PathmeshPlugin {
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
    pub fn from_polyanya_mesh(mesh: polyanya::Mesh) -> PathMesh {
        PathMesh {
            mesh: Arc::new(mesh),
        }
    }

impl BlockingPathMesh {
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

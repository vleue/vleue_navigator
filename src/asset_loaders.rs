//! Asset loaders that can load a [`PathMesh`] from a file

use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::{Transform, Vec3},
    utils::BoxedFuture,
};
use polyanya::PolyanyaFile;

use crate::PathMesh;

/// Asset loader for a mesh in the `mesh 2` format with a `.polyanya.mesh` extension.
///
/// See <https://github.com/vleue/polyanya/blob/main/meshes/format.txt> for format description.
#[derive(Default, Debug, Clone, Copy)]
pub struct PathMeshPolyanyaLoader;

impl AssetLoader for PathMeshPolyanyaLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            load_context.set_default_asset(LoadedAsset::new(PathMesh {
                mesh: Arc::new(PolyanyaFile::from_bytes(bytes).into()),
                transform: Transform::from_scale(Vec3::splat(1.)),
            }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["polyanya.mesh"]
    }
}

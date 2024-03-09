//! Asset loaders that can load a [`NavMesh`] from a file

use std::sync::Arc;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::{Transform, Vec3},
    utils::{thiserror, BoxedFuture},
};
use polyanya::PolyanyaFile;

use crate::NavMesh;

/// Error that can happen while reading a `NavMesh` from a file
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum NavMeshLoaderError {
    /// An [IO](std::io) Error
    #[error("Couldn't load polyanya file: {0}")]
    Io(#[from] std::io::Error),
}

/// Asset loader for a mesh in the `mesh 2` format with a `.polyanya.mesh` extension.
///
/// See <https://github.com/vleue/polyanya/blob/main/meshes/format.txt> for format description.
#[derive(Default, Debug, Clone, Copy)]
pub struct NavMeshPolyanyaLoader;

impl AssetLoader for NavMeshPolyanyaLoader {
    type Asset = NavMesh;
    type Settings = ();
    type Error = NavMeshLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let navmesh = NavMesh {
                mesh: Arc::new(PolyanyaFile::from_bytes(&bytes).into()),
                transform: Transform::from_scale(Vec3::splat(1.)),
            };
            Ok(navmesh)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["polyanya.mesh"]
    }
}

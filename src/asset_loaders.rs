//! Asset loaders that can load a [`PathMesh`] from a file

use std::{error::Error, fmt::Display, sync::Arc};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::{Transform, Vec3},
    utils::BoxedFuture,
};
use polyanya::PolyanyaFile;

use crate::PathMesh;

/// Error that can happen while reading a `PathMesh` from a file
#[derive(Debug)]
pub enum PathMeshLoaderError {
    /// Error when reading file
    Io(std::io::Error),
}

impl Display for PathMeshLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathMeshLoaderError::Io(io_error) => write!(f, "IO error: {}", io_error),
        }
    }
}

impl Error for PathMeshLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PathMeshLoaderError::Io(io_error) => Some(io_error),
        }
    }
}
/// Asset loader for a mesh in the `mesh 2` format with a `.polyanya.mesh` extension.
///
/// See <https://github.com/vleue/polyanya/blob/main/meshes/format.txt> for format description.
#[derive(Default, Debug, Clone, Copy)]
pub struct PathMeshPolyanyaLoader;

impl AssetLoader for PathMeshPolyanyaLoader {
    type Asset = PathMesh;
    type Settings = ();
    type Error = PathMeshLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(PathMeshLoaderError::Io)?;
            let pathmesh = PathMesh {
                mesh: Arc::new(PolyanyaFile::from_bytes(bytes.as_slice()).into()),
                transform: Transform::from_scale(Vec3::splat(1.)),
            };
            Ok(pathmesh)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["polyanya.mesh"]
    }
}

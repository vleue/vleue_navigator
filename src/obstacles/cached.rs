use std::sync::{Arc, OnceLock};

use bevy::{
    math::Vec2,
    prelude::Component,
    transform::components::{GlobalTransform, Transform},
};

use super::ObstacleSource;

/// An obstacle source that will cache the polygon from its source obstacle, so that it doesn't need to
/// be recomputed every time. To clear the cache, you can use [`clear`](CachedObstacle::clear).
#[derive(Clone, Component, Debug)]
pub struct CachedObstacle<T: ObstacleSource> {
    polygon: Arc<OnceLock<Vec<Vec2>>>,
    source: T,
}

impl<T: ObstacleSource> CachedObstacle<T> {
    /// Create a new cached obstacle from another obstacle source
    pub fn new(source: T) -> Self {
        Self {
            polygon: Arc::new(OnceLock::new()),
            source,
        }
    }

    /// Clear the cache for this obstacle
    pub fn clear(&mut self) {
        self.polygon = Arc::new(OnceLock::new());
    }
}

impl<T: ObstacleSource> ObstacleSource for CachedObstacle<T> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        self.polygon
            .get_or_init(|| {
                T::get_polygon(&self.source, obstacle_transform, navmesh_transform)
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .clone()
    }
}

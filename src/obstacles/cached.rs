use std::sync::{Arc, RwLock};

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
    polygon: Arc<RwLock<Option<Vec<Vec2>>>>,
    source: T,
}

impl<T: ObstacleSource> CachedObstacle<T> {
    /// Create a new cached obstacle from another obstacle source
    pub fn new(source: T) -> Self {
        Self {
            polygon: Arc::new(RwLock::new(None)),
            source,
        }
    }

    /// Clear the cache for this obstacle
    pub fn clear(&mut self) {
        self.polygon = Arc::new(RwLock::new(None));
    }
}

impl<T: ObstacleSource> ObstacleSource for CachedObstacle<T> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        if let Some(poly) = self.polygon.read().unwrap().as_ref() {
            return poly.clone();
        }
        let poly = T::get_polygon(&self.source, obstacle_transform, navmesh_transform);
        let mut writer = self.polygon.write().unwrap();
        *writer = Some(poly.clone());
        poly
    }
}

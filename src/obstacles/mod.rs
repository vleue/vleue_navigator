use bevy::{
    math::Vec2,
    prelude::Component,
    transform::components::{GlobalTransform, Transform},
};

mod aabb;
pub(crate) mod primitive;

/// Trait to mark a component as the source of position and shape of an obstacle.
pub trait ObstacleSource: Component + Clone {
    /// Get the polygon of the obstacle in the local space of the mesh.
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2>;
}

use bevy::{
    math::{vec3, Vec2, Vec3, Vec3Swizzles},
    render::primitives::Aabb,
    transform::components::{GlobalTransform, Transform},
};

use super::ObstacleSource;

impl ObstacleSource for Aabb {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();
        let to_vec2 = |v: Vec3| navmesh_transform.transform_point(v).xy();

        vec![
            to_vec2(transform.transform_point(vec3(
                -self.half_extents.x,
                self.half_extents.y,
                self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(
                -self.half_extents.x,
                -self.half_extents.y,
                -self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(
                self.half_extents.x,
                -self.half_extents.y,
                -self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(
                self.half_extents.x,
                self.half_extents.y,
                self.half_extents.z,
            ))),
        ]
    }
}

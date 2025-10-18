use bevy::{
    camera::primitives::Aabb,
    math::{Dir3, Quat, Vec2, Vec3, Vec3Swizzles, vec3},
    transform::components::{GlobalTransform, Transform},
};

use crate::world_to_mesh;

use super::ObstacleSource;

impl ObstacleSource for Aabb {
    fn get_polygons(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        (up, _shift): (Dir3, f32),
    ) -> Vec<Vec<Vec2>> {
        let transform = obstacle_transform.compute_transform();
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let to_world = |v: Vec3| {
            let v = v.xzy();
            let v = if up.is_negative_bitmask().count_ones() % 2 == 1 {
                Quat::from_rotation_arc(-Vec3::Y, up.into()).mul_vec3(v)
            } else {
                Quat::from_rotation_arc(Vec3::Y, up.into()).mul_vec3(v)
            };
            transform.transform_point(v)
        };
        let to_navmesh = |v: Vec3| world_to_mesh.transform_point3(v).xy();

        vec![vec![
            to_navmesh(to_world(vec3(
                -self.half_extents.x,
                self.half_extents.y,
                self.half_extents.z,
            ))),
            to_navmesh(to_world(vec3(
                -self.half_extents.x,
                -self.half_extents.y,
                -self.half_extents.z,
            ))),
            to_navmesh(to_world(vec3(
                self.half_extents.x,
                -self.half_extents.y,
                -self.half_extents.z,
            ))),
            to_navmesh(to_world(vec3(
                self.half_extents.x,
                self.half_extents.y,
                self.half_extents.z,
            ))),
        ]]
    }
}

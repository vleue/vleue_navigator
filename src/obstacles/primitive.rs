use std::f32::consts::PI;

use bevy::{
    math::{vec2, EulerRot, Rot2, Vec2, Vec3Swizzles},
    prelude::{
        Capsule2d, Circle, CircularSector, CircularSegment, Component, Ellipse, Rectangle,
        RegularPolygon, Rhombus,
    },
    transform::components::{GlobalTransform, Transform},
};

use super::ObstacleSource;

#[derive(Component, Debug, Clone, Copy)]
pub enum PrimitiveObstacle {
    Rectangle(Rectangle),
    Circle(Circle),
    Ellipse(Ellipse),
    CircularSector(CircularSector),
    CircularSegment(CircularSegment),
    Capsule(Capsule2d),
    RegularPolygon(RegularPolygon),
    Rhombus(Rhombus),
}

// Functions in this module are copied from Bevy
mod copypasta {
    use std::f32::consts::TAU;

    use bevy::math::Vec2;

    pub(crate) fn ellipse_inner(half_size: Vec2, resolution: usize) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let angle = i as f32 * TAU / resolution as f32;
            let (x, y) = angle.sin_cos();
            Vec2::new(x, y) * half_size
        })
    }

    pub(crate) fn arc_2d_inner(
        direction_angle: f32,
        arc_angle: f32,
        radius: f32,
        resolution: usize,
    ) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let start = direction_angle - arc_angle / 2.;

            let angle =
                start + (i as f32 * (arc_angle / resolution as f32)) + std::f32::consts::FRAC_PI_2;

            Vec2::new(angle.cos(), angle.sin()) * radius
        })
    }

    pub(crate) fn single_circle_coordinate(
        radius: f32,
        resolution: usize,
        nth_point: usize,
    ) -> Vec2 {
        let angle = nth_point as f32 * TAU / resolution as f32;
        let (x, y) = angle.sin_cos();
        Vec2::new(x, y) * radius
    }
}

trait TransformPoint2d {
    fn transform_point_2d(&self, point: Vec2) -> Vec2;
}

impl TransformPoint2d for Transform {
    fn transform_point_2d(&self, mut point: Vec2) -> Vec2 {
        point = self.scale.xy() * point;
        point = Rot2::radians(self.rotation.to_euler(EulerRot::XYZ).2) * point;
        point += self.translation.xy();
        point
    }
}

impl ObstacleSource for PrimitiveObstacle {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        _navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();

        match self {
            PrimitiveObstacle::Rectangle(primitive) => vec![
                transform.transform_point_2d(vec2(-primitive.half_size.x, -primitive.half_size.y)),
                transform.transform_point_2d(vec2(-primitive.half_size.x, primitive.half_size.y)),
                transform.transform_point_2d(vec2(primitive.half_size.x, primitive.half_size.y)),
                transform.transform_point_2d(vec2(primitive.half_size.x, -primitive.half_size.y)),
            ],
            PrimitiveObstacle::Circle(primitive) => {
                copypasta::ellipse_inner(vec2(primitive.radius, primitive.radius), 32)
                    .map(|v| transform.transform_point_2d(v))
                    .collect()
            }
            PrimitiveObstacle::Ellipse(primitive) => {
                copypasta::ellipse_inner(primitive.half_size, 32)
                    .map(|v| transform.transform_point_2d(v))
                    .collect()
            }
            PrimitiveObstacle::CircularSector(primitive) => {
                let mut arc =
                    copypasta::arc_2d_inner(0.0, primitive.arc.angle(), primitive.arc.radius, 32)
                        .map(|v| transform.transform_point_2d(v))
                        .collect::<Vec<_>>();
                arc.push(transform.translation.xy());
                arc
            }
            PrimitiveObstacle::CircularSegment(primitive) => {
                copypasta::arc_2d_inner(0.0, primitive.arc.angle(), primitive.arc.radius, 32)
                    .map(|v| transform.transform_point_2d(v))
                    .collect()
            }
            PrimitiveObstacle::Capsule(primitive) => {
                let mut points = copypasta::arc_2d_inner(0.0, PI, primitive.radius, 32)
                    .map(|v| transform.transform_point_2d(v + primitive.half_length * Vec2::Y))
                    .collect::<Vec<_>>();
                points.extend(
                    copypasta::arc_2d_inner(0.0, PI, primitive.radius, 32).map(|v| {
                        transform.transform_point_2d(
                            (Rot2::radians(PI) * v) - primitive.half_length * Vec2::Y,
                        )
                    }),
                );
                points
            }
            PrimitiveObstacle::RegularPolygon(primitive) => (0..=primitive.sides)
                .map(|p| {
                    copypasta::single_circle_coordinate(
                        primitive.circumcircle.radius,
                        primitive.sides,
                        p,
                    )
                })
                .map(|v| transform.transform_point_2d(v))
                .collect(),
            PrimitiveObstacle::Rhombus(primitive) => {
                [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)]
                    .map(|(sign_x, sign_y)| {
                        Vec2::new(
                            primitive.half_diagonals.x * sign_x,
                            primitive.half_diagonals.y * sign_y,
                        )
                    })
                    .into_iter()
                    .map(|v| transform.transform_point_2d(v))
                    .collect()
            }
        }
    }
}

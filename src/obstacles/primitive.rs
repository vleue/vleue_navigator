use bevy::{
    math::{vec2, Rot2, Vec2, Vec3, Vec3Swizzles},
    prelude::{
        Capsule2d, Circle, CircularSector, CircularSegment, Component, Ellipse, Rectangle,
        RegularPolygon, Rhombus,
    },
    transform::components::{GlobalTransform, Transform},
};

use super::{ObstacleSource, RESOLUTION};

/// A primitive obstacle that can be used to create a [`NavMesh`].
/// Variants are made from primitive shapes defined in Bevy
#[derive(Component, Debug, Clone, Copy)]
pub enum PrimitiveObstacle {
    /// A rectangle primitive.
    Rectangle(Rectangle),
    /// A circle primitive.
    Circle(Circle),
    /// An ellipse primitive
    Ellipse(Ellipse),
    /// A primitive representing a circular sector: a pie slice of a circle.
    CircularSector(CircularSector),
    /// A primitive representing a circular segment:
    /// the area enclosed by the arc of a circle and its chord (the line between its endpoints).
    CircularSegment(CircularSegment),
    /// A 2D capsule primitive, also known as a stadium or pill shape.
    Capsule(Capsule2d),
    /// A polygon where all vertices lie on a circle, equally far apart.
    RegularPolygon(RegularPolygon),
    /// A rhombus primitive, also known as a diamond shape.
    Rhombus(Rhombus),
}

// Functions in this module are copied from Bevy
mod copypasta {
    use std::f64::consts::TAU;

    use bevy::math::Vec2;

    pub(crate) fn ellipse_inner(half_size: Vec2, resolution: u32) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let angle = i as f64 * TAU / resolution as f64;
            let (x, y) = angle.sin_cos();
            Vec2::new(x as f32, y as f32) * half_size
        })
    }

    pub(crate) fn arc_2d_inner(
        direction_angle: f64,
        arc_angle: f64,
        radius: f32,
        resolution: u32,
    ) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let start = direction_angle - arc_angle / 2.;

            let angle =
                start + (i as f64 * (arc_angle / resolution as f64)) + std::f64::consts::FRAC_PI_2;

            Vec2::new(angle.cos() as f32, angle.sin() as f32) * radius
        })
    }

    pub(crate) fn single_circle_coordinate(radius: f32, resolution: u32, nth_point: usize) -> Vec2 {
        let angle = nth_point as f64 * TAU / resolution as f64;
        let (x, y) = angle.sin_cos();
        Vec2::new(x as f32, y as f32) * radius
    }
}

impl ObstacleSource for PrimitiveObstacle {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();
        let to_vec2 = |v: Vec3| v.xy();

        let to_navmesh_vec3 = |v: Vec3| {
            let v = navmesh_transform.rotation.mul_vec3(v);
            v
        };

        let to_navmesh = |v: Vec2| {
            let v = v.extend(0.0);
            let v = navmesh_transform.rotation.inverse().mul_vec3(v);
            let v = transform.transform_point(v);
            to_navmesh_vec3(v)
        };

        match self {
            PrimitiveObstacle::Rectangle(primitive) => vec![
                to_vec2(to_navmesh(vec2(
                    -primitive.half_size.x,
                    -primitive.half_size.y,
                ))),
                to_vec2(to_navmesh(vec2(
                    -primitive.half_size.x,
                    primitive.half_size.y,
                ))),
                to_vec2(to_navmesh(vec2(
                    primitive.half_size.x,
                    primitive.half_size.y,
                ))),
                to_vec2(to_navmesh(vec2(
                    primitive.half_size.x,
                    -primitive.half_size.y,
                ))),
            ],
            PrimitiveObstacle::Circle(primitive) => {
                copypasta::ellipse_inner(vec2(primitive.radius, primitive.radius), RESOLUTION)
                    .map(|v| to_vec2(to_navmesh(v)))
                    .collect()
            }
            PrimitiveObstacle::Ellipse(primitive) => {
                copypasta::ellipse_inner(primitive.half_size, RESOLUTION)
                    .map(|v| to_vec2(to_navmesh(v)))
                    .collect()
            }
            PrimitiveObstacle::CircularSector(primitive) => {
                let mut arc = copypasta::arc_2d_inner(
                    0.0,
                    primitive.arc.angle() as f64,
                    primitive.arc.radius,
                    RESOLUTION,
                )
                .map(|v| to_vec2(to_navmesh(v)))
                .collect::<Vec<_>>();
                arc.push(to_vec2(to_navmesh_vec3(transform.translation)));
                arc
            }
            PrimitiveObstacle::CircularSegment(primitive) => copypasta::arc_2d_inner(
                0.0,
                primitive.arc.angle() as f64,
                primitive.arc.radius,
                RESOLUTION,
            )
            .map(|v| to_vec2(to_navmesh(v)))
            .collect(),
            PrimitiveObstacle::Capsule(primitive) => {
                let mut points = copypasta::arc_2d_inner(
                    0.0,
                    std::f64::consts::PI,
                    primitive.radius,
                    RESOLUTION,
                )
                .map(|v| to_vec2(to_navmesh(v + primitive.half_length * Vec2::Y)))
                .collect::<Vec<_>>();
                points.extend(
                    copypasta::arc_2d_inner(
                        0.0,
                        std::f64::consts::PI,
                        primitive.radius,
                        RESOLUTION,
                    )
                    .map(|v| {
                        to_vec2(to_navmesh(
                            (Rot2::radians(std::f32::consts::PI) * v)
                                - primitive.half_length * Vec2::Y,
                        ))
                    }),
                );
                points
            }
            PrimitiveObstacle::RegularPolygon(primitive) => (0..=primitive.sides)
                .map(|p| {
                    copypasta::single_circle_coordinate(
                        primitive.circumcircle.radius,
                        primitive.sides as u32,
                        p,
                    )
                })
                .map(|v| to_vec2(to_navmesh(v)))
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
                    .map(|v| to_vec2(to_navmesh(v)))
                    .collect()
            }
        }
    }
}

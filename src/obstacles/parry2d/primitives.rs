use bevy::{math::bounding::Bounded2d, prelude::*};
use parry2d::{
    mass_properties::MassProperties,
    math::Isometry,
    query::{
        PointQuery, RayCast, details::local_ray_intersection_with_support_map_with_params,
        gjk::VoronoiSimplex, point::local_point_projection_on_support_map,
    },
    shape::*,
};

use nalgebra::{Point2, UnitVector2, Vector2};

use crate::obstacles::parry2d::math::{
    AdjustPrecision, AsF32, FRAC_PI_2, PI, Scalar, TAU, na_iso_to_iso,
};
/// An ellipse shape that can be stored in a [`SharedShape`] for an [`Ellipse`].
///
/// This wrapper is required to allow implementing the necessary traits from [`parry2d`]
/// for Bevy's [`Ellipse`] type.
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct EllipseShape(pub Ellipse);

impl SupportMap for EllipseShape {
    #[inline]
    fn local_support_point(&self, direction: &Vector2<Scalar>) -> Point2<Scalar> {
        let [a, b] = self.half_size.adjust_precision().to_array();
        let denom = (direction.x.powi(2) * a * a + direction.y.powi(2) * b * b).sqrt();
        Point2::new(a * a * direction.x / denom, b * b * direction.y / denom)
    }
}

impl Shape for EllipseShape {
    fn clone_dyn(&self) -> Box<dyn Shape> {
        Box::new(*self)
    }

    fn scale_dyn(
        &self,
        scale: &parry2d::math::Vector<Scalar>,
        _num_subdivisions: u32,
    ) -> Option<Box<dyn Shape>> {
        let half_size = Vec2::from(*scale).f32() * self.half_size;
        Some(Box::new(EllipseShape(Ellipse::new(
            half_size.x,
            half_size.y,
        ))))
    }

    fn compute_local_aabb(&self) -> parry2d::bounding_volume::Aabb {
        let aabb = self.aabb_2d(Isometry2d::IDENTITY);
        parry2d::bounding_volume::Aabb::new(
            aabb.min.adjust_precision().into(),
            aabb.max.adjust_precision().into(),
        )
    }

    fn compute_aabb(&self, position: &Isometry<Scalar>) -> parry2d::bounding_volume::Aabb {
        let isometry = na_iso_to_iso(position);
        let aabb = self.aabb_2d(isometry);
        parry2d::bounding_volume::Aabb::new(
            aabb.min.adjust_precision().into(),
            aabb.max.adjust_precision().into(),
        )
    }

    fn compute_local_bounding_sphere(&self) -> parry2d::bounding_volume::BoundingSphere {
        let sphere = self.bounding_circle(Isometry2d::IDENTITY);
        parry2d::bounding_volume::BoundingSphere::new(
            sphere.center.adjust_precision().into(),
            sphere.radius().adjust_precision(),
        )
    }

    fn compute_bounding_sphere(
        &self,
        position: &Isometry<Scalar>,
    ) -> parry2d::bounding_volume::BoundingSphere {
        let isometry = na_iso_to_iso(position);
        let sphere = self.bounding_circle(isometry);
        parry2d::bounding_volume::BoundingSphere::new(
            sphere.center.adjust_precision().into(),
            sphere.radius().adjust_precision(),
        )
    }

    fn clone_box(&self) -> Box<dyn Shape> {
        Box::new(*self)
    }

    fn mass_properties(&self, density: Scalar) -> MassProperties {
        let volume = self.area().adjust_precision();
        let mass = volume * density;
        let inertia = mass * self.half_size.length_squared().adjust_precision() / 4.0;
        MassProperties::new(Point2::origin(), mass, inertia)
    }

    fn is_convex(&self) -> bool {
        true
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Custom
    }

    fn as_typed_shape(&self) -> TypedShape {
        TypedShape::Custom(self)
    }

    fn ccd_thickness(&self) -> Scalar {
        self.half_size.max_element().adjust_precision()
    }

    fn ccd_angular_thickness(&self) -> Scalar {
        PI
    }

    fn as_support_map(&self) -> Option<&dyn SupportMap> {
        Some(self)
    }
}

impl RayCast for EllipseShape {
    fn cast_local_ray_and_get_normal(
        &self,
        ray: &parry2d::query::Ray,
        max_toi: Scalar,
        solid: bool,
    ) -> Option<parry2d::query::RayIntersection> {
        local_ray_intersection_with_support_map_with_params(
            self,
            &mut VoronoiSimplex::new(),
            ray,
            max_toi,
            solid,
        )
    }
}

impl PointQuery for EllipseShape {
    fn project_local_point(
        &self,
        pt: &parry2d::math::Point<Scalar>,
        solid: bool,
    ) -> parry2d::query::PointProjection {
        local_point_projection_on_support_map(self, &mut VoronoiSimplex::new(), pt, solid)
    }

    fn project_local_point_and_get_feature(
        &self,
        pt: &parry2d::math::Point<Scalar>,
    ) -> (parry2d::query::PointProjection, FeatureId) {
        (self.project_local_point(pt, false), FeatureId::Unknown)
    }
}

/// A regular polygon shape that can be stored in a [`SharedShape`] for a regular polygon.
///
/// This wrapper is required to allow implementing the necessary traits from [`parry2d`]
/// for Bevy's [`RegularPolygon`] type.
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct RegularPolygonShape(pub RegularPolygon);

impl SupportMap for RegularPolygonShape {
    #[inline]
    fn local_support_point(&self, direction: &Vector2<Scalar>) -> Point2<Scalar> {
        // TODO: For polygons with a small number of sides, maybe just iterating
        //       through the vertices and comparing dot products is faster?

        let external_angle = self.external_angle_radians().adjust_precision();
        let circumradius = self.circumradius().adjust_precision();

        // Counterclockwise
        let angle_from_top = if direction.x < 0.0 {
            -Vec2::from(*direction).angle_to(Vec2::Y)
        } else {
            TAU - Vec2::from(*direction).angle_to(Vec2::Y)
        };

        // How many rotations of `external_angle` correspond to the vertex closest to the support direction.
        let n = (angle_from_top / external_angle).round() % self.sides as Scalar;

        // Rotate by an additional 90 degrees so that the first vertex is always at the top.
        let target_angle = n * external_angle + FRAC_PI_2;

        // Compute the vertex corresponding to the target angle on the unit circle.
        Point2::from(circumradius * Vec2::from_angle(target_angle))
    }
}

impl PolygonalFeatureMap for RegularPolygonShape {
    #[inline]
    fn local_support_feature(
        &self,
        direction: &UnitVector2<Scalar>,
        out_feature: &mut PolygonalFeature,
    ) {
        let external_angle = self.external_angle_radians().adjust_precision();
        let circumradius = self.circumradius().adjust_precision();

        // Counterclockwise
        let angle_from_top = if direction.x < 0.0 {
            -Vec2::from(*direction).angle_to(Vec2::Y)
        } else {
            TAU - Vec2::from(*direction).angle_to(Vec2::Y)
        };

        // How many rotations of `external_angle` correspond to the vertices.
        let n_unnormalized = angle_from_top / external_angle;
        let n1 = n_unnormalized.floor() % self.sides as Scalar;
        let n2 = n_unnormalized.ceil() % self.sides as Scalar;

        // Rotate by an additional 90 degrees so that the first vertex is always at the top.
        let target_angle1 = n1 * external_angle + FRAC_PI_2;
        let target_angle2 = n2 * external_angle + FRAC_PI_2;

        // Compute the vertices corresponding to the target angle on the unit circle.
        let vertex1 = Point2::from(circumradius * Vec2::from_angle(target_angle1));
        let vertex2 = Point2::from(circumradius * Vec2::from_angle(target_angle2));

        *out_feature = PolygonalFeature {
            vertices: [vertex1, vertex2],
            vids: [
                PackedFeatureId::vertex(n1 as u32),
                PackedFeatureId::vertex(n2 as u32),
            ],
            fid: PackedFeatureId::face(n1 as u32),
            num_vertices: 2,
        };
    }
}

impl Shape for RegularPolygonShape {
    fn clone_dyn(&self) -> Box<dyn Shape> {
        Box::new(*self)
    }

    fn scale_dyn(
        &self,
        scale: &parry2d::math::Vector<Scalar>,
        _num_subdivisions: u32,
    ) -> Option<Box<dyn Shape>> {
        let circumradius = Vec2::from(*scale).f32() * self.circumradius();
        Some(Box::new(RegularPolygonShape(RegularPolygon::new(
            circumradius.length(),
            self.sides,
        ))))
    }

    fn compute_local_aabb(&self) -> parry2d::bounding_volume::Aabb {
        let aabb = self.aabb_2d(Isometry2d::IDENTITY);
        parry2d::bounding_volume::Aabb::new(
            aabb.min.adjust_precision().into(),
            aabb.max.adjust_precision().into(),
        )
    }

    fn compute_aabb(&self, position: &Isometry<Scalar>) -> parry2d::bounding_volume::Aabb {
        let isometry = na_iso_to_iso(position);
        let aabb = self.aabb_2d(isometry);
        parry2d::bounding_volume::Aabb::new(
            aabb.min.adjust_precision().into(),
            aabb.max.adjust_precision().into(),
        )
    }

    fn compute_local_bounding_sphere(&self) -> parry2d::bounding_volume::BoundingSphere {
        let sphere = self.bounding_circle(Isometry2d::IDENTITY);
        parry2d::bounding_volume::BoundingSphere::new(
            sphere.center.adjust_precision().into(),
            sphere.radius().adjust_precision(),
        )
    }

    fn compute_bounding_sphere(
        &self,
        position: &Isometry<Scalar>,
    ) -> parry2d::bounding_volume::BoundingSphere {
        let isometry = na_iso_to_iso(position);
        let sphere = self.bounding_circle(isometry);
        parry2d::bounding_volume::BoundingSphere::new(
            sphere.center.adjust_precision().into(),
            sphere.radius().adjust_precision(),
        )
    }

    fn clone_box(&self) -> Box<dyn Shape> {
        Box::new(*self)
    }

    fn mass_properties(&self, density: Scalar) -> MassProperties {
        let volume = self.area().adjust_precision();
        let mass = volume * density;

        let half_external_angle = PI / self.sides as Scalar;
        let angular_inertia = mass * self.circumradius().adjust_precision().powi(2) / 6.0
            * (1.0 + 2.0 * half_external_angle.cos().powi(2));

        MassProperties::new(Point2::origin(), mass, angular_inertia)
    }

    fn is_convex(&self) -> bool {
        true
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Custom
    }

    fn as_typed_shape(&self) -> TypedShape {
        TypedShape::Custom(self)
    }

    fn ccd_thickness(&self) -> Scalar {
        self.circumradius().adjust_precision()
    }

    fn ccd_angular_thickness(&self) -> Scalar {
        PI - self.internal_angle_radians().adjust_precision()
    }

    fn as_support_map(&self) -> Option<&dyn SupportMap> {
        Some(self)
    }

    fn as_polygonal_feature_map(&self) -> Option<(&dyn PolygonalFeatureMap, Scalar)> {
        Some((self, 0.0))
    }

    fn feature_normal_at_point(
        &self,
        feature: FeatureId,
        _point: &Point2<Scalar>,
    ) -> Option<UnitVector2<Scalar>> {
        match feature {
            FeatureId::Face(id) => {
                let external_angle = self.external_angle_radians().adjust_precision();
                let normal_angle = id as Scalar * external_angle - external_angle * 0.5 + FRAC_PI_2;
                Some(UnitVector2::new_unchecked(
                    Vec2::from_angle(normal_angle).into(),
                ))
            }
            FeatureId::Vertex(id) => {
                let external_angle = self.external_angle_radians().adjust_precision();
                let normal_angle = id as Scalar * external_angle + FRAC_PI_2;
                Some(UnitVector2::new_unchecked(
                    Vec2::from_angle(normal_angle).into(),
                ))
            }
            _ => None,
        }
    }
}

impl RayCast for RegularPolygonShape {
    fn cast_local_ray_and_get_normal(
        &self,
        ray: &parry2d::query::Ray,
        max_toi: Scalar,
        solid: bool,
    ) -> Option<parry2d::query::RayIntersection> {
        local_ray_intersection_with_support_map_with_params(
            self,
            &mut VoronoiSimplex::new(),
            ray,
            max_toi,
            solid,
        )
    }
}

impl PointQuery for RegularPolygonShape {
    fn project_local_point(
        &self,
        pt: &parry2d::math::Point<Scalar>,
        solid: bool,
    ) -> parry2d::query::PointProjection {
        local_point_projection_on_support_map(self, &mut VoronoiSimplex::new(), pt, solid)
    }

    fn project_local_point_and_get_feature(
        &self,
        pt: &parry2d::math::Point<Scalar>,
    ) -> (parry2d::query::PointProjection, FeatureId) {
        (self.project_local_point(pt, false), FeatureId::Unknown)
    }
}

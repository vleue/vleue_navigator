use bevy::prelude::*;
use itertools::Either;
use parry2d::{
    math::{DIM, Point},
    query::Unsupported,
    shape::{RoundShape, SharedShape, TypedShape, Voxels},
};

use crate::{
    obstacles::parry2d::{
        InnerObstacleSource,
        error::TrimeshBuilderError,
        math::{AdjustPrecision, AsF32, IVector, Scalar, Vector, make_isometry},
        primitives::{EllipseShape, RegularPolygonShape},
        transform::{Position, Rotation},
    },
    prelude::ObstacleSource,
};

/// A sharedshape obstacle that can be used to create a [`NavMesh`].
/// something defined in avian2d
#[derive(Debug, Clone, Component)]
pub struct SharedShapeStorage {
    /// The raw unscaled collider shape.
    pub shape: SharedShape,
    /// The scaled version of the collider shape.
    ///
    /// If the scale is `Vector::ONE`, this will be `None` and `unscaled_shape`
    /// will be used instead.
    scaled_shape: SharedShape,
    /// The global scale used for the collider shape.
    scale: Vec2,
}

/// Controls how the voxelization determines which voxel needs
/// to be considered empty, and which ones will be considered full.
#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Hash, PartialEq, Debug)]
pub enum FillMode {
    /// Only consider full the voxels intersecting the surface of the
    /// shape being voxelized.
    SurfaceOnly,
    /// Use a flood-fill technique to consider fill the voxels intersecting
    /// the surface of the shape being voxelized, as well as all the voxels
    /// bounded of them.
    FloodFill {
        /// Detects holes inside of a solid contour.
        detect_cavities: bool,
        /// Attempts to properly handle self-intersections.
        detect_self_intersections: bool,
    },
}

impl From<FillMode> for parry2d::transformation::voxelization::FillMode {
    fn from(value: FillMode) -> Self {
        match value {
            FillMode::SurfaceOnly => Self::SurfaceOnly,
            FillMode::FloodFill {
                detect_cavities,
                detect_self_intersections,
            } => Self::FloodFill {
                detect_cavities,
                detect_self_intersections,
            },
        }
    }
}

/// Flags used for the preprocessing of a triangle mesh collider.
#[repr(transparent)]
#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(opaque, Hash, PartialEq, Debug)]
pub struct TrimeshFlags(u8);
impl From<SharedShape> for SharedShapeStorage {
    fn from(value: SharedShape) -> Self {
        Self {
            shape: value.clone(),
            scaled_shape: value,
            scale: Vec2::ONE,
        }
    }
}

bitflags::bitflags! {
    impl TrimeshFlags: u8 {
        /// If set, the half-edge topology of the trimesh will be computed if possible.
        const HALF_EDGE_TOPOLOGY = 0b0000_0001;
        /// If set, the half-edge topology and connected components of the trimesh will be computed if possible.
        ///
        /// Because of the way it is currently implemented, connected components can only be computed on
        /// a mesh where the half-edge topology computation succeeds. It will no longer be the case in the
        /// future once we decouple the computations.
        const CONNECTED_COMPONENTS = 0b0000_0010;
        /// If set, any triangle that results in a failing half-hedge topology computation will be deleted.
        const DELETE_BAD_TOPOLOGY_TRIANGLES = 0b0000_0100;
        /// If set, the trimesh will be assumed to be oriented (with outward normals).
        ///
        /// The pseudo-normals of its vertices and edges will be computed.
        const ORIENTED = 0b0000_1000;
        /// If set, the duplicate vertices of the trimesh will be merged.
        ///
        /// Two vertices with the exact same coordinates will share the same entry on the
        /// vertex buffer and the index buffer is adjusted accordingly.
        const MERGE_DUPLICATE_VERTICES = 0b0001_0000;
        /// If set, the triangles sharing two vertices with identical index values will be removed.
        ///
        /// Because of the way it is currently implemented, this methods implies that duplicate
        /// vertices will be merged. It will no longer be the case in the future once we decouple
        /// the computations.
        const DELETE_DEGENERATE_TRIANGLES = 0b0010_0000;
        /// If set, two triangles sharing three vertices with identical index values (in any order) will be removed.
        ///
        /// Because of the way it is currently implemented, this methods implies that duplicate
        /// vertices will be merged. It will no longer be the case in the future once we decouple
        /// the computations.
        const DELETE_DUPLICATE_TRIANGLES = 0b0100_0000;
        /// If set, a special treatment will be applied to contact manifold calculation to eliminate
        /// or fix contacts normals that could lead to incorrect bumps in physics simulation
        /// (especially on flat surfaces).
        ///
        /// This is achieved by taking into account adjacent triangle normals when computing contact
        /// points for a given triangle.
        const FIX_INTERNAL_EDGES = 0b1000_0000 | Self::ORIENTED.bits() | Self::MERGE_DUPLICATE_VERTICES.bits();
    }
}

impl From<TrimeshFlags> for parry2d::shape::TriMeshFlags {
    fn from(value: TrimeshFlags) -> Self {
        Self::from_bits(value.bits().into()).unwrap()
    }
}

/// Parameters controlling the VHACD convex decomposition.
///
/// See <https://github.com/Unity-Technologies/VHACD#parameters> for details.
#[derive(Clone, PartialEq, Debug, Copy, Reflect)]
#[reflect(PartialEq, Debug)]
pub struct VhacdParameters {
    /// Maximum concavity.
    ///
    /// Default: 0.1 (in 2D), 0.01 (in 3D).
    /// Valid range `[0.0, 1.0]`.
    pub concavity: Scalar,
    /// Controls the bias toward clipping along symmetry planes.
    ///
    /// Default: 0.05.
    /// Valid Range: `[0.0, 1.0]`.
    pub alpha: Scalar,
    /// Controls the bias toward clipping along revolution planes.
    ///
    /// Default: 0.05.
    /// Valid Range: `[0.0, 1.0]`.
    pub beta: Scalar,
    /// Resolution used during the voxelization stage.
    ///
    /// Default: 256 (in 2D), 64 (in 3D).
    pub resolution: u32,
    /// Controls the granularity of the search for the best
    /// clipping plane during the decomposition.
    ///
    /// Default: 4
    pub plane_downsampling: u32,
    /// Controls the precision of the convex-hull generation
    /// process during the clipping plane selection stage.
    ///
    /// Default: 4
    pub convex_hull_downsampling: u32,
    /// Controls the way the input mesh or polyline is being
    /// voxelized.
    ///
    /// Default: `FillMode::FloodFill { detect_cavities: false, detect_self_intersections: false }`
    pub fill_mode: FillMode,
    /// Controls whether the convex-hull should be approximated during the decomposition stage.
    /// Setting this to `true` increases performances with a slight degradation of the decomposition
    /// quality.
    ///
    /// Default: true
    pub convex_hull_approximation: bool,
    /// Controls the max number of convex-hull generated by the convex decomposition.
    ///
    /// Default: 1024
    pub max_convex_hulls: u32,
}
impl Default for VhacdParameters {
    fn default() -> Self {
        Self {
            resolution: 256,
            concavity: 0.1,
            plane_downsampling: 4,
            convex_hull_downsampling: 4,
            alpha: 0.05,
            beta: 0.05,
            convex_hull_approximation: true,
            max_convex_hulls: 1024,
            fill_mode: FillMode::FloodFill {
                detect_cavities: false,
                detect_self_intersections: false,
            },
        }
    }
}

impl From<VhacdParameters> for parry2d::transformation::vhacd::VHACDParameters {
    fn from(value: VhacdParameters) -> Self {
        Self {
            concavity: value.concavity,
            alpha: value.alpha,
            beta: value.beta,
            resolution: value.resolution,
            plane_downsampling: value.plane_downsampling,
            convex_hull_downsampling: value.convex_hull_downsampling,
            fill_mode: value.fill_mode.into(),
            convex_hull_approximation: value.convex_hull_approximation,
            max_convex_hulls: value.max_convex_hulls,
        }
    }
}

impl SharedShapeStorage {
    /// Returns the raw unscaled shape of the collider.
    pub fn shape(&self) -> &SharedShape {
        &self.shape
    }

    /// Returns the shape of the collider with the scale from its `GlobalTransform` applied.
    pub fn shape_scaled(&self) -> &SharedShape {
        &self.scaled_shape
    }

    /// Sets the unscaled shape of the collider. The collider's scale will be applied to this shape.
    pub fn set_shape(&mut self, shape: SharedShape) {
        self.shape = shape;

        // TODO: The number of subdivisions probably shouldn't be hard-coded
        if let Ok(scaled) = scale_shape(&self.shape, self.scale, 10) {
            self.scaled_shape = scaled;
        } else {
            log::error!("Failed to create convex hull for scaled collider.");
        }
    }

    /// Returns the global scale of the collider.
    pub fn scale(&self) -> Vec2 {
        self.scale
    }

    /// Creates a collider with a compound shape defined by a given vector of colliders with a position and a rotation.
    ///
    /// Especially for dynamic rigid bodies, compound shape colliders should be preferred over triangle meshes and polylines,
    /// because convex shapes typically provide more reliable results.
    ///
    /// If you want to create a compound shape from a 3D triangle mesh or 2D polyline, consider using the
    /// [`Collider::convex_decomposition`] method.
    pub fn compound(
        shapes: Vec<(
            impl Into<Position>,
            impl Into<Rotation>,
            impl Into<SharedShapeStorage>,
        )>,
    ) -> Self {
        let shapes = shapes
            .into_iter()
            .map(|(p, r, c)| {
                (
                    make_isometry(*p.into(), r.into()),
                    c.into().shape_scaled().clone(),
                )
            })
            .collect::<Vec<_>>();
        SharedShape::compound(shapes).into()
    }

    /// Creates a collider with a circle shape defined by its radius.
    pub fn circle(radius: Scalar) -> Self {
        SharedShape::ball(radius).into()
    }

    /// Creates a collider with an ellipse shape defined by a half-width and half-height.
    pub fn ellipse(half_width: Scalar, half_height: Scalar) -> Self {
        SharedShape::new(EllipseShape(Ellipse::new(half_width, half_height))).into()
    }

    /// Creates a collider with a rectangle shape defined by its extents.
    pub fn rectangle(x_length: Scalar, y_length: Scalar) -> Self {
        SharedShape::cuboid(x_length * 0.5, y_length * 0.5).into()
    }

    /// Creates a collider with a rectangle shape defined by its extents and rounded corners.
    pub fn round_rectangle(x_length: Scalar, y_length: Scalar, border_radius: Scalar) -> Self {
        SharedShape::round_cuboid(x_length * 0.5, y_length * 0.5, border_radius).into()
    }

    /// Creates a collider with a capsule shape defined by its radius
    /// and its height along the `Y` axis, excluding the hemispheres.
    pub fn capsule(radius: Scalar, length: Scalar) -> Self {
        SharedShape::capsule(
            (Vector::Y * length * 0.5).into(),
            (Vector::NEG_Y * length * 0.5).into(),
            radius,
        )
        .into()
    }

    /// Creates a collider with a capsule shape defined by its radius and endpoints `a` and `b`.
    pub fn capsule_endpoints(radius: Scalar, a: Vector, b: Vector) -> Self {
        SharedShape::capsule(a.into(), b.into(), radius).into()
    }

    /// Creates a collider with a [half-space](https://en.wikipedia.org/wiki/Half-space_(geometry)) shape
    /// defined by the outward normal of its planar boundary.
    pub fn half_space(outward_normal: Vector) -> Self {
        SharedShape::halfspace(nalgebra::Unit::new_normalize(outward_normal.into())).into()
    }

    /// Creates a collider with a segment shape defined by its endpoints `a` and `b`.
    pub fn segment(a: Vector, b: Vector) -> Self {
        SharedShape::segment(a.into(), b.into()).into()
    }

    /// Creates a collider with a triangle shape defined by its points `a`, `b`, and `c`.
    ///
    /// If the triangle is oriented clockwise, it will be reversed to be counterclockwise
    /// by swapping `b` and `c`. This is needed for collision detection.
    ///
    /// If you know that the given points produce a counterclockwise triangle,
    /// consider using [`Collider::triangle_unchecked`] instead.
    pub fn triangle(a: Vector, b: Vector, c: Vector) -> Self {
        let mut triangle = parry2d::shape::Triangle::new(a.into(), b.into(), c.into());

        // Make sure the triangle is counterclockwise. This is needed for collision detection.
        if triangle.orientation(1e-8) == parry2d::shape::TriangleOrientation::Clockwise {
            triangle.reverse();
        }

        SharedShape::new(triangle).into()
    }

    /// Creates a collider with a triangle shape defined by its points `a`, `b`, and `c`.
    ///
    /// The orientation of the triangle is assumed to be counterclockwise.
    /// This is needed for collision detection.
    ///
    /// If you are unsure about the orientation of the triangle, consider using [`Collider::triangle`] instead.
    pub fn triangle_unchecked(a: Vector, b: Vector, c: Vector) -> Self {
        SharedShape::triangle(a.into(), b.into(), c.into()).into()
    }

    /// Creates a collider with a regular polygon shape defined by the circumradius and the number of sides.
    pub fn regular_polygon(circumradius: f32, sides: u32) -> Self {
        SharedShape::new(RegularPolygonShape(RegularPolygon::new(
            circumradius,
            sides,
        )))
        .into()
    }

    /// Creates a collider with a polyline shape defined by its vertices and optionally an index buffer.
    pub fn polyline(vertices: Vec<Vector>, indices: Option<Vec<[u32; 2]>>) -> Self {
        let vertices = vertices.into_iter().map(|v| v.into()).collect();
        SharedShape::polyline(vertices, indices).into()
    }

    /// Creates a collider with a triangle mesh shape defined by its vertex and index buffers.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// The [`CollisionMargin`] component can be used to add thickness to the shape if needed.
    /// For thin shapes like triangle meshes, it can help improve collision stability and performance.
    ///
    /// # Panics
    ///
    /// Panics if the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn trimesh(vertices: Vec<Vector>, indices: Vec<[u32; 3]>) -> Self {
        Self::try_trimesh(vertices, indices)
            .unwrap_or_else(|error| panic!("Trimesh creation failed: {error:?}"))
    }

    /// Tries to create a collider with a triangle mesh shape defined by its vertex and index buffers.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// The [`CollisionMargin`] component can be used to add thickness to the shape if needed.
    /// For thin shapes like triangle meshes, it can help improve collision stability and performance.
    ///
    /// # Errors
    ///
    /// Returns a [`TrimeshBuilderError`] if the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn try_trimesh(
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
    ) -> Result<Self, TrimeshBuilderError> {
        let vertices = vertices.into_iter().map(|v| v.into()).collect();
        SharedShape::trimesh(vertices, indices).map(|trimesh| trimesh.into())
    }

    /// Creates a collider with a triangle mesh shape defined by its vertex and index buffers
    /// and flags controlling the preprocessing.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// The [`CollisionMargin`] component can be used to add thickness to the shape if needed.
    /// For thin shapes like triangle meshes, it can help improve collision stability and performance.
    ///
    /// # Panics
    ///
    /// Panics if after preprocessing the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn trimesh_with_config(
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
        flags: TrimeshFlags,
    ) -> Self {
        Self::try_trimesh_with_config(vertices, indices, flags)
            .unwrap_or_else(|error| panic!("Trimesh creation failed: {error:?}"))
    }

    /// Tries to create a collider with a triangle mesh shape defined by its vertex and index buffers
    /// and flags controlling the preprocessing.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// The [`CollisionMargin`] component can be used to add thickness to the shape if needed.
    /// For thin shapes like triangle meshes, it can help improve collision stability and performance.
    ///
    /// # Errors
    ///
    /// Returns a [`TrimeshBuilderError`] if after preprocessing the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn try_trimesh_with_config(
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
        flags: TrimeshFlags,
    ) -> Result<Self, TrimeshBuilderError> {
        let vertices = vertices.into_iter().map(|v| v.into()).collect();
        SharedShape::trimesh_with_flags(vertices, indices, flags.into())
            .map(|trimesh| trimesh.into())
    }

    /// Creates a collider shape with a compound shape obtained from the decomposition of a given polyline
    /// defined by its vertex and index buffers.
    pub fn convex_decomposition(vertices: Vec<Vector>, indices: Vec<[u32; 2]>) -> Self {
        let vertices = vertices.iter().map(|v| (*v).into()).collect::<Vec<_>>();
        SharedShape::convex_decomposition(&vertices, &indices).into()
    }

    /// Creates a collider shape with a compound shape obtained from the decomposition of a given polyline
    /// defined by its vertex and index buffers. The given [`VhacdParameters`] are used for configuring
    /// the decomposition process.
    pub fn convex_decomposition_with_config(
        vertices: Vec<Vector>,
        indices: Vec<[u32; 2]>,
        params: &VhacdParameters,
    ) -> Self {
        let vertices = vertices.iter().map(|v| (*v).into()).collect::<Vec<_>>();
        SharedShape::convex_decomposition_with_params(&vertices, &indices, &(*params).into()).into()
    }

    /// Creates a collider with a [convex polygon](https://en.wikipedia.org/wiki/Convex_polygon) shape obtained after computing
    /// the [convex hull](https://en.wikipedia.org/wiki/Convex_hull) of the given points.
    pub fn convex_hull(points: Vec<Vector>) -> Option<Self> {
        let points = points.iter().map(|v| (*v).into()).collect::<Vec<_>>();
        SharedShape::convex_hull(&points).map(Into::into)
    }

    /// Creates a collider with a [convex polygon](https://en.wikipedia.org/wiki/Convex_polygon) shape **without** computing
    /// the [convex hull](https://en.wikipedia.org/wiki/Convex_hull) of the given points: convexity of the input is
    /// assumed and not checked.
    pub fn convex_polyline(points: Vec<Vector>) -> Option<Self> {
        let points = points.iter().map(|v| (*v).into()).collect::<Vec<_>>();
        SharedShape::convex_polyline(points).map(Into::into)
    }
    /// Creates a collider shape made of voxels.
    ///
    /// Each voxel has the size `voxel_size` and grid coordinate given by `grid_coordinates`.
    pub fn voxels(voxel_size: Vector, grid_coordinates: &[IVector]) -> Self {
        let shape = Voxels::new(
            voxel_size.into(),
            &Self::ivec_array_from_point_int_array(grid_coordinates),
        );
        SharedShape::new(shape).into()
    }
    /// Creates a collider shape made of voxels.
    ///
    /// Each voxel has the size `voxel_size` and contains at least one point from `points`.
    pub fn voxels_from_points(voxel_size: Vector, points: &[Vector]) -> Self {
        SharedShape::voxels_from_points(
            voxel_size.into(),
            &Self::vec_array_from_point_float_array(points),
        )
        .into()
    }
    /// Creates a voxel collider obtained from the decomposition of the given polyline into voxelized convex parts.
    pub fn voxelized_polyline(
        vertices: &[Vector],
        indices: &[[u32; 2]],
        voxel_size: Scalar,
        fill_mode: FillMode,
    ) -> Self {
        let vertices = Self::vec_array_from_point_float_array(vertices);
        SharedShape::voxelized_mesh(&vertices, indices, voxel_size, fill_mode.into()).into()
    }
    #[doc = "Creates a collider with a compound shape obtained from the decomposition of the given polyline into voxelized convex parts."]
    pub fn voxelized_convex_decomposition(
        vertices: &[Vector],
        indices: &[[u32; DIM]],
    ) -> Vec<Self> {
        Self::voxelized_convex_decomposition_with_config(
            vertices,
            indices,
            &VhacdParameters::default(),
        )
    }
    #[doc = "Creates a collider with a compound shape obtained from the decomposition of the given polyline into voxelized convex parts."]
    pub fn voxelized_convex_decomposition_with_config(
        vertices: &[Vector],
        indices: &[[u32; DIM]],
        parameters: &VhacdParameters,
    ) -> Vec<Self> {
        SharedShape::voxelized_convex_decomposition_with_params(
            &Self::vec_array_from_point_float_array(vertices),
            indices,
            &(*parameters).into(),
        )
        .into_iter()
        .map(|c| c.into())
        .collect()
    }

    fn ivec_array_from_point_int_array(points: &[IVector]) -> Vec<Point<i32>> {
        points
            .iter()
            .map(|p| Point::new(p.x, p.y))
            .collect::<Vec<_>>()
    }

    fn vec_array_from_point_float_array(points: &[Vector]) -> Vec<Point<Scalar>> {
        points
            .iter()
            .map(|p| Point::new(p.x, p.y))
            .collect::<Vec<_>>()
    }

    /// Creates a collider with a heightfield shape.
    ///
    /// A 2D heightfield is a segment along the `X` axis, subdivided at regular intervals.
    ///
    /// `heights` is a list indicating the altitude of each subdivision point, and `scale` controls
    /// the scaling factor along each axis.
    pub fn heightfield(heights: Vec<Scalar>, scale: Vector) -> Self {
        SharedShape::heightfield(heights.into(), scale.into()).into()
    }
}

pub fn scale_shape(
    shape: &SharedShape,
    scale: Vector,
    num_subdivisions: u32,
) -> Result<SharedShape, Unsupported> {
    let scale = scale.abs();
    match shape.as_typed_shape() {
        TypedShape::Cuboid(s) => Ok(SharedShape::new(s.scaled(&scale.abs().into()))),
        TypedShape::RoundCuboid(s) => Ok(SharedShape::new(RoundShape {
            border_radius: s.border_radius,
            inner_shape: s.inner_shape.scaled(&scale.abs().into()),
        })),
        TypedShape::Capsule(c) => match c.scaled(&scale.abs().into(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {scale} to Capsule shape.");
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::Ball(b) => {
            {
                if scale.x == scale.y {
                    Ok(SharedShape::ball(b.radius * scale.x.abs()))
                } else {
                    // A 2D circle becomes an ellipse when scaled non-uniformly.
                    Ok(SharedShape::new(EllipseShape(Ellipse {
                        half_size: Vec2::splat(b.radius) * scale.f32().abs(),
                    })))
                }
            }
        }
        TypedShape::Segment(s) => Ok(SharedShape::new(s.scaled(&scale.into()))),
        TypedShape::Triangle(t) => Ok(SharedShape::new(t.scaled(&scale.into()))),
        TypedShape::RoundTriangle(t) => Ok(SharedShape::new(RoundShape {
            border_radius: t.border_radius,
            inner_shape: t.inner_shape.scaled(&scale.into()),
        })),
        TypedShape::TriMesh(t) => Ok(SharedShape::new(t.clone().scaled(&scale.into()))),
        TypedShape::Polyline(p) => Ok(SharedShape::new(p.clone().scaled(&scale.into()))),
        TypedShape::HalfSpace(h) => match h.scaled(&scale.into()) {
            None => {
                log::error!("Failed to apply scale {scale} to HalfSpace shape.");
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::HeightField(h) => Ok(SharedShape::new(h.clone().scaled(&scale.into()))),
        TypedShape::ConvexPolygon(cp) => match cp.clone().scaled(&scale.into()) {
            None => {
                log::error!("Failed to apply scale {scale} to ConvexPolygon shape.");
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::RoundConvexPolygon(cp) => match cp.inner_shape.clone().scaled(&scale.into()) {
            None => {
                log::error!("Failed to apply scale {scale} to RoundConvexPolygon shape.");
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(RoundShape {
                border_radius: cp.border_radius,
                inner_shape: scaled,
            })),
        },

        TypedShape::Compound(c) => {
            let mut scaled = Vec::with_capacity(c.shapes().len());

            for (iso, shape) in c.shapes() {
                scaled.push((
                    make_isometry(
                        Vector::from(iso.translation) * scale,
                        Rotation::radians(iso.rotation.angle()),
                    ),
                    scale_shape(shape, scale, num_subdivisions)?,
                ));
            }
            Ok(SharedShape::compound(scaled))
        }
        TypedShape::Custom(shape) => {
            {
                if let Some(ellipse) = shape.as_shape::<EllipseShape>() {
                    return Ok(SharedShape::new(EllipseShape(Ellipse {
                        half_size: ellipse.half_size * scale.f32().abs(),
                    })));
                }
                if let Some(polygon) = shape.as_shape::<RegularPolygonShape>() {
                    if scale.x == scale.y {
                        return Ok(SharedShape::new(RegularPolygonShape(RegularPolygon::new(
                            polygon.circumradius() * scale.x.abs(),
                            polygon.sides,
                        ))));
                    } else {
                        let vertices = polygon
                            .vertices(0.0)
                            .into_iter()
                            .map(|v| v.adjust_precision().into())
                            .collect::<Vec<_>>();

                        return scale_shape(
                            &SharedShape::convex_hull(&vertices).unwrap(),
                            scale,
                            num_subdivisions,
                        );
                    }
                }
            }
            Err(Unsupported)
        }
        TypedShape::Voxels(_) => Err(Unsupported),
    }
}

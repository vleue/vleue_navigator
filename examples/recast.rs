use std::f32::consts::FRAC_PI_2;

use bevy::{color::palettes, prelude::*};
use polyanya::{Layer, Vertex};
use serde::Deserialize;
use vleue_navigator::{VleueNavigatorPlugin, display_mesh_gizmo, prelude::*};

fn main() {
    let mut rdr =
        std::io::BufReader::new(std::fs::File::open("assets/recast/poly_mesh.json").unwrap());
    let polygon_mesh: PolygonMesh = serde_json::from_reader(&mut rdr).unwrap();

    let layer = Layer::new(
        polygon_mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                Vertex::new(
                    Vec2::new(
                        v.z as f32 * polygon_mesh.cell_size,
                        v.x as f32 * polygon_mesh.cell_size,
                    ),
                    polygon_mesh
                        .polygons()
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.contains(&(i as u16)))
                        .map(|(n, _)| n as u32)
                        .collect(),
                )
            })
            .collect(),
        polygon_mesh
            .polygons()
            .into_iter()
            .map(|p| polyanya::Polygon::new(p.into_iter().map(|i| i as u32).collect(), false))
            .collect(),
    )
    .unwrap();

    let navmesh = polyanya::Mesh {
        layers: vec![layer],
        search_delta: 0.1,
        search_steps: 5,
    };

    App::new()
        .insert_resource(ClearColor(Color::srgb(0., 0., 0.01)))
        .add_plugins((
            DefaultPlugins,
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_recast_navmesh, draw_parsed_recast_navmesh))
        .insert_resource(polygon_mesh)
        .insert_resource(RecastNavmesh(navmesh))
        .run();
}

#[derive(Resource)]
struct RecastNavmesh(polyanya::Mesh);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(50.0, 150.0, -30.0).looking_at(Vec3::new(0.0, 0.0, -30.0), Vec3::Y),
    ));

    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("recast/dungeon.glb")),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.0;
    }
}

fn draw_recast_navmesh(mut gizmos: Gizmos, recast: Res<PolygonMesh>) {
    for polygon in recast.polygons() {
        let points = recast.polygon(polygon);
        let mut points: Vec<_> = points
            .iter()
            .map(|p| {
                p.as_vec3() * Vec3::new(recast.cell_size, recast.cell_height, recast.cell_size)
                    + recast.aabb.min
            })
            .collect();
        points.push(points[0]);

        gizmos.linestrip(points, Color::linear_rgb(1.0, 0.0, 0.0));
    }
}

fn draw_parsed_recast_navmesh(
    mut gizmos: Gizmos,
    recast: Res<RecastNavmesh>,
    recast_original: Res<PolygonMesh>,
) {
    display_mesh_gizmo(
        &recast.0,
        &Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -FRAC_PI_2, 0.0, -FRAC_PI_2))
            .with_translation(recast_original.aabb.min)
            .into(),
        palettes::tailwind::RED_400.into(),
        &mut gizmos,
    );
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Resource)]
pub struct PolygonMesh {
    /// The mesh vertices.
    pub vertices: Vec<UVec3>,
    /// Polygon and neighbor data. [Length: [`Self::polygon_count`] * 2 * [`Self::vertices_per_polygon`]
    pub polygons: Vec<u16>,
    /// The region id assigned to each polygon.
    pub regions: Vec<String>,
    /// The flags assigned to each polygon.
    pub flags: Vec<u16>,
    /// The area id assigned to each polygon.
    pub areas: Vec<u8>,
    /// The number of allocated polygons
    pub max_polygons: usize,
    /// The maximum number of vertices per polygon
    pub vertices_per_polygon: usize,
    /// The bounding box of the mesh in world space.
    pub aabb: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    pub cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    pub cell_height: f32,
    /// The AABB border size used to generate the source data from which the mesh was derived.
    pub border_size: u16,
    /// The max error of the polygon edges in the mesh.
    pub max_edge_error: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Aabb3d {
    pub min: Vec3,
    pub max: Vec3,
}

impl PolygonMesh {
    fn polygons(&self) -> Vec<Vec<u16>> {
        self.polygons
            .chunks(self.vertices_per_polygon * 2)
            .map(|chunk| {
                chunk
                    .iter()
                    .take(self.vertices_per_polygon)
                    .take_while(|p| **p != 65535)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn polygon(&self, vertices: Vec<u16>) -> Vec<UVec3> {
        vertices
            .iter()
            .map(|&vertex| self.vertices[vertex as usize])
            .collect()
    }
}

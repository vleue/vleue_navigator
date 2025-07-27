use std::f32::consts::FRAC_PI_2;

use bevy::{color::palettes, platform::collections::HashMap, prelude::*};
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
                        v.x as f32 * polygon_mesh.cell_size,
                        v.z as f32 * polygon_mesh.cell_size,
                    ) + polygon_mesh.aabb.min.xz(),
                    polygon_mesh
                        .polygons()
                        .iter()
                        .enumerate()
                        .filter_map(|(n, p)| p.contains(&(i as u16)).then_some(n as u32))
                        .collect(),
                )
            })
            .collect(),
        polygon_mesh
            .polygons()
            .into_iter()
            .map(|p| {
                let mut p: Vec<_> = p.into_iter().map(|i| i as u32).collect();
                p.reverse();
                polyanya::Polygon::new(p.into_iter().map(|i| i as u32).collect(), false)
            })
            .collect(),
    )
    .unwrap();

    let mut navmesh = polyanya::Mesh {
        layers: vec![layer],
        search_delta: 0.1,
        search_steps: 5,
    };
    navmesh.reorder_neighbors_ccw_and_fix_corners();
    navmesh.bake();

    let mut rdr =
        std::io::BufReader::new(std::fs::File::open("assets/recast/detail_mesh.json").unwrap());
    let detailed_mesh: DetailedMesh = serde_json::from_reader(&mut rdr).unwrap();

    let common = detailed_mesh.common_vertices();
    let layer = Layer::new(
        detailed_mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                Vertex::new(
                    Vec2::new(v.x, v.z),
                    detailed_mesh
                        .triangles()
                        .iter()
                        .enumerate()
                        .filter_map(|(n, p)| {
                            common
                                .get(&(i as u32))
                                .unwrap()
                                .iter()
                                .find(|ii| p.contains(&(**ii as u32)))
                                .map(|_| n as u32)
                        })
                        .collect(),
                )
            })
            .collect(),
        detailed_mesh
            .triangles()
            .into_iter()
            .map(|p| polyanya::Polygon::new(vec![p[2] as u32, p[1] as u32, p[0] as u32], false))
            .collect(),
    )
    .unwrap();

    let mut detailed_navmesh = polyanya::Mesh {
        layers: vec![layer],
        search_delta: 0.1,
        search_steps: 5,
    };
    detailed_navmesh.reorder_neighbors_ccw_and_fix_corners();
    detailed_navmesh.bake();

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
        .insert_resource(RecastNavmesh(navmesh, detailed_navmesh))
        .run();
}

#[derive(Resource)]
struct RecastNavmesh(polyanya::Mesh, polyanya::Mesh);

fn setup(
    mut commands: Commands,
    // asset_server: Res<AssetServer>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(50.0, 150.0, -30.0).looking_at(Vec3::new(0.0, 0.0, -30.0), Vec3::Y),
    ));

    // commands.spawn(SceneRoot(
    //     asset_server.load(GltfAssetLabel::Scene(0).from_asset("recast/dungeon.glb")),
    // ));

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

fn draw_recast_navmesh(// mut gizmos: Gizmos, recast: Res<PolygonMesh>
) {
    // for polygon in recast.polygons() {
    //     let points = recast.polygon(polygon);
    //     let mut points: Vec<_> = points
    //         .iter()
    //         .map(|p| {
    //             p.as_vec3() * Vec3::new(recast.cell_size, recast.cell_height, recast.cell_size)
    //                 + recast.aabb.min
    //         })
    //         .collect();
    //     points.push(points[0]);

    //     gizmos.linestrip(points, Color::linear_rgb(1.0, 0.0, 0.0));
    // }
}

fn draw_parsed_recast_navmesh(
    mut gizmos: Gizmos,
    recast: Res<RecastNavmesh>,
    recast_original: Res<PolygonMesh>,
    time: Res<Time>,
) {
    let start = vec3(46.998413, 9.998184, 1.717747);
    let end = vec3(20.703018, 18.651773, -80.770203);

    gizmos.sphere(
        vec3(start.x, recast_original.aabb.min.y, start.z),
        1.0,
        palettes::tailwind::LIME_400,
    );
    gizmos.sphere(
        vec3(end.x, recast_original.aabb.min.y, end.z),
        1.0,
        palettes::tailwind::YELLOW_400,
    );

    let mesh = if time.elapsed_secs() as u32 % 2 == 0 {
        &recast.0
    } else {
        &recast.1
    };
    display_mesh_gizmo(
        mesh,
        &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
        palettes::tailwind::BLUE_400.into(),
        &mut gizmos,
    );
    let Some(path) = mesh.path(start.xz(), end.xz()) else {
        return;
    };

    let mut path = path
        .path
        .iter()
        .map(|v| vec3(v.x, recast_original.aabb.min.y + 0.1, v.y))
        .collect::<Vec<_>>();
    path.insert(0, vec3(start.x, recast_original.aabb.min.y + 0.1, start.z));
    gizmos.linestrip(path, palettes::tailwind::RED_600);
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

    // fn polygon(&self, vertices: Vec<u16>) -> Vec<UVec3> {
    //     vertices
    //         .iter()
    //         .map(|&vertex| self.vertices[vertex as usize])
    //         .collect()
    // }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Resource)]
pub struct DetailedMesh {
    meshes: Vec<SubMesh>,
    vertices: Vec<Vec3>,
    triangles: Vec<([u32; 3], u32)>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Resource)]
pub struct SubMesh {
    first_vertex_index: u32,
    vertex_count: u32,
    first_triangle_index: u32,
    triangle_count: u32,
}

impl DetailedMesh {
    fn triangles(&self) -> Vec<[u32; 3]> {
        self.meshes
            .iter()
            .flat_map(|mesh| {
                self.triangles
                    .iter()
                    .skip(mesh.first_triangle_index as usize)
                    .take(mesh.triangle_count as usize)
                    .map(|&([a, b, c], _)| {
                        [
                            a + mesh.first_vertex_index,
                            b + mesh.first_vertex_index,
                            c + mesh.first_vertex_index,
                        ]
                    })
            })
            .collect()
    }

    fn common_vertices(&self) -> HashMap<u32, Vec<u32>> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                (
                    i as u32,
                    self.vertices
                        .iter()
                        .enumerate()
                        .filter_map(|(i2, v2)| (v == v2).then_some(i2 as u32))
                        .collect(),
                )
            })
            .collect()
    }

    // fn triangle(&self, vertices: [u32; 3]) -> [Vec3; 3] {
    //     [
    //         self.vertices[vertices[0] as usize],
    //         self.vertices[vertices[1] as usize],
    //         self.vertices[vertices[2] as usize],
    //     ]
    // }
}

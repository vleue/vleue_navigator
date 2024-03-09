# NavMesh for Bevy

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Release Doc](https://docs.rs/bevy_pathmesh/badge.svg)](https://docs.rs/bevy_pathmesh)
[![Crate](https://img.shields.io/crates/v/bevy_pathmesh.svg)](https://crates.io/crates/bevy_pathmesh)

Navigation mesh for [Bevy](http://github.com/bevyengine/bevy) using [Polyanya](https://github.com/vleue/polyanya).

![map with many points finding their paths](https://raw.githubusercontent.com/vleue/bevy_pathmesh/main/screenshots/many.png)

Check out the [WASM demo](https://vleue.github.io/vleue_navigator/)

> [!WARNING]
> This crate has been renamed to [vleue_navigator](https://github.com/vleue/vleue_navigator). For updates and continued support, change your dependency!

## Usage

Loading a mesh from a gLTF file, then building a `PathMesh` from it and using it for getting paths between random points.

```rust,no_run
use bevy::{
    gltf::{Gltf, GltfMesh},
    prelude::*,
};

use bevy_pathmesh::{PathMesh, PathMeshPlugin};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PathMeshPlugin))
        .add_systems(Startup, load)
        .add_systems(Update, get_path)
        .run()
}

#[derive(Resource)]
struct Handles(Handle<Gltf>, Option<Handle<PathMesh>>);

fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Handles(asset_server.load("navmesh.glb"), None));
}

fn get_path(
    mut handles: ResMut<Handles>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    meshes: Res<Assets<Mesh>>,
    mut path_meshes: ResMut<Assets<PathMesh>>,
) {
    if handles.1.is_none() {
        // Get the gltf struct loaded from the file
        let Some(gltf) = gltfs.get(&handles.0) else {
            return
         };
        // Get the mesh called `navmesh`
        let Some(gltf_mesh) = gltf_meshes.get(&gltf.named_meshes["navmesh"]) else {
            return
         };
        // Get the actual mesh
        let Some(mesh) = meshes.get(&gltf_mesh.primitives[0].mesh) else {
            return
        };
        // Build a `PathMesh` from that mesh, then save it as an asset
        handles.1 = Some(path_meshes.add(PathMesh::from_bevy_mesh(mesh)));
    } else {
        // Get the path mesh, then search for a path
        let Some(path_mesh) = path_meshes.get(handles.1.as_ref().unwrap()) else {
            return
        };
        // Find two random point
        let from = Vec2::new(
            rand::thread_rng().gen_range(-50.0..50.0),
            rand::thread_rng().gen_range(-50.0..50.0),
        );
        let to = Vec2::new(
            rand::thread_rng().gen_range(-50.0..50.0),
            rand::thread_rng().gen_range(-50.0..50.0),
        );
        if let Some(path) = path_mesh.path(from, to) {
            info!("path from {} to {}: {:?}", from, to, path);
        } else {
            info!("no path between {} and {}", from, to)
        }
    }
}
```

|Bevy|bevy_pathmesh|
|---|---|
|0.13|0.6|
|0.11|0.5|
|0.10|0.4|

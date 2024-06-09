# Navigation for Bevy with NavMesh

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Release Doc](https://docs.rs/vleue_navigator/badge.svg)](https://docs.rs/vleue_navigator)
[![Crate](https://img.shields.io/crates/v/vleue_navigator.svg)](https://crates.io/crates/vleue_navigator)

Navigation mesh for [Bevy](http://github.com/bevyengine/bevy) using [Polyanya](https://github.com/vleue/polyanya).

![map with many points finding their paths](https://raw.githubusercontent.com/vleue/vleue_navigator/main/screenshots/many.png)

Check out the [WASM demo](https://vleue.github.io/vleue_navigator/)

## Usage

Loading a mesh from a gLTF file, then building a `NavMesh` from it and using it for getting paths between random points.

```rust,no_run
use bevy::{
    gltf::{Gltf, GltfMesh},
    prelude::*,
};

use vleue_navigator::{NavMesh, VleueNavigatorPlugin};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VleueNavigatorPlugin))
        .add_systems(Startup, load)
        .add_systems(Update, get_path)
        .run();
}

#[derive(Resource)]
struct Handles(Handle<Gltf>, Option<Handle<NavMesh>>);

fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Handles(asset_server.load("navmesh.glb"), None));
}

fn get_path(
    mut handles: ResMut<Handles>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    meshes: Res<Assets<Mesh>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
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
        // Build a `NavMesh` from that mesh, then save it as an asset
        handles.1 = Some(navmeshes.add(NavMesh::from_bevy_mesh(mesh)));
    } else {
        // Get the navmesh, then search for a path
        let Some(navmesh) = navmeshes.get(handles.1.as_ref().unwrap()) else {
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
        if let Some(path) = navmesh.path(from, to) {
            info!("path from {} to {}: {:?}", from, to, path);
        } else {
            info!("no path between {} and {}", from, to)
        }
    }
}
```

|Bevy|vleue_navigator|
|---|---|
|0.14|0.8|
|0.13|0.7|

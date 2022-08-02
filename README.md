# NavMesh for Bevy

Very basic support of NavMesh in Bevy.
Current status:
* Not very user friendly
* Has a bug that sometimes follow an edge instead of a more optimized path
* Has a bug that very rarely put the character outside of the mesh

To view the demo:

```
cargo run --example --plane
```

## Next Steps

* Implement cutting out part of the mesh and recomputing the smallest part needed of the navmesh
* Baking a navmesh from a scene
* Serialize baked navmesh

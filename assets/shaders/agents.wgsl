// Draws every agent as one screen-aligned quad, in a single draw call.
//
// The mesh bound to this material carries no useful data: it only exists to
// make the draw call the right length. Each quad's position, colour and size
// are pulled out of a storage buffer using the vertex index, so moving agents
// only ever costs a small per-agent buffer write instead of a full mesh
// re-upload.

#import bevy_sprite::mesh2d_view_bindings::view

// Everything is bitcast into `u32` rather than stored as floats: a float that
// carries packed colour bits can land on a NaN pattern, which the GPU is free
// to rewrite.
//
// xy: centre in world space, z: colour packed as rgba8unorm, w: half size
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read> agents: array<vec4<u32>>;

// Depth the quads are drawn at. It has to match the z of the entity holding
// this material, otherwise the navmesh underneath sorts after the agents but
// still passes the depth test, and paints over them.
const AGENT_Z: f32 = 1.0;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // The mesh is sized for the maximum number of agents seen so far; collapse
    // the quads past the current count so they cover no pixels.
    let agent_index = vertex_index / 6u;
    if agent_index >= arrayLength(&agents) {
        out.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
        out.color = vec4(0.0);
        return out;
    }

    let agent = agents[agent_index];
    let center = vec2(bitcast<f32>(agent.x), bitcast<f32>(agent.y));
    let half_size = bitcast<f32>(agent.w);

    // two triangles over the corners (-1,-1) (1,-1) (1,1) (-1,1)
    var corners = array(0u, 1u, 2u, 0u, 2u, 3u);
    let corner = corners[vertex_index % 6u];
    let offset = vec2(
        select(-1.0, 1.0, corner == 1u || corner == 2u),
        select(-1.0, 1.0, corner == 2u || corner == 3u),
    );

    out.clip_position = view.clip_from_world * vec4(center + offset * half_size, AGENT_Z, 1.0);
    out.color = unpack4x8unorm(agent.z);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// Simplified shader for Powder Game demo
// Renders world texture with camera transform

struct CameraUniform {
    position: vec2<f32>,
    zoom: f32,
    aspect: f32,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Fragment shader
@group(0) @binding(0)
var world_texture: texture_2d<f32>;
@group(0) @binding(1)
var world_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Transform screen space to NDC to world space
    let ndc = (in.tex_coords - 0.5) * 2.0;
    let ndc_flipped = vec2<f32>(ndc.x, -ndc.y); // Flip Y for world coordinates

    let world_pos = vec2<f32>(
        (ndc_flipped.x * camera.aspect / camera.zoom) + camera.position.x,
        (ndc_flipped.y / camera.zoom) + camera.position.y
    );

    // Transform world to texture space (texture is centered at camera position)
    let texture_size = 1024.0; // Must match world_size in config
    let half_size = texture_size / 2.0;

    // Texture coordinates: world origin at texture center
    let tex_coords = vec2<f32>(
        (world_pos.x + half_size) / texture_size,
        (world_pos.y + half_size) / texture_size
    );

    // Bounds check
    if tex_coords.x < 0.0 || tex_coords.x > 1.0 ||
       tex_coords.y < 0.0 || tex_coords.y > 1.0 {
        // Background color (dark)
        return vec4<f32>(0.1, 0.1, 0.15, 1.0);
    }

    let color = textureSample(world_texture, world_sampler, tex_coords);
    return color;
}

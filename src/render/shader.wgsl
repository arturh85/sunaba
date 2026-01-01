// Vertex shader

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

// Debug overlays (temperature and light)
@group(2) @binding(0)
var temp_texture: texture_2d<f32>;
@group(2) @binding(1)
var temp_sampler: sampler;
@group(2) @binding(2)
var light_texture: texture_2d<f32>;
@group(2) @binding(3)
var light_sampler: sampler;

struct OverlayUniform {
    overlay_type: u32,  // 0=none, 1=temperature, 2=light
    _padding: vec3<u32>,
};

@group(2) @binding(4)
var<uniform> overlay: OverlayUniform;

// Map temperature (in Celsius) to color gradient
fn temperature_to_color(temp: f32) -> vec3<f32> {
    // Temperature ranges and colors:
    // < 0°C: Deep blue (frozen)
    // 0-20°C: Blue to Cyan (cold)
    // 20-50°C: Cyan to Green (cool)
    // 50-100°C: Green to Yellow (warm)
    // 100-200°C: Yellow to Orange (hot)
    // 200-500°C: Orange to Red (very hot)
    // > 500°C: Bright red (extreme)

    if temp < 0.0 {
        return vec3<f32>(0.0, 0.0, 0.5); // Deep blue
    } else if temp < 20.0 {
        let t = temp / 20.0;
        return mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 1.0), t); // Blue to cyan
    } else if temp < 50.0 {
        let t = (temp - 20.0) / 30.0;
        return mix(vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), t); // Cyan to green
    } else if temp < 100.0 {
        let t = (temp - 50.0) / 50.0;
        return mix(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), t); // Green to yellow
    } else if temp < 200.0 {
        let t = (temp - 100.0) / 100.0;
        return mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 0.5, 0.0), t); // Yellow to orange
    } else if temp < 500.0 {
        let t = (temp - 200.0) / 300.0;
        return mix(vec3<f32>(1.0, 0.5, 0.0), vec3<f32>(1.0, 0.0, 0.0), t); // Orange to red
    } else {
        return vec3<f32>(1.0, 0.0, 0.0); // Bright red
    }
}

// Map light level (0-15) to color gradient
fn light_to_color(light_level: f32) -> vec3<f32> {
    // Light levels (0-15):
    // 0: Complete darkness (black)
    // 1-3: Very dark (deep purple/blue)
    // 4-7: Dim (purple to blue)
    // 8-11: Moderate (blue to cyan)
    // 12-14: Bright (cyan to white)
    // 15: Full light (bright white)

    let normalized = clamp(light_level / 15.0, 0.0, 1.0);

    if normalized < 0.2 {
        // 0-3: Black to deep purple
        let t = normalized / 0.2;
        return mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.2, 0.0, 0.4), t);
    } else if normalized < 0.5 {
        // 4-7: Deep purple to blue
        let t = (normalized - 0.2) / 0.3;
        return mix(vec3<f32>(0.2, 0.0, 0.4), vec3<f32>(0.0, 0.3, 0.8), t);
    } else if normalized < 0.75 {
        // 8-11: Blue to cyan
        let t = (normalized - 0.5) / 0.25;
        return mix(vec3<f32>(0.0, 0.3, 0.8), vec3<f32>(0.0, 0.8, 1.0), t);
    } else {
        // 12-15: Cyan to bright white
        let t = (normalized - 0.75) / 0.25;
        return mix(vec3<f32>(0.0, 0.8, 1.0), vec3<f32>(1.0, 1.0, 1.0), t);
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Transform screen space to NDC to world space
    let ndc = (in.tex_coords - 0.5) * 2.0;
    let ndc_flipped = vec2<f32>(ndc.x, -ndc.y); // Flip Y for world coordinates

    let world_pos = vec2<f32>(
        (ndc_flipped.x * camera.aspect / camera.zoom) + camera.position.x,
        (ndc_flipped.y / camera.zoom) + camera.position.y
    );

    // Transform world to texture space (512x512, centered at origin)
    let texture_size = 512.0;
    let tex_coords = vec2<f32>(
        (world_pos.x + texture_size * 0.5) / texture_size,
        (world_pos.y + texture_size * 0.5) / texture_size  // No flip - renderer writes Y-up
    );

    // Bounds check
    if tex_coords.x < 0.0 || tex_coords.x > 1.0 ||
       tex_coords.y < 0.0 || tex_coords.y > 1.0 {
        return vec4<f32>(0.1, 0.1, 0.15, 1.0); // Background color
    }

    let color = textureSample(world_texture, world_sampler, tex_coords);

    // Apply debug overlays
    if overlay.overlay_type == 1u {
        // Temperature overlay - use player-relative coordinates
        let temp_texture_size = 320.0;  // 5 chunks × 64 pixels
        let temp_tex_coords = vec2<f32>(
            (world_pos.x - camera.position.x + temp_texture_size * 0.5) / temp_texture_size,
            (world_pos.y - camera.position.y + temp_texture_size * 0.5) / temp_texture_size
        );
        let temp_value = textureSample(temp_texture, temp_sampler, temp_tex_coords).r;
        let temp_color = temperature_to_color(temp_value);

        // Blend with 40% overlay opacity
        let blended = mix(color.rgb, temp_color, 0.4);
        return vec4<f32>(blended * color.a, color.a);
    } else if overlay.overlay_type == 2u {
        // Light overlay - use player-relative coordinates
        let light_texture_size = 320.0;  // 5 chunks × 64 pixels
        let light_tex_coords = vec2<f32>(
            (world_pos.x - camera.position.x + light_texture_size * 0.5) / light_texture_size,
            (world_pos.y - camera.position.y + light_texture_size * 0.5) / light_texture_size
        );
        let light_value = textureSample(light_texture, light_sampler, light_tex_coords).r;
        let light_color = light_to_color(light_value);

        // Blend with 50% overlay opacity (slightly more visible than temperature)
        let blended = mix(color.rgb, light_color, 0.5);
        return vec4<f32>(blended * color.a, color.a);
    }

    return vec4<f32>(color.rgb * color.a, color.a);
}

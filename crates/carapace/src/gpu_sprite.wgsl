struct PxUniform {
    palette: array<vec3<f32>, 256>,
    fit_factor: vec2<f32>,
};

@group(0) @binding(0) var texture: texture_2d<u32>;
@group(0) @binding(1) var depth_texture: texture_2d<u32>;
@group(0) @binding(2) var<uniform> uniform: PxUniform;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) layer: u32,
};

@vertex fn vertex(in: VertexIn) -> VertexOut {
    return VertexOut(vec4(in.position, 0., 1.), in.uv, in.layer);
}

@fragment fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(texture));
    let coord = vec2<i32>(dims * in.uv);
    let index = textureLoad(texture, coord, 0).r;
    let depth = textureLoad(depth_texture, coord, 0).r;

    if depth > in.layer {
        discard;
    }

    if index == 0u {
        return vec4(0.);
    }

    return vec4(uniform.palette[index], 1.);
}

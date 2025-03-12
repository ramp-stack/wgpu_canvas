struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) bounds: vec4<f32>,
    @location(3) z: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) bounds: vec4<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  //if uv.x < in.bounds[0] || uv.x > in.bounds[2] ||
  //   uv.y < in.bounds[1] || uv.y > in.bounds[3] {
  //    discard;
  //}
    let coords = vec2<u32>(u32(floor(in.uv.x)), u32(floor(in.uv.y)));
    return textureLoad(t_diffuse, coords, 0);
}

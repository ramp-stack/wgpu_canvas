struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) bounds: vec4<f32>,
    @location(4) z: f32,
    @location(5) stroke: f32,
    @location(6) color: vec4<f32>,
    @location(7) rotation: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) bounds: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,
    @location(4) @interpolate(flat) color: vec4<f32>
};

@vertex
fn vs_main(shape: ShapeInput) -> VertexOutput {
    let c = cos(shape.rotation);
    let s = sin(shape.rotation);
    let center = shape.position + vec2<f32>(shape.size.x * 0.5, -shape.size.y * 0.5);
    let p = shape.position - center;
    let r = vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y) + center;

    return VertexOutput(
        vec4<f32>(r, shape.z, 1.0),
        shape.uv, shape.size, shape.bounds, shape.stroke, shape.color
    );
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < in.bounds[0] || in.uv.x > in.bounds[2] ||
       in.uv.y < in.bounds[1] || in.uv.y > in.bounds[3] {
        discard;
    }
    if in.stroke > 0 {
        if in.uv.x > in.stroke && in.uv.x < in.size.x-in.stroke &&
           in.uv.y > in.stroke && in.uv.y < in.size.y-in.stroke {
            discard;
        }
    }
    return in.color;
}

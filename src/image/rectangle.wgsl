struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) bound: vec4<f32>,
    @location(3) stroke: vec2<f32>,
    @location(4) z: f32,
}

fn bound(uv: vec2<f32>, bound: vec4<f32>) {
    if uv[0] < bound[0] || uv[0] > bound[2] ||
    uv[1] < bound[1] || uv[1] > bound[3] {
        discard;
    }
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) bound: vec4<f32>,
    @location(2) @interpolate(flat) stroke: vec2<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;
    out.bound = shape.bound;
    out.stroke = shape.stroke;

    return out;
}

fn alpha(uv: vec2<f32>, stroke: vec2<f32>) -> f32 {
    if (stroke[0] == 0.0 && stroke[1] == 0.0) ||
       (uv[0] < stroke[0] || uv[0] > 1.0-stroke[0] ||
       uv[1] < stroke[1] || uv[1] > 1.0-stroke[1]) {
        return 1.0;
    } else {
        return 0.0;
    }
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    bound(in.uv, in.bound);

    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    return vec4<f32>(color[0], color[1], color[2], color[3]*alpha(in.uv, in.stroke));
}

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

    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
    @location(5) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;
    out.bound = shape.bound;
    out.stroke = shape.stroke;

    out.color = color;

    return out;
}

fn alpha(uv: vec2<f32>, stroke: vec2<f32>) -> f32 {
    var a = 0.01;//Anti Ailiasing length

    var x = uv[0]-0.5;
    var y = uv[1]-0.5;
    var d = (x*x+y*y)*4;

    var s = 1.0;
    if stroke[0] != 0.0 && stroke[1] != 0.0 {
        var sx = 1.0 - (stroke[0] * 2.0);
        var sy = 1.0 - (stroke[1] * 2.0);
        var sd = (((x/sx)*(x/sx)) + ((y/sy)*(y/sy))) * 4;
        s = smoothstep(1.0-a, 1.0, sd);
    }

    return smoothstep(1.0, 1.0-a, d) * s;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    bound(in.uv, in.bound);
    return vec4<f32>(in.color[0], in.color[1], in.color[2], in.color[3]*alpha(in.uv, in.stroke));
}

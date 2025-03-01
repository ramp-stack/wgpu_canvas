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

    @location(3) radi: vec2<f32>,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
    @location(5) radi: vec2<f32>,
    @location(6) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;
    out.bound = shape.bound;
    out.stroke = shape.stroke;

    out.color = color;
    out.radi = radi;

    return out;
}

fn alpha(uv: vec2<f32>, stroke: vec2<f32>, radi: vec2<f32>) -> f32 {
    var x = 0.0;
    var y = 0.0;

    if uv[0] < radi[0] && uv[1] < radi[1] {
        x = (uv[0] - (radi[0])) / (radi[0] * 2.0);
        y = (uv[1] - (radi[1])) / (radi[1] * 2.0);
    } else if uv[0] > (1.0-radi[0]) && uv[1] < radi[1] {
        x = (uv[0] - (1.0-radi[0])) / (radi[0] * 2.0);
        y = (uv[1] - (radi[1])) / (radi[1] * 2.0);
    } else if uv[0] < radi[0] && uv[1] > (1.0-radi[1]) {
        x = (uv[0] - (radi[0])) / (radi[0] * 2.0);
        y = (uv[1] - (1.0-radi[1])) / (radi[1] * 2.0);
    } else if uv[0] > (1.0-radi[0]) && uv[1] > (1.0-radi[1]) {
        x = (uv[0] - (1.0-radi[0])) / (radi[0] * 2.0);
        y = (uv[1] - (1.0-radi[1])) / (radi[1] * 2.0);
    } else {
        if (stroke[0] == 0.0 && stroke[1] == 0.0) ||
           (uv[0] < stroke[0] || uv[0] > 1.0-stroke[0] ||
           uv[1] < stroke[1] || uv[1] > 1.0-stroke[1]) {
            return 1.0;
        } else {
            return 0.0;
        }
    }

    var a = 0.01;//Anti Ailiasing length

    var sd = 1.0;
    if stroke[0] != 0.0 && stroke[0] < radi[0] &&
       stroke[1] != 0.0 && stroke[1] < radi[1] {
        var sx = x / (1.0 - ((stroke[0] / radi[0])));
        var sy = y / (1.0 - ((stroke[1] / radi[1])));
        sd = smoothstep(1.0-a, 1.0, (sx*sx+sy*sy) * 4);
    }

    var d  = (x*x+y*y) * 4;
    return smoothstep(1.0, 1.0-a, d) * sd;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    bound(in.uv, in.bound);
    return vec4<f32>(in.color[0], in.color[1], in.color[2], in.color[3]*alpha(in.uv, in.stroke, in.radi));
}

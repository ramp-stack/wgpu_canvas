struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) ratio: vec2<f32>,
    @location(3) bound: vec4<f32>,
    @location(4) stroke: f32,
    @location(5) z: f32,
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
    @location(1) @interpolate(flat) ratio: vec2<f32>,
    @location(2) @interpolate(flat) bound: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,

    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
    @location(6) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;
    out.bound = shape.bound;
    out.stroke = shape.stroke;

    out.color = color;

    return out;
}

fn alpha(uv: vec2<f32>, ratio: vec2<f32>, stroke: f32) -> f32 {
    return 1.0;
  //let sx = stroke*ratio[0];
  //let sy = stroke*ratio[1];
  //if stroke == 0.0 ||
  //   (uv[0] < sx || uv[0] > 1.0-sx ||
  //   uv[1] < sy || uv[1] > 1.0-sy) {
  //    return 1.0;
  //} else {
  //    return 0.0;
  //}
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //bound(in.uv, in.bound);
    return vec4<f32>(in.color[0], in.color[1], in.color[2], in.color[3]*alpha(in.uv, in.ratio, in.stroke));
}

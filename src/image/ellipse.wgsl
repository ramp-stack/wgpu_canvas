struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) bounds: vec4<f32>,
    @location(4) z: f32,
    @location(5) stroke: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) bounds: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,
};

@vertex
fn vs_main(
    shape: ShapeInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.uv = shape.uv;

    out.size = shape.size;

    out.bounds = shape.bounds;
    out.stroke = shape.stroke;

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  //if in.uv.x < in.bounds[0] || in.uv.x > in.bounds[2] ||
  //   in.uv.y < in.bounds[1] || in.uv.y > in.bounds[3] {
  //    discard;
  //}

    let a = (in.size.x / 2.0);
    let b = (in.size.y / 2.0);

    let x = (a-(in.uv.x)) / (a - 1.0);
    let y = (b-(in.uv.y)) / (b - 1.0);

    let d = x*x+y*y;

    let p = (2.0/a);

  //var x = in.uv[0]-0.5;
  //var y = in.uv[1]-0.5;
  //var d = (x*x+y*y)*4;

  //var alpha = 1.0;
  //if d > 1.0 {alpha = 0.0;}

    var alpha = 1.0-smoothstep(1.0, 1.0+p, d);
  //if alpha == 1.0 {return vec4<f32>(1.0, 1.0, 0.0, 1.0);}
  //if alpha == 0.0 {return vec4<f32>(1.0, 0.0, 0.0, 1.0);}




  //if in.stroke > 0 {
  //    if in.uv.x > in.stroke && in.uv.x < in.size.x-in.stroke &&
  //       in.uv.y > in.stroke && in.uv.y < in.size.y-in.stroke {
  //        discard;
  //    }
  //}
    let coords = vec2<u32>(u32(floor(in.uv.x)), u32(floor(in.uv.y)));
    //let color = textureLoad(t_diffuse, coords, 0);
    let color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    return vec4<f32>(color[0], color[1], color[2], color[3]*alpha);
}

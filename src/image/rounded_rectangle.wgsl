struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) bounds: vec4<f32>,
    @location(4) z: f32,
    @location(5) stroke: f32,
    @location(6) corner_radius: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) bounds: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,
    @location(4) @interpolate(flat) corner_radius: f32,
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
    out.corner_radius = shape.corner_radius;

    return out;
}

fn alpha(uv: vec2<f32>, size: vec2<f32>, stroke: f32, cr: f32) -> f32 {
    var x = 0.0;
    var y = 0.0;

    if uv.x < cr && uv.y < cr {
        x = (cr-uv.x);
        y = (cr-uv.y);
    } else if uv.x > size[0]-cr && uv.y < cr {
        x = ((size[0]-cr)-uv.x);
        y = (cr-uv.y);
    } else if uv.x < cr && uv.y > size[1]-cr {
        x = (cr-uv.x);
        y = ((size[1]-cr)-uv.y);
    } else if uv.x > size[0]-cr && uv.y > size[1]-cr {
        x = ((size[0]-cr)-uv.x);
        y = ((size[1]-cr)-uv.y);
    } else {
        if stroke > 0 {
            if uv.x > stroke && uv.x < size.x-stroke &&
               uv.y > stroke && uv.y < size.y-stroke {
                return 0.0;
            }
            return 1.0;
        }
    }

    let a = (size.x / 2.0);
    let b = (size.y / 2.0);
    let dx = x / (cr - 1.0);
    let dy = y / (cr - 1.0);
    let d = dx*dx+dy*dy;
    let p = (2.0/cr);

    var s = 1.0;
    if stroke > 0 && stroke < cr {
        let sx = x / (cr-stroke-0.5);
        let sy = y / (cr-stroke-0.5);
        let sd = sx*sx+sy*sy;
        s = smoothstep(1.0, 1.0+p, sd);
    }

    return (1.0-smoothstep(1.0, 1.0+p, d)) * s;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < in.bounds[0] || in.uv.x > in.bounds[2] ||
       in.uv.y < in.bounds[1] || in.uv.y > in.bounds[3] {
        discard;
    }
    let coords = vec2<u32>(u32(floor(in.uv.x)), u32(floor(in.uv.y)));
    //let color = textureLoad(t_diffuse, coords, 0);
    let color = textureSample(t_diffuse, s_diffuse, vec2<f32>(in.uv.x / in.size.x, in.uv.y / in.size.y));
    let alpha = alpha(in.uv, in.size, in.stroke, in.corner_radius);
    return vec4<f32>(color[0], color[1], color[2], color[3]*alpha);
}

struct ShapeInput {
    @location(0) uv: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) bounds: vec4<f32>,
    @location(4) z: f32,
    @location(5) stroke: f32,
    @location(6) corner_radius: f32,
    @location(7) color: vec4<f32>,
    @location(8) rotation: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) bounds: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,
    @location(4) @interpolate(flat) corner_radius: f32,
    @location(5) @interpolate(flat) color: vec4<f32>,
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
        shape.uv,
        shape.size,
        shape.bounds,
        shape.stroke,
        shape.corner_radius,
        shape.color
    );
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < in.bounds[0] || in.uv.x > in.bounds[2] ||
       in.uv.y < in.bounds[1] || in.uv.y > in.bounds[3] {
        discard;
    }
    let alpha = alpha(in.uv, in.size, in.stroke, in.corner_radius);
    return vec4<f32>(in.color[0], in.color[1], in.color[2], in.color[3]*alpha);
}

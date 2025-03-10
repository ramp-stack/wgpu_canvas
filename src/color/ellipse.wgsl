struct ShapeInput {
    @location(0) coords: vec2<f32>,
    @location(1) position: vec2<f32>,
    @location(2) bound: vec4<f32>,
    @location(3) stroke: f32,
    @location(4) z: f32,
}

fn bound(coords: vec2<f32>, bound: vec4<f32>) {
    if coords.x < bound[0] || coords.x > bound[2] ||
    coords.y < bound[1] || coords.y > bound[3] {
        discard;
    }
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) coords: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) bound: vec4<f32>,
    @location(3) @interpolate(flat) stroke: f32,

    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(
    shape: ShapeInput,
    @location(5) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(shape.position, shape.z, 1.0);
    out.coords = shape.coords;
    out.size = vec2<f32>(abs(shape.coords.x), abs(shape.coords.y));
    out.bound = shape.bound;
    out.stroke = shape.stroke;

    out.color = color;

    return out;
}

fn alpha(coords: vec2<f32>, size: vec2<f32>, str: f32) -> f32 {
    //var r = length(coords)*2.0;
    //var t = atan2(coords.y, coords.x); //Angle


    var x = coords.x;
    var y = coords.y;

    var a = size.x/4.0;
    var b = size.y/4.0;


  //var c = sqrt((a*a)-(b*b));

  //var O = atan((a*tan(t))/b);//Target Angle

  //var PI = 3.14159;


  //var stroke = -40.0;
  //for (var t: f32 = -PI; t < PI; t += 0.1) {
  //    var c = vec2<f32>(50.0*cos(3.0*t), 50.0*sin(2.0*t));
  //  //var c = vec2<f32>(
  //  //    cos(t)*(
  //  //        (
  //  //            (b*stroke) /
  //  //            sqrt(((a*a)*(sin(t)*sin(t)) + ((b*b)*(cos(t)*cos(t)))))
  //  //        ) + a
  //  //    ),
  //  //    sin(t)*(
  //  //        (
  //  //            (a*stroke) /
  //  //            sqrt(((a*a)*(sin(t)*sin(t)) + ((b*b)*(cos(t)*cos(t)))))
  //  //        ) + b
  //  //    )
  //  //);

  //    if coords.x < c.x+0.5 && coords.x > c.x-0.5 && coords.y < c.y+0.5 && coords.y > c.y-0.5{
  //        return 1.0;
  //    }
  //}


////if coords.x < 0.0 {
////    O = -O;
////}


//  var t_coords = vec2<f32>(
//      a*cos(O),
//      b*sin(2.0*O)
//  );

//  if coords.x < 0.0 {t_coords = -t_coords;} //Mirror for negative x

//  if coords.x < t_coords.x+0.5 && coords.x > t_coords.x-0.5 {
//      return 1.0;
//  }

//  if coords.y < t_coords.y+0.5 && coords.y > t_coords.y-0.5 {
//      return 0.9;
//  }

////if length(coords) < length(t_coords) {
////    return 1.0;
////}

//  var test = 0.78539*1.0;
//  if O < test+0.05 && O > test-0.05 {return 0.8;}
//  if t < test+0.05 && t > test-0.05 {return 1.0;}

    if floor(abs(coords[0])) % 10 == 0.0 || floor(abs(coords[1])) % 10 == 0.0 {return 0.5;}

    return 0.2;


  
  //var sd = distance(pos, vec2<f32>(sx, sy))*2.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    bound(in.coords, in.bound);
    return vec4<f32>(
        in.color[0], in.color[1], in.color[2],
        in.color[3]*alpha(in.coords, in.size, in.stroke)
    );
}

use wgpu::{PipelineCompilationOptions, BindGroupLayoutDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, TextureViewDimension, BindGroupLayoutEntry, DepthStencilState, TextureSampleType, MultisampleState, BindGroupLayout, RenderPipeline, PrimitiveState, FragmentState, TextureFormat, ShaderStages, BufferUsages, IndexFormat, VertexState, BindingType, RenderPass, BindGroup, Device, Queue, VertexBufferLayout, VertexStepMode, BufferAddress};
use wgpu_dyn_buffer::{DynamicBufferDescriptor, DynamicBuffer};

use super::Area;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ShapeVertex {
    pub uv: [f32; 2],
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub bounds: [f32; 4],
    pub z_index: f32,
    pub stroke: f32
}

impl ShapeVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 6] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32, 5 => Float32];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn new(width: u32, height: u32, area: Area, stroke: u32, size: (u32, u32)) -> [ShapeVertex; 4] {
        let w = |x: f32| ((x / width as f32) * 2.0) - 1.0;
        let h = |y: f32| 1.0 - ((y / height as f32) * 2.0);

        let x = w(area.offset.0 as f32);
        let y = h(area.offset.1 as f32);
        let x2 = w((area.offset.0+size.0) as f32);
        let y2 = h((area.offset.1+size.1) as f32);

        let stroke = stroke.min(size.0.min(size.1)) as f32;

        let size = [size.0 as f32, size.1 as f32];

        let bx = area.bounds.0 as f32 - area.offset.0 as f32;
        let by = area.bounds.1 as f32 - area.offset.1 as f32;
        let bx2 = (area.bounds.0 as f32 - area.offset.0 as f32) + area.bounds.2 as f32;
        let by2 = (area.bounds.1 as f32 - area.offset.1 as f32) + area.bounds.3 as f32;
        let bounds = [bx, by, bx2, by2];

        let z_index = area.z_index as f32 / u16::MAX as f32;

        [
            ShapeVertex{uv: [0.0, 0.0], position: [x, y], size, bounds, z_index, stroke},
            ShapeVertex{uv: [size[0], 0.0], position: [x2, y], size, bounds, z_index, stroke},
            ShapeVertex{uv: [0.0, size[1]], position: [x, y2], size, bounds, z_index, stroke},
            ShapeVertex{uv: [size[0], size[1]], position: [x2, y2], size, bounds, z_index, stroke}
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct RoundedRectangleVertex {
    pub shape: ShapeVertex,
    pub corner_radius: f32,
}

impl RoundedRectangleVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 7] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32, 5 => Float32, 6 => Float32];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn new(width: u32, height: u32, area: Area, stroke: u32, size: (u32, u32), corner_radius: u32) -> [RoundedRectangleVertex; 4] {
        ShapeVertex::new(width, height, area, stroke, size).into_iter().map(|shape|
            RoundedRectangleVertex{shape, corner_radius: corner_radius as f32}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}

//  #[derive(Clone, Debug, Copy)]
//  pub struct Ellipse(u32, (u32, u32));

//  impl Ellipse {
//      pub fn new(self, width: u32, height: u32, area: Area) -> [ShapeVertex; 4] {
//          ShapeVertex::new(width, height, area, self.0, self.1)
//      }
//  }

//  #[derive(Clone, Debug, Copy)]
//  pub struct Rectangle(u32, (u32, u32));

//  impl Rectangle {
//      pub fn new(self, width: u32, height: u32, area: Area) -> [ShapeVertex; 4] {
//          ShapeVertex::new(width, height, area, self.0, self.1)
//      }
//  }

//  #[derive(Clone, Debug, Copy)]
//  pub struct RoundedRectangle(u32, (u32, u32), u32);

//  impl RoundedRectangle {
//      pub fn new(self, width: u32, height: u32, area: Area) -> [RoundedRectangleVertex; 4] {
//          RoundedRectangleVertex::new(width, height, area, self.0, self.1, self.2)
//      }
//  }







//  use fast_image_resize::{Resizer, ResizeOptions, ResizeAlg, FilterType};

//  use std::time::Instant;

//  use image::{GenericImage, DynamicImage, RgbaImage, Rgba};
//  use super::image::{Image};

//  const EMPTY: image::Rgba<u8> = image::Rgba([0, 0, 0, 0]);

//  pub struct Parametric(u32, u32);

//  impl Parametric {
//      pub fn build(self, mut iter: impl FnMut(f64, f64, f64, Box<dyn Fn(f64, f64) -> (u32, u32)>)) {
//          let hsize = (self.0 as f64/2.0, self.1 as f64/2.0);
//          let steps = (self.0+self.1)*16;
//          let increment = std::f64::consts::PI/(steps as f64 / 2.0);

//          let a = hsize.0-0.5;
//          let b = hsize.1-0.5;

//          let to_coord = move |x: f64, y: f64| -> (u32, u32) {
//              let x = ((a)+x).round();
//              let y = ((b)+y).round();
//              ((x as u32).min(self.0-1), (y as u32).min(self.1-1))
//          };

//          for i in 0..steps {
//              let t = -std::f64::consts::PI + (increment*i as f64);
//              iter(a, b, t, Box::new(to_coord))
//          }
//      }
//  }

//  #[derive(Clone, Copy, Debug, Hash)]
//  pub struct Ellipse{
//      stroke: u32,
//      size: (u32, u32)
//  }

//  impl Ellipse {
//      pub fn color(self, pixel: Rgba<u8>) -> RgbaImage {
//          let time = Instant::now();
//          println!("margin: {}", time.elapsed().as_millis());
//          let alpha = 4;
//          let size = (self.size.0*alpha, self.size.1*alpha);
//          let stroke = self.stroke.min(self.size.0/2)*alpha;

//          let mut image = image::DynamicImage::new(size.0, size.1, image::ColorType::Rgba8);

//          println!("start: {}", time.elapsed().as_millis());
//        //for x in 0..size.0 {
//        //    for y in 0..size.1 {
//        //        image.put_pixel(x, y, EMPTY);
//        //    }
//        //}
//        //println!("filled: {}", time.elapsed().as_millis());

//          for x in 0..stroke {
//              for y in 0..stroke {
//                  image.put_pixel(x, y, pixel);
//                  image.put_pixel(x, size.1-y-1, pixel);
//                  image.put_pixel(size.0-x-1, size.1-y-1, pixel);
//                  image.put_pixel(size.0-x-1, y, pixel);
//              }
//          }
//          println!("corners: {}", time.elapsed().as_millis());

//          if stroke > 0 {
//              let k = -(stroke as f64);
//              Parametric(size.0, size.1).build(
//                  |a: f64, b: f64, t: f64, to_coord: Box<dyn Fn(f64, f64) -> (u32, u32)>| {
//                  let x = (a + (b*k / (((a*a)*(t.sin()*t.sin()))+((b*b)*(t.cos()*t.cos()))).sqrt())) * t.cos();
//                  let y = (b + (a*k / (((a*a)*(t.sin()*t.sin()))+((b*b)*(t.cos()*t.cos()))).sqrt())) * t.sin();

//                  let (x, y) = to_coord(x, y);
//                  if (x as f64) < a {
//                      for i in 0..x+1 {
//                          image.put_pixel(i, y, pixel);
//                      }
//                  } else {
//                      for i in x..1+(a+a) as u32 {
//                          image.put_pixel(i, y, pixel);
//                      }
//                  }

//                  if (y as f64) < b {
//                      for i in 0..y+1 {
//                          image.put_pixel(x, i, pixel);
//                      }
//                  } else {
//                      for i in y..1+(b+b) as u32 {
//                          image.put_pixel(x, i, pixel);
//                      }
//                  }
//              });
//              println!("stroked: {}", time.elapsed().as_millis());

//              Parametric(size.0, size.1).build(
//                  |a: f64, b: f64, t: f64, to_coord: Box<dyn Fn(f64, f64) -> (u32, u32)>| {
//                  let x = (a) * t.cos();
//                  let y = (b) * t.sin();

//                  let (x, y) = to_coord(x, y);

//                  if (x as f64) < a {
//                      for i in 0..x+1 {
//                          image.put_pixel(i, y, EMPTY);
//                      }
//                  } else {
//                      for i in x..1+(a+a) as u32 {
//                          image.put_pixel(i, y, EMPTY);
//                      }
//                  }
//              });
//          } else {
//              Parametric(size.0, size.1).build(
//                  |a: f64, b: f64, t: f64, to_coord: Box<dyn Fn(f64, f64) -> (u32, u32)>| {
//                  let x = (a) * t.cos();
//                  let y = (b) * t.sin();

//                  let (x, y) = to_coord(x, y);

//                  if (x as f64) < a {
//                      for i in x..(a.ceil() as u32) {
//                          image.put_pixel(i, y, pixel);
//                      }
//                  } else {
//                      for i in (a.ceil() as u32)..x {
//                          image.put_pixel(i, y, pixel);
//                      }
//                  }
//              });
//          }
//          println!("ellipse: {}", time.elapsed().as_millis());


//          let mut dst_image = image::DynamicImage::new(
//              self.size.0, self.size.1, image::ColorType::Rgba8
//          );
//          let mut resizer = Resizer::new();
//          resizer.resize(&image, &mut dst_image,
//              &ResizeOptions::new().resize_alg(ResizeAlg::SuperSampling(FilterType::Lanczos3, 12))
//          ).unwrap();
//          println!("resized: {}", time.elapsed().as_millis());

//          dst_image.into()
//      }
//      pub fn image(self, image: Image) -> RgbaImage {todo!()}
//  }

//  #[derive(Clone, Copy, Debug, Hash)]
//  pub struct Rectangle{
//      stroke: u32,
//      size: (u32, u32)
//  }

//  impl Rectangle {
//      pub fn color(self, color: Rgba<u8>) -> RgbaImage {todo!()}
//      pub fn image(self, image: Image) -> RgbaImage {
//          let mut dst_image = image::DynamicImage::new(
//              self.size.0, self.size.1, image::ColorType::Rgba8
//          );
//          let mut resizer = Resizer::new();
//          resizer.resize(&DynamicImage::from(RgbaImage::clone(&image)), &mut dst_image,
//              &ResizeOptions::new().resize_alg(ResizeAlg::SuperSampling(FilterType::Lanczos3, 12))
//          ).unwrap();

//          dst_image.into()
//      }
//  }

//  #[derive(Clone, Copy, Debug, Hash)]
//  pub struct RoundedRectangle{
//      stroke: u32,
//      size: (u32, u32),
//      corner_radius: u32
//  }

//  impl RoundedRectangle {
//      pub fn color(self, color: Rgba<u8>) -> RgbaImage {todo!()}
//      pub fn image(self, image: Image) -> RgbaImage {
//          todo!()
//        //let mut image = image::DynamicImage::new(self.size.0, self.size.1, image::ColorType::Rgba8);

//        //for x in 0..self.size.0 {
//        //    for y in 0..self.size.1 {
//        //        image.put_pixel(x, y, pixel(x, y));
//        //    }
//        //}

//        //let mut dst_image = image::DynamicImage::new(
//        //    self.size.0, self.size.1, image::ColorType::Rgba8
//        //);
//        //let mut resizer = Resizer::new();
//        //resizer.resize(&image, &mut dst_image,
//        //    &ResizeOptions::new().resize_alg(ResizeAlg::SuperSampling(FilterType::Lanczos3, 12))
//        //).unwrap();

//        //RawImage(dst_image.into_bytes(), self.size.0, self.size.1)
//      }
//  }


//  pub enum Shape {
//      Ellipse(u32, (u32, u32)),
//      Rectangle(u32, (u32, u32)),
//      RoundedRectangle(u32, (u32, u32), u32),
//  }

//  impl Shape {
//      pub(crate) fn color(self, color: (u8, u8, u8, u8)) -> RgbaImage {
//          match self {
//              Shape::Ellipse(stroke, size) => Ellipse{stroke, size}.color(
//                  image::Rgba([color.0, color.1, color.2, color.3])
//              ),
//              Shape::Rectangle(stroke, size) => Rectangle{stroke, size}.color(
//                  image::Rgba([color.0, color.1, color.2, color.3])
//              ),
//              Shape::RoundedRectangle(stroke, size, corner_radius) =>
//                  RoundedRectangle{stroke, size, corner_radius}.color(
//                  image::Rgba([color.0, color.1, color.2, color.3])
//              )
//          }
//      }

//      pub(crate) fn image(self, image: Image) -> RgbaImage {
//          match self {
//              Shape::Ellipse(stroke, size) => Ellipse{stroke, size}.image(image),
//              Shape::Rectangle(stroke, size) => Rectangle{stroke, size}.image(image),
//              Shape::RoundedRectangle(stroke, size, corner_radius) =>
//                  RoundedRectangle{stroke, size, corner_radius}.image(image)
//          }
//      }
//  }


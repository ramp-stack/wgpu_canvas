#![allow(clippy::multiple_bound_locations)]

use wgpu::{VertexBufferLayout, VertexStepMode, BufferAddress, VertexAttribute, VertexFormat};

use crate::{RgbaImage, Area, Color};
use crate::shape::Shape;
use std::sync::Arc;

pub trait Vertex: std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable{
    fn attributes() -> Vec<VertexFormat> where Self: Sized;

    fn layout() -> VertexBufferLayout<'static> where Self: Sized {
        let mut offset = 0;
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::attributes().into_iter().enumerate().map(|(i, a)| {
                let va = VertexAttribute{
                    format: a,
                    offset,
                    shader_location: i as u32
                };
                offset += a.size();
                va
            }).collect::<Vec<_>>().leak(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub uv: [f32; 2],
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub bounds: [f32; 4],
    pub z_index: f32,
    pub stroke: f32
}

impl Vertex for ShapeVertex {
    fn attributes() -> Vec<VertexFormat> {
        vec![
            VertexFormat::Float32x2, VertexFormat::Float32x2, VertexFormat::Float32x2,
            VertexFormat::Float32x4, VertexFormat::Float32, VertexFormat::Float32
        ]
    }
}

impl ShapeVertex {
    pub fn transform_point(width: f32, height: f32, p: [f32; 2]) -> [f32; 2] {
        let w = |x: f32| ((x / width) * 2.0) - 1.0;
        let h = |y: f32| 1.0 - ((y / height) * 2.0);
        [w(p[0]), h(p[1])]
    }
    pub fn transform(width: f32, height: f32, p: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
        [
            Self::transform_point(width, height, p[0]),
            Self::transform_point(width, height, p[1]),
            Self::transform_point(width, height, p[2]),
            Self::transform_point(width, height, p[3])
        ]
    }

    pub fn new(width: f32, height: f32, z: u16, area: Area, shape: Shape) -> [ShapeVertex; 4] {
        let op = shape.positions(area.offset);
        let positions = Self::transform(width, height, op);
        let size = shape.wh();

        let stroke = shape.stroke();

        let bounds = area.bounds.unwrap_or((0.0, 0.0, width, height));
        let bx = bounds.0 - area.offset.0;
        let by = bounds.1 - area.offset.1;
        let bx2 = bx + bounds.2;
        let by2 = by + bounds.3;
        let [bx, by] = Self::transform_point(width, height, [bx, by]);
        let [bx2, by2] = Self::transform_point(width, height, [bx2, by2]);
        let bounds = [bx, by, bx2, by2];

        let z_index = z as f32 / u16::MAX as f32;

        [
            ShapeVertex{uv: [0.0, 0.0], position: positions[0], size, bounds, z_index, stroke},
            ShapeVertex{uv: [size[0], 0.0], position: positions[1], size, bounds, z_index, stroke},
            ShapeVertex{uv: [0.0, size[1]], position: positions[2], size, bounds, z_index, stroke},
            ShapeVertex{uv: [size[0], size[1]], position: positions[3], size, bounds, z_index, stroke}
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RoundedRectangleVertex {
    pub shape: ShapeVertex,
    pub corner_radius: f32,
}

impl Vertex for RoundedRectangleVertex {
    fn attributes() -> Vec<VertexFormat> {
        [ShapeVertex::attributes(), vec![VertexFormat::Float32]].concat()
    }
}

impl RoundedRectangleVertex {
    pub fn new(width: f32, height: f32, z: u16, area: Area, shape: Shape, corner_radius: f32) -> [RoundedRectangleVertex; 4] {
        ShapeVertex::new(width, height, z, area, shape).into_iter().map(|shape|
            RoundedRectangleVertex{shape, corner_radius}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}


#[repr(packed, C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex<V: Vertex = ShapeVertex> {
    pub shape: V,
    pub color: [f32; 4]
}

impl<V: Vertex> Vertex for ColorVertex<V> {
    fn attributes() -> Vec<VertexFormat> {
        [V::attributes(), vec![VertexFormat::Float32x4]].concat()
    }
}

impl<V: Vertex> ColorVertex<V> {
    pub fn new(shape: [V; 4], color: Color) -> [ColorVertex<V>; 4] {
        let c = |f: u8| if f == 0 {0.0} else {(((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4)};
        let color = [c(color.0), c(color.1), c(color.2), c(color.3)];
        shape.into_iter().map(|shape|
            ColorVertex{shape, color}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}

#[repr(packed, C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex<V: Vertex = ShapeVertex> {
    pub color: ColorVertex<V>,
    pub texture: [f32; 2]
}

impl<V: Vertex> Vertex for ImageVertex<V> {
    fn attributes() -> Vec<VertexFormat> {
        [ColorVertex::<V>::attributes(), vec![VertexFormat::Float32x2]].concat()
    }
}

impl<V: Vertex> ImageVertex<V> {
    pub fn new(shape: [V; 4], image: &Arc<RgbaImage>, size: (f32, f32), color: Option<Color>) -> [ImageVertex<V>; 4] {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 1.0;

        let wi = image.width() as f32;
        let hi = image.height() as f32;
        let ws = size.0;
        let hs = size.1;

        let wr = ws / wi;
        let hr = hs / hi;

        if hr > wr {
            let d = (1.0-(wr / hr)) / 2.0;
            x = d;
            x2 = 1.0-d;
        } else {
            let d = (1.0-(hr / wr)) / 2.0;
            y = d;
            y2 = 1.0-d;
        }

        let color = ColorVertex::new(shape, color.unwrap_or_default());

        [
            ImageVertex{color: color[0], texture: [x, y]},
            ImageVertex{color: color[1], texture: [x2, y]},
            ImageVertex{color: color[2], texture: [x, y2]},
            ImageVertex{color: color[3], texture: [x2, y2]},
        ]
    }
}

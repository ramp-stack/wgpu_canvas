#![allow(clippy::multiple_bound_locations)]

use wgpu::{VertexBufferLayout, VertexStepMode, BufferAddress, VertexAttribute, VertexFormat};

use super::Area;
use crate::image::Image;

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }
}

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
    pub fn new(width: u32, height: u32, area: Area, stroke: u32, size: (u32, u32), corner_radius: u32) -> [RoundedRectangleVertex; 4] {
        ShapeVertex::new(width, height, area, stroke, size).into_iter().map(|shape|
            RoundedRectangleVertex{shape, corner_radius: corner_radius as f32}
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
        shape.into_iter().map(|shape|
            ColorVertex{shape, color: color.color()}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}

#[repr(packed, C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex<V: Vertex = ShapeVertex> {
    pub shape: V,
    pub texture: [f32; 2]
}

impl<V: Vertex> Vertex for ImageVertex<V> {
    fn attributes() -> Vec<VertexFormat> {
        [V::attributes(), vec![VertexFormat::Float32x2]].concat()
    }
}

impl<V: Vertex> ImageVertex<V> {
    pub fn new(shape: [V; 4], image: &Image, size: (u32, u32)) -> [ImageVertex<V>; 4] {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 1.0;

        let wi = image.size().0 as f32;
        let hi = image.size().1 as f32;
        let ws = size.0 as f32;
        let hs = size.1 as f32;

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

        [
            ImageVertex{shape: shape[0], texture: [x, y]},
            ImageVertex{shape: shape[1], texture: [x2, y]},
            ImageVertex{shape: shape[2], texture: [x, y2]},
            ImageVertex{shape: shape[3], texture: [x2, y2]},
        ]
    }
}

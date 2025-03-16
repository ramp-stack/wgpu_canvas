use wgpu::{VertexBufferLayout, VertexStepMode, BufferAddress};

use super::Area;

pub type Color = (u8, u8, u8, u8);

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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ColorVertex {
    pub shape: ShapeVertex,
    pub color: [f32; 4]
}

impl ColorVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 7] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32, 5 => Float32, 6 => Float32x4];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn new(width: u32, height: u32, area: Area, stroke: u32, size: (u32, u32), color: Color) -> [ColorVertex; 4] {
        ShapeVertex::new(width, height, area, stroke, size).into_iter().map(|shape|
            ColorVertex{shape, color: [color.0 as f32, color.1 as f32, color.2 as f32, color.3 as f32]}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct RoundedRectangleColorVertex {
    pub shape: RoundedRectangleVertex,
    pub color: [f32; 4],
}

impl RoundedRectangleColorVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 8] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32, 5 => Float32, 6 => Float32, 7 => Float32x4];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn new(width: u32, height: u32, area: Area, stroke: u32, size: (u32, u32), corner_radius: u32, color: Color) -> [RoundedRectangleColorVertex; 4] {
        RoundedRectangleVertex::new(width, height, area, stroke, size, corner_radius).into_iter().map(|shape|
            RoundedRectangleColorVertex{shape, color: [color.0 as f32, color.1 as f32, color.2 as f32, color.3 as f32]}
        ).collect::<Vec<_>>().try_into().unwrap()
    }
}

use wgpu::{VertexBufferLayout, VertexStepMode, BufferAddress, VertexAttribute, VertexFormat};

use super::Area;

pub(crate) trait Vertex: std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable{
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
pub(crate) struct ShapeVertex {
    pub uv: [f32; 2],
    pub position: [f32; 2],
    pub bound: [f32; 4],
    pub stroke: [f32; 2],
    pub z_index: f32
}

impl Vertex for ShapeVertex {
    fn attributes() -> Vec<VertexFormat> {
        vec![
            VertexFormat::Float32x2, VertexFormat::Float32x2, VertexFormat::Float32x4, VertexFormat::Float32x2, VertexFormat::Float32
        ]
    }
}

pub(crate) trait Shape: Clone + std::fmt::Debug {
    type Vertex: Vertex;
    fn build(self, width: u32, height: u32, area: Area, l: u16) -> ([Self::Vertex; 4], [u16; 6]);
}

#[derive(Clone, Copy, Debug)]
pub struct GenericShape {
    pub stroke: (u32, u32),
    pub size: (u32, u32)
}

impl Shape for GenericShape {
    type Vertex = ShapeVertex;

    fn build(self, width: u32, height: u32, area: Area, l: u16) -> ([Self::Vertex; 4], [u16; 6]) {
        let s = |s: u32, size: u32| {
            if s == 0 { 0.0 } else {(s as f32).min(size as f32 / 2.0) / size as f32}
        };
        let w = |x: u32| ((x as f32 / width as f32) * 2.0) - 1.0;
        let h = |y: u32| 1.0 - ((y as f32 / height as f32) * 2.0);
        let p = |b: f32, s: f32, e: f32| (s-b)/(s-e);

        let x = w(area.offset.0);
        let y = h(area.offset.1);
        let x2 = w(area.offset.0+self.size.0);
        let y2 = h(area.offset.1+self.size.1);

        let stroke = [s(self.stroke.0, self.size.0), s(self.stroke.1, self.size.1)];
        let z_index = area.z_index as f32 / u16::MAX as f32;

        let bx = p(w(area.bounds.0), x, x2);
        let by = p(h(area.bounds.1), y, y2);
        let bx2 = p(w(area.bounds.0+area.bounds.2), x, x2);
        let by2 = p(h(area.bounds.1+area.bounds.3), y, y2);
        let bound = [bx, by, bx2, by2];

        (
            [
                ShapeVertex{
                    uv: [0.0, 0.0],
                    position: [x, y],
                    bound,
                    stroke,
                    z_index
                },
                ShapeVertex{
                    uv: [1.0, 0.0],
                    position: [x2, y],
                    bound,
                    stroke,
                    z_index
                },
                ShapeVertex{
                    uv: [0.0, 1.0],
                    position: [x, y2],
                    bound,
                    stroke,
                    z_index
                },
                ShapeVertex{
                    uv: [1.0, 1.0],
                    position: [x2, y2],
                    bound,
                    stroke,
                    z_index
                }
            ],
            [l, l+1, l+2, l+1, l+2, l+3]
        )
    }
}

pub type Ellipse = GenericShape;
pub type Rectangle = GenericShape;

#[derive(Clone, Copy, Debug)]
pub struct RoundedRectangle {
    pub shape: GenericShape,
    pub corner_radius: u32
}

impl Shape for RoundedRectangle {
    type Vertex = RoundedRectangleVertex;

    fn build(self, width: u32, height: u32, a: Area, l: u16) -> ([Self::Vertex; 4], [u16; 6]) {
        let size = self.shape.size;
        let cr = (self.corner_radius as f32).min(size.0.min(size.1) as f32 / 2.0);
        let rw = cr * (1.0 / size.0 as f32);
        let rh = cr * (1.0 / size.1 as f32);

        let (v, i) = self.shape.build(width, height, a, l);
        (v.into_iter().map(|shape|
            RoundedRectangleVertex{shape, radi: [rw, rh]}
        ).collect::<Vec<_>>().try_into().unwrap(), i)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct RoundedRectangleVertex {
    shape: ShapeVertex,
    radi: [f32; 2]
}

impl Vertex for RoundedRectangleVertex {
    fn attributes() -> Vec<wgpu::VertexFormat> {
        [ShapeVertex::attributes(), vec![VertexFormat::Float32x2]].concat()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ShapeType {
    Ellipse(Ellipse),
    Rectangle(Rectangle),
    RoundedRectangle(RoundedRectangle)
}

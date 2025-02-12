use wgpu_cyat::cyat::{ShapeBuilder, Attributes};
use wgpu_cyat::cyat;

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum DrawCommand {
    QuadraticBezierTo(u32, u32, u32, u32), //x, y, ctrlx, ctrly
    CubicBezierTo(u32, u32, u32, u32, u32, u32), //x, y, ctrlx, ctrly, ctrlx2, ctrly2
    LineTo(u32, u32), //x, y
}

#[derive(Clone, Debug)]
pub enum Shape {
    Draw(u32, u32, Vec<DrawCommand>),
    RoundedRectangle(u32, u32, u32),
    Rectangle(u32, u32),
    Circle(u32)
}

impl Shape {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Shape::Draw(_x, _y, _c) => {
                todo!()
                //let (min, max) = c.iter().copied().fold(((0, 0), (0, 0)), |((lx, ly), (hx, y)), c|
            },
            Shape::RoundedRectangle(w, h, _) => (*w, *h),
            Shape::Rectangle(w, h) => (*w, *h),
            Shape::Circle(r) => (r*2, r*2)
        }
    }
}

pub struct CanvasShapeBuilder<A: Attributes> {
    shape: Shape,
    pub stroke_width: Option<u32>,
    attributes: A,
    offset: (u32, u32),
    width: f32,
    height: f32
}

impl<A: Attributes> CanvasShapeBuilder<A> {
    pub const TOLERANCE: f32 = 0.0005;

    pub fn new(
        shape: Shape,
        stroke_width: Option<u32>,
        attributes: A,
        offset: (u32, u32),
        width: f32,
        height: f32
    ) -> Self {
        CanvasShapeBuilder {
            shape, stroke_width, attributes, offset, width, height
        }
    }

    fn lx(&self, x: u32) -> f32 {x as f32 / self.width}
    fn ly(&self, y: u32) -> f32 {y as f32 / self.height}

    fn point(&self, x: u32, y: u32) -> (f32, f32) {
        (-1.0+self.lx(self.offset.0+x), 1.0-self.ly(self.offset.1+y))
    }

    pub fn build_shape(self) -> cyat::Shape<A> {
        let a = self.attributes;
        match self.shape {
            Shape::Draw(x, y, ref commands) => {
                let p = self.point(x, y);
                let commands = commands.iter().copied().map(|command| match command {
                    DrawCommand::QuadraticBezierTo(x, y, ctrlx, ctrly) => {
                        let tp = self.point(x, y);
                        let cp = self.point(ctrlx, ctrly);
                        cyat::DrawCommand::QuadraticBezierTo(a, tp.0, tp.1, cp.0, cp.1)
                    },
                    DrawCommand::CubicBezierTo(x, y, ctrlx, ctrly, ctrlx2, ctrly2) => {
                        let tp = self.point(x, y);
                        let cp = self.point(ctrlx, ctrly);
                        let cp2 = self.point(ctrlx2, ctrly2);
                        cyat::DrawCommand::CubicBezierTo(a, tp.0, tp.1, cp.0, cp.1, cp2.0, cp2.1)
                    },
                    DrawCommand::LineTo( x, y) => {
                        let tp = self.point(x, y);
                        cyat::DrawCommand::LineTo(a, tp.0, tp.1)
                    }
                }).collect::<Vec<_>>();
                cyat::Shape::Draw(a, p.0, p.1, commands)
            },
            Shape::RoundedRectangle(x, y, r) => {
                let p = self.point(0, 0);
                let p2 = self.point(x, y);
                cyat::Shape::RoundedRectangle(a, p.0, p.1, p2.0, p2.1, self.lx(r), self.ly(r))
            },
            Shape::Rectangle(x, y) => {
                let p = self.point(0, 0);
                let p2 = self.point(x, y);
                cyat::Shape::Rectangle(a, p.0, p.1, p2.0, p2.1)
            },
            Shape::Circle(r) => {
                let p = self.point(r, r);
                cyat::Shape::Ellipse(a, p.0, p.1, self.lx(r), self.ly(r))
            }
        }
    }
}

impl<A: Attributes> From<CanvasShapeBuilder<A>> for ShapeBuilder<A> {
    fn from(c: CanvasShapeBuilder<A>) -> Self {
        let stroke_width = c.stroke_width;
        ShapeBuilder::new(c.build_shape(), stroke_width, CanvasShapeBuilder::<A>::TOLERANCE)
    }
}

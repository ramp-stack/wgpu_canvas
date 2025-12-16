#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Ellipse(f32, (f32, f32), f32),
    Rectangle(f32, (f32, f32), f32),
    RoundedRectangle(f32, (f32, f32), f32, f32),
}
impl Shape {
    pub fn stroke(&self) -> f32 {match self {
        Shape::Ellipse(s, (w, h), _) => s.min(w.min(*h)),
        Shape::Rectangle(s, (w, h), _) => s.min(w.min(*h)),
        Shape::RoundedRectangle(s, (w, h), _, _) => s.min(w.min(*h)),
    }}

    pub fn angle(&self) -> f32 {match self {
        Shape::Ellipse(_, _, a) => *a,
        Shape::Rectangle(_, _, a) => *a,
        Shape::RoundedRectangle(_, _, a, _) => *a,
    }}

    pub fn wh(&self) -> [f32; 2] {match self {
        Shape::Ellipse(_, (w, h), _) => [*w, *h],
        Shape::Rectangle(_, (w, h), _) => [*w, *h],
        Shape::RoundedRectangle(_, (w, h), _, _) => [*w, *h],
    }}

    pub fn size(&self) -> (f32, f32) {
        let theta = self.angle().to_radians();
        let cos = theta.cos().abs();
        let sin = theta.sin().abs();
        match self {
            Shape::Ellipse(_, (w, h), _) => {
                let rx = w * 0.5;
                let ry = h * 0.5;
                let rx2 = rx * rx;
                let ry2 = ry * ry;

                // Width and height of the bounding box
                let bb_width  = 2.0 * (rx2 * cos * cos + ry2 * sin * sin).sqrt();
                let bb_height = 2.0 * (rx2 * sin * sin + ry2 * cos * cos).sqrt();

                (bb_width, bb_height)
            }
            Shape::Rectangle(_, (w, h), _) | Shape::RoundedRectangle(_, (w, h), _, _) => {
                let half_w_proj = (w * 0.5) * cos + (h * 0.5) * sin;
                let half_h_proj = (w * 0.5) * sin + (h * 0.5) * cos;

                let bb_width = 2.0 * half_w_proj;
                let bb_height = 2.0 * half_h_proj;
                (bb_width, bb_height)
            },
        }
    }

    pub(crate) fn positions(&self, offset: (f32, f32)) -> [[f32; 2]; 4] {
        let theta = self.angle().to_radians();
        let cos = theta.cos();
        let sin = theta.sin();
        let [w, h] = self.wh();
        let hw = w*0.5;
        let hh = h*0.5;
        let cx = offset.0+hw;
        let cy = offset.1+hh;

        let rotate = |px: f32, py: f32| -> [f32; 2] {
            let dx = px - cx;
            let dy = py - cy;
            [
                cx + dx * cos - dy * sin,
                cy + dx * sin + dy * cos,
            ]
        };

        let mut positions = [
            rotate(offset.0, offset.1), rotate(offset.0+w, offset.1), rotate(offset.0, offset.1+h), rotate(offset.0+w, offset.1+h)
        ];

        let shift = match self {
            Shape::Rectangle(_, _, _) | Shape::RoundedRectangle(_, _, _, _) => {
                let min = positions.into_iter().reduce(|r, i| [r[0].min(i[0]), r[1].min(i[1])]).unwrap();
                [(offset.0-min[0]), (offset.1-min[1])]
            },
            Shape::Ellipse(_, _, _) => {
                let rx2 = hw * hw;
                let ry2 = hh * hh;

                let half_w = (rx2 * cos * cos + ry2 * sin * sin).sqrt();
                let half_h = (rx2 * sin * sin + ry2 * cos * cos).sqrt();

                let min_x = cx - half_w;
                let min_y = cy - half_h;
                [offset.0-min_x, offset.1-min_y]
            }
        };
        positions.iter_mut().for_each(|p| {p[0] += shift[0]; p[1] += shift[1]});
        positions
    }
}

use wgpu_canvas::*;

use wgpu_canvas::{Canvas, Atlas, Area, Shape, ShapeType, Item, Image};
use maverick_os::{Application, Context, Services};
use maverick_os::window::{
    Event, Lifetime, Input, TouchPhase, Touch, MouseScrollDelta, ElementState, Key, NamedKey
};

use std::sync::mpsc::{Receiver, channel};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::Instant;

pub struct Ramp{
    atlas: Atlas,
    canvas: Canvas,
    screen: (f32, f32),
    scale_factor: f64
}
impl Ramp {
    pub fn physical(&self, x: f32) -> f32 {(x as f64 * self.scale_factor) as f32}
    pub fn logical(&self, x: f32) -> f32 {(x as f64 / self.scale_factor) as f32}

    fn shape(&self, shape: ShapeType) -> ShapeType {
        match shape {
            ShapeType::Ellipse(s, (w, h), a) =>
                ShapeType::Ellipse(self.physical(s), (self.physical(w), self.physical(h)), a),
            ShapeType::Rectangle(s, (w, h), a) =>
                ShapeType::Rectangle(self.physical(s), (self.physical(w), self.physical(h)), a),
            ShapeType::RoundedRectangle(s, (w, h), a, c) =>
                ShapeType::RoundedRectangle(self.physical(s), (self.physical(w), self.physical(h)), a, self.physical(c)),
        }
    }
}
impl Services for Ramp {}
impl Application for Ramp {
    async fn new(ctx: &mut Context) -> Self {
        let scale_factor = ctx.window.scale_factor;
        let screen = (
            (ctx.window.size.0 as f64 / scale_factor) as f32,
            (ctx.window.size.1 as f64 / scale_factor) as f32,
        );

        Ramp{
            atlas: Atlas::default(),
            canvas: Canvas::new(ctx.window.handle.clone(), ctx.window.size.0, ctx.window.size.1).await,
            screen,
            scale_factor,
        }
    }
    async fn on_event(&mut self, ctx: &mut Context, event: Event) {
        let window = matches!(event, Event::Lifetime(Lifetime::Resumed)).then(|| ctx.window.handle.clone());
        match event {
            Event::Lifetime(lifetime) => match lifetime {
                Lifetime::Resized | Lifetime::Resumed => {
                    self.scale_factor = ctx.window.scale_factor;
                    self.canvas.resize(window, ctx.window.size.0, ctx.window.size.1);
                    self.screen = (self.logical(ctx.window.size.0 as f32), self.logical(ctx.window.size.1 as f32));
                },
                Lifetime::Draw => {
                    let drawn = vec![
                        (Area{
                            offset: (100.0, 100.0),
                            bounds: None
                        }, Item::Shape(Shape{
                            shape: ShapeType::Rectangle(0.0, (100.0, 100.0), 0.0),
                            color: Color(0, 255, 255, 255)
                        }))
                    ];
                    let scaled: Vec<_> = drawn.into_iter().map(|(a, i)| {
                        (Area{
                            offset: (self.physical(a.offset.0), self.physical(a.offset.1)),
                            bounds: a.bounds.map(|b| (self.physical(b.0), self.physical(b.1), self.physical(b.2), self.physical(b.3)))
                        }, match i {
                            Item::Shape(shape) => Item::Shape(Shape{
                                shape: self.shape(shape.shape),
                                color: shape.color
                            }),
                            Item::Image(image) => Item::Image(Image{
                                shape: self.shape(image.shape),
                                image: image.image,
                                color: image.color
                            }),
                            Item::Text(mut text) => Item::Text({
                                text.width = text.width.map(|w| self.physical(w));
                                text.spans.iter_mut().for_each(|span| {
                                    span.font_size = self.physical(span.font_size);
                                    span.line_height = span.line_height.map(|l| self.physical(l));
                                    span.kerning = self.physical(span.kerning);
                                });
                                text
                            })
                        })
                    }).collect();
                    self.canvas.draw(&mut self.atlas, scaled);
                },
                lifetime => {println!("Lifetime {:?}", lifetime);}
            },
            Event::Input(input) => match input {
                input => {println!("I got input: {:?}", input);}
            }
        }
    }
}

maverick_os::start!(Ramp);

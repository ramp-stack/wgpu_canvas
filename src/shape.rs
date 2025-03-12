use std::hash::{DefaultHasher, Hasher, Hash};

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use fast_image_resize::{Resizer, ResizeOptions, ResizeAlg, FilterType};

use super::{ImageAtlas, ImageKey, Image};
use image_crate::GenericImage;

pub type ShapeKey = u64;

pub trait Shape: Hash {
    fn build(self) -> Image;
}

#[derive(Default)]
pub struct ShapeAtlas {
    inner: HashMap<ShapeKey, ImageKey>
}

impl ShapeAtlas {
    pub fn add(&mut self, shape: impl Shape, image_atlas: &mut ImageAtlas) -> ShapeKey {
        let mut hasher = DefaultHasher::new();
        shape.hash(&mut hasher);
        let key = hasher.finish();
        if let Entry::Vacant(entry) = self.inner.entry(key) {
            entry.insert(image_atlas.add(shape.build()));
        }
        key
    }

    pub fn get(&mut self, key: &ShapeKey) -> ImageKey {
        *self.inner.get(key).expect("Shape is not cached")
    }

    pub fn remove(&mut self, key: &ShapeKey) {
        self.inner.remove(key);
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct Ellipse {
    pub color: (u8, u8, u8, u8),
    pub stroke: u32,
    pub size: (u32, u32),
}

impl Shape for Ellipse {
    fn build(self) -> Image {
        let size = (self.size.0*4, self.size.1*4);

        let mut image = image_crate::DynamicImage::new(size.0, size.1, image_crate::ColorType::Rgba8);
      //for x in 0..size.0 {
      //    for y in 0..size.1 {
      //        image.put_pixel(x, y, image_crate::Rgba([200, 200, 200, 255]));
      //    }
      //}

        let pixel = image_crate::Rgba([self.color.0, self.color.1, self.color.2, self.color.3]);
        let hsize = (size.0/2, size.1/2);

        let steps = size.0*size.1;
        let increment = std::f64::consts::PI/(steps as f64 / 2.0);

        let k = (self.stroke as f64)*-4.0;
        let a = hsize.0 as f64 - 0.5;
        let b = hsize.1 as f64 - 0.5;
        for i in 0..steps {
            let t = -std::f64::consts::PI + (increment*i as f64);

            let x = a * t.cos();
            let y = b * t.sin();

            let x = (a+x).round();
            let y = (b+y).round();

            let x = (x as u32).min(size.0-1);
            let y = (y as u32).min(size.1-1);

            //image.put_pixel(x, y, pixel);

          //let x = (a + (b*k / (((a*a)*(t.sin()*t.sin()))+((b*b)*(t.cos()*t.cos()))).sqrt())) * t.cos();
          //let y = (b + (a*k / (((a*a)*(t.sin()*t.sin()))+((b*b)*(t.cos()*t.cos()))).sqrt())) * t.sin();

          //let x = (0.5+a+x).round();
          //let y = (0.5+b+y).round();

          //let x = (x as u32).min(size.0-1);
          //let y = (y as u32).min(size.1-1);

          //image.put_pixel(
          //    x, y,
          //    pixel
          //);


            if x < hsize.0 {
                for i in x..hsize.0 {
                    image.put_pixel(i, y, pixel);
                }
            } else {
                for i in hsize.0..x {
                    image.put_pixel(i, y, pixel);
                }
            }
        }

        let mut dst_image = image_crate::DynamicImage::new(
            self.size.0, self.size.1, image_crate::ColorType::Rgba8
        );

        let mut resizer = Resizer::new();
        resizer.resize(&image, &mut dst_image,
            //&ResizeOptions::new().resize_alg(ResizeAlg::Interpolation(FilterType::CatmullRom))
            &None
        ).unwrap();


        //&ResizeOptions::new().resize_alg(ResizeAlg::SuperSampling(FilterType::Lanczos3, 255))
      //image_crate::imageops::resize(
      //    &image, self.size.0, self.size.1,
      //    image_crate::imageops::FilterType::Gaussian
      //)
        dst_image.into()
    }
}

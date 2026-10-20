//! A Gray4 framebuffer for host-side layout tests.
//!
//! The framebuffer implements `DrawTarget` and writes PGM images for manual
//! review under `CARGO_TARGET_TMPDIR`. It is test support only, not part of
//! the library.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Dimensions, Point, Size};
use embedded_graphics::pixelcolor::{Gray4, GrayColor};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

pub struct Framebuffer<const W: usize, const H: usize> {
    pixels: [[u8; W]; H],
}

impl<const W: usize, const H: usize> Framebuffer<W, H> {
    pub fn new() -> Self {
        Self {
            pixels: [[15; W]; H],
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> Gray4 {
        Gray4::new(self.pixels[y][x])
    }

    /// Write the screen as a binary PGM image with maximum value 15.
    pub fn write_pgm(&self, name: &str) -> io::Result<PathBuf> {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oink-ui-snapshots");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.pgm"));
        let mut out = BufWriter::new(File::create(&path)?);
        write!(out, "P5\n{} {}\n15\n", W, H)?;
        for row in &self.pixels {
            out.write_all(row)?;
        }
        out.flush()?;
        Ok(path)
    }
}

impl<const W: usize, const H: usize> Default for Framebuffer<W, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize> Dimensions for Framebuffer<W, H> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), Size::new(W as u32, H as u32))
    }
}

impl<const W: usize, const H: usize> DrawTarget for Framebuffer<W, H> {
    type Color = Gray4;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 {
                continue;
            }
            let (x, y) = (point.x as usize, point.y as usize);
            if x < W && y < H {
                self.pixels[y][x] = color.luma();
            }
        }
        Ok(())
    }
}

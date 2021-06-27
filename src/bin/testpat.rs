// Copyright 2020-2021 bd_
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions: The above copyright
// notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use thiserror::Error;
use std::convert::TryInto;

const MARGIN: f64 = 8.0;
const PADDING: f64 = 8.0;

const LEFT_FONT: &str = "Sans 32";
const RIGHT_FONT: &str = "Sans 20";

const LEFT_COL_WIDTH: i32 = 284;
const RIGHT_COL_WIDTH: i32 = 852;
const TOTAL_WIDTH: f64 = (LEFT_COL_WIDTH + RIGHT_COL_WIDTH) as f64;

#[derive(Error, Debug)]
pub enum UpdaterError {
    #[error("Cairo error: {0}")]
    CairoError(cairo::Status),
}

impl From<cairo::Status> for UpdaterError {
    fn from(s: cairo::Status) -> Self {
        UpdaterError::CairoError(s)
    }
}

fn convert_err<E>(err: E) -> anyhow::Error
where
    UpdaterError: From<E>,
{
    UpdaterError::from(err).into()
}

fn convert_part(v: usize) -> u8 {
    let v: u8 = v.try_into().unwrap();
    let mut v = v << 2;
    if v > 96 {
        v += 2;
    }
    v
}


fn from_value(value: usize) -> [u8;4] {
    if value >= (1 << 18) {
        panic!("Value {} is too large to be represented", value);
    }

    let r = (value >> 12) & 0x3F;
    let g = (value >> 6) & 0x3F;
    let b = value & 0x3F;

    let r: u8 = convert_part(r);
    let g: u8 = convert_part(g);
    let b: u8 = convert_part(b);

    [b,g,r,0xff]
}
fn main() -> anyhow::Result<()> {
    let block_width: usize = 1;

    let dim = 16 * block_width;

    let mut surface = cairo::ImageSurface::create(cairo::Format::Rgb24, dim as i32, dim as i32)
        .map_err(convert_err)?;
    let stride = surface.get_stride();
    let mut data = surface.get_data()?;

    dbg!(data.len());

    let rows = (&mut data[..]).chunks_exact_mut(stride as usize);

    for (y, stride) in rows.enumerate() {
        let stride = &mut stride[0..(dim * 4)];

        for (x, slice) in stride.chunks_exact_mut(4).enumerate() {
            let x = x / block_width;
            let y = y / block_width;

            // UV coords are low to top
            let y = 15 - y;

            let value = x + (y << 4);

            slice.copy_from_slice(&from_value(value)[..]);
            // B G R A
        }
    }

    // Render to file
    std::mem::drop(data);

    let f = std::fs::File::create("colors.png")?;
    let mut f = std::io::BufWriter::new(f);

    surface.write_to_png(&mut f)?;

    Ok(())
}

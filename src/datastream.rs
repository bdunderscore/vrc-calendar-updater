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

use crate::render_prims::*;

use anyhow::{bail, Result, Context};

use std::convert::{TryFrom, TryInto};
use tracing::{debug, info, trace};

const DATA_COL_WIDTH: i32 = 64;
const HEADER_HEIGHT: u32 = 128;

#[derive(Copy, Clone, Debug, Default)]
pub struct ByteColor {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}

trait SelectBit {
    fn select(self, n: Self, a: u8) -> u8;
}

impl SelectBit for usize {
    fn select(self, n: Self, a: u8) -> u8 {
        if self & (1 << n) != 0 {
            a
        } else {
            0
        }
    }
}

fn convert_part(v: u32) -> Result<u8> {
    let v: u8 = v.try_into()?;
    let mut v = v << 2;
    if v > 96 {
        v += 2;
    }
    Ok(v)
}

impl ByteColor {
    fn from_value(value: u32) -> Result<Self> {
        if value >= (1 << 18) {
            bail!(format!("Value {} is too large to be represented", value));
        }

        let r = (value >> 12) & 0x3F;
        let g = (value >> 6) & 0x3F;
        let b = value & 0x3F;

        let r: u8 = convert_part(r)?;
        let g: u8 = convert_part(g)?;
        let b: u8 = convert_part(b)?;

        Ok(Self { r, g, b, a: 0xFF })
    }

    fn to_array(self) -> [u8; 4] {
        let ByteColor { r, g, b, a } = self;

        let le_bytes = [b, g, r, a];
        let int_val = u32::from_le_bytes(le_bytes);
        int_val.to_ne_bytes()
    }
}

impl From<crate::Color> for ByteColor {
    fn from(color: crate::Color) -> Self {
        ByteColor {
            r: f64::max(0.0, f64::min(255.0, color.r * 255.0)).round() as u8,
            g: f64::max(0.0, f64::min(255.0, color.g * 255.0)).round() as u8,
            b: f64::max(0.0, f64::min(255.0, color.b * 255.0)).round() as u8,
            a: 0xff
        }
    }
}


impl From<(u8,u8,u8)> for ByteColor {
    fn from(color: (u8,u8,u8)) -> Self {
        ByteColor {
            r: color.0,
            g: color.1,
            b: color.2,
            a: 0xff
        }
    }
}

impl TryFrom<u32> for ByteColor {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self> {
        ByteColor::from_value(value)
    }
}

macro_rules! write_elem {
    ($ds:expr, $v:expr) => {
        {
            let v = $v;
            let mut s = String::from(stringify!($v));
            if s.starts_with("self.") {
                s.drain(0..5);
            }
            let s = s.to_uppercase();
            let col : ByteColor = v.try_into().with_context(|| format!("converting {} ({})", &s, v))?;
            trace!("#define SCROLLCAL_DSOFF_{} {} // {}", s, $ds.len(), v);
            $ds.push(col);
        }
    }
}

/// The metadata that will be encoded into the output image
#[derive(Default, Debug, Clone)]
pub struct DatastreamElements {
    // Parameters for identifying datastream elements
    pub datastream_width: u32,
    pub datastream_height: u32,

    // Parameters for identifying the local coords of visual elements

    // Overall display height/width
    pub viewport_h: u32,
    pub viewport_w: u32,

    // The following coordinates identify the box surrounding the primary scrollable
    // section of the display.
    pub header_h: u32,
    pub footer_h: u32,
    pub border_l: u32,
    pub border_r: u32,

    // This identifies the height of the day headers
    pub day_header_height: u32,

    // Size of the band at the top of the top header which isn't scrolled
    pub header_blend_start: u32,

    // End of the area to blend over to the scrollable part of the header
    pub header_blend_end: u32,

    // Y-position at which we split the sides when scrolling off the header
    pub scroll_split_point: u32,

    // Coordinates of ByteColor column dividers
    pub col_divs: [u32;3],

    // Main palette
    pub palette: [ByteColor;8],

    // The following coordinates locate items in the texture space.
    // Our texture space coordinates place the origin at the upper left, and are expressed in texels.

    // Left border is placed at (0,0) (with x,y coords swapped)
    // Right border is placed at (0,section_pad*2)
    
    // Padding between elements (particularly the sides)
    pub section_pad: u32,

    // Height of the scrollable section.
    pub scroll_height: u32,
    // Texture coordinates at which the scrollable section begins
    pub scroll_tex_y: u32,

    // Y-coordinate offset of the background sample section
    pub bg_sample_y: u32,

    // Height of the background sample section
    pub bg_sample_h: u32,

    // Y-coordinate offsets of the header and footer sections
    pub header_tex_y: u32,
    pub footer_tex_y: u32,

    // x,y coords of the day header. We include only the left and right sides; the middle
    // is generated by stretching the middle pixels to fill
    pub day_header_tex_x: u32,
    pub day_header_tex_alpha_x: u32,
    pub day_header_tex_y: u32,
    // Size of the sides that we include
    pub day_header_side_width: u32,
    // The size we stretch the day header to
    pub day_header_true_width: u32,

    pub vdata: Vec<VerticalData>
}

pub const FLAG_IS_DAY_HEADER : u32 = (1 << 17);

#[derive(Clone,Copy,Debug,Eq, PartialEq)]
pub enum RowColorInfo {
    Colors([u8;4]),
    DayHeader { offset: u32 }
}

// Information for a specific row in the scrollable section
#[derive(Debug, Clone)]
pub struct VerticalData {
    // y-coordinate of the day header before us
    pub prev_day_header: u32,
    pub col_info: RowColorInfo,
}

impl DatastreamElements {
    pub fn encode(&self) -> Result<Vec<ByteColor>> {
        let mut ds = Vec::new();

        //return Ok(vec![]);

        write_elem!(ds, self.datastream_width);
        write_elem!(ds, self.datastream_height);

        write_elem!(ds, self.viewport_w);
        write_elem!(ds, self.viewport_h);

        write_elem!(ds, self.header_h);
        write_elem!(ds, self.footer_h);
        write_elem!(ds, self.border_l);
        write_elem!(ds, self.border_r);

        write_elem!(ds, self.day_header_height);
        
        for div in self.col_divs.iter().copied() {
            write_elem!(ds, div);
        }
        eprintln!("#define SCROLLCAL_DSOFF_PALETTE {}", ds.len());
        for col in self.palette.iter().copied() {
            ds.push(col);
        }

        write_elem!(ds, self.section_pad);
        write_elem!(ds, self.scroll_height);
        write_elem!(ds, self.scroll_tex_y);
        write_elem!(ds, self.bg_sample_y);
        write_elem!(ds, self.bg_sample_h);
        write_elem!(ds, self.header_tex_y);
        write_elem!(ds, self.footer_tex_y);
        write_elem!(ds, self.day_header_tex_x);
        write_elem!(ds, self.day_header_tex_alpha_x);
        write_elem!(ds, self.day_header_tex_y);
        write_elem!(ds, self.day_header_side_width);
        write_elem!(ds, self.day_header_true_width);

        write_elem!(ds, self.header_blend_start);
        write_elem!(ds, self.header_blend_end);
        write_elem!(ds, self.scroll_split_point);

        let vdata_len : u32 = self.vdata.len().try_into().context("vdata.len() conversion")?;

        write_elem!(ds, vdata_len);

        trace!("#define SCROLLCAL_DSOFF_PREVDH {}", ds.len());
        for (i, vd) in self.vdata.iter().enumerate() {            
            ds.push(vd.prev_day_header.try_into().context("prev_day_header")?);
        }

        trace!("#define SCROLLCAL_DSOFF_ROWINFO {}", ds.len());
        
        for (i, vd) in self.vdata.iter().enumerate() {            
            match vd.col_info {
                RowColorInfo::Colors(colors) => {
                    // Encode colors into a single pixel
                    let mut tmp_colors : Vec<u32> = vec![];
                    for (j, col) in colors.iter().copied().enumerate() {
                        if col >= 8 {
                            bail!("Color out of range");
                        }

                        tmp_colors.push(col as u32);
                    }

                    let col_info : u32 = (tmp_colors[0] << 9) | (tmp_colors[1] << 6) | (tmp_colors[2] << 3) | tmp_colors[3];

                    ds.push(col_info.try_into().context("color_info")?);
                },
                RowColorInfo::DayHeader{offset} => ds.push((offset | FLAG_IS_DAY_HEADER).try_into().unwrap())
            }
        }

        Ok(ds)
    }

        
    pub fn write(&self, surf: &mut cairo::ImageSurface) -> Result<()> {
        let data = self.encode()?;

        if data.len() > (self.datastream_width * self.datastream_height) as usize {
            bail!("Not enough space for datastream");
        }

        let stride_size : usize = surf.get_stride().try_into()?;
        let img_width : usize = surf.get_width().try_into()?;
        let mut img_data = surf.get_data()?;

        let strides = data.chunks(self.datastream_width.try_into()?);

        for (y, stride) in strides.enumerate() {
            let mut row = &mut img_data[stride_size * y .. stride_size * (y + 1)];

            for (rx, col) in stride.iter().copied().enumerate() {
                let x = img_width - rx - 1;
                let v = col.to_array();

                row[x*4..(x+1)*4].copy_from_slice(&v);
            }
        }

        Ok(())
    }

}

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

use super::{rgb, RGBInt};

pub const RGB_TEXT_ENDED: RGBInt = rgb(0x9BAEC0);
pub const RGB_TIME_ENDED: RGBInt = rgb(0x7D8D93);
pub const RGB_TEXT: RGBInt = rgb(0x694342);
pub const RGB_TIME: RGBInt = rgb(0x7D5757);
pub const RGB_DATE: RGBInt = rgb(0xEFD4A5);
pub const RGB_TIME_DASH: RGBInt = rgb(0xC28979);

pub const PALETTE: [RGBInt;8] = [
    RGB_DATE,
    RGB_TEXT_ENDED,
    RGB_TIME_ENDED,
    RGB_TEXT,
    RGB_TIME,
    RGB_TIME_DASH,
    rgb(0xFF00FF),
    rgb(0x00FFFF),
];

pub const PAL_DATE: u8 = 0;
pub const PAL_TEXT_ENDED : u8 = 1;
pub const PAL_TIME_ENDED: u8 = 2;
pub const PAL_TEXT: u8 = 3;
pub const PAL_TIME: u8 = 4;
pub const PAL_TIME_DASH: u8 = 5;

pub const VIEWPORT_HEIGHT : u32 = 1447;
pub const VIEWPORT_WIDTH  : u32 = 1024;

pub const TEXTURE_HEIGHT  : u32 = 4096;
pub const TEXTURE_WIDTH   : u32 = VIEWPORT_WIDTH;

pub const MARGIN: f64 = 8.0;
pub const PADDING: f64 = 8.0;

pub const SECTION_PAD: f64 = 32.0;

pub const LEFT_BORDER: i32 = 23;
pub const RIGHT_BORDER: i32 = 71;

pub const VARIABLE_OUTER_LEFT: i32 = 23;
pub const VARIABLE_OUTER_RIGHT: i32 = 953;

pub const VARIABLE_TOP: i32 = 585;//602;
pub const VARIABLE_TEMPLATE_TOP : i32 = 590;
pub const VARIABLE_BOTTOM: i32 = 1313;
pub const VARIABLE_HEADER_BOTTOM: i32 = VARIABLE_TOP + 95; // todo - implicit

pub const DAY_HEADER_HEIGHT: i32 = VARIABLE_HEADER_BOTTOM - VARIABLE_TOP;
pub const TIME_COL_LEFT: i32 = 28;
pub const TIME_COL_RIGHT: i32 = 139;
pub const EVENT_INFO_LEFT: i32 = 144 + 16;
pub const EVENT_INFO_RIGHT: i32 = 948;
pub const VARIABLE_LEFT: i32 = TIME_COL_LEFT;
pub const VARIABLE_RIGHT: i32 = EVENT_INFO_RIGHT;

pub const FONT_SCALE : f64 = 1.0;
pub const FONT_DAY_HEADER: &str = "M+ 1m bold 21.6";
pub const FONT_TIME: &str = "M+ 1m bold 16.2";
pub const FONT_END_TIME: &str = "M+ 1m regular 10.8";
pub const FONT_EVENT_INFO: &str = "M+ 1m medium 16.2";
pub const FONT_CONFIG_INFO: &str = "M+ 1m regular 10.8";

pub const EVENT_MARKER_HEIGHT: f64 = 16.0;
pub const EVENT_MARKER_WIDTH: f64 = EVENT_MARKER_HEIGHT * 0.866;
pub const EVENT_MARKER_CLIP: f64 = 4.0;

pub const RGB_EVENT_MARKER: RGBInt = rgb(0x5A494F);

pub const SWATCH_SIZE: i32 = 32;

pub const BG_SAMPLE_HEIGHT: u32 = 32;

pub const SCROLL_SPLIT_POINT: i32 = VARIABLE_BOTTOM;

pub const HEADER_BLEND_START: i32 = 8;
pub const HEADER_BLEND_END: i32 = 16;

pub fn config_datastream_info() -> crate::datastream::DatastreamElements {
    use crate::datastream::ByteColor;

    let mut palette : [ByteColor;8] = [ByteColor::default();8];
    for i in 0..8 {
        palette[i] = PALETTE[i].into();
    }

    crate::datastream::DatastreamElements {
        datastream_width: u32::max_value(),
        datastream_height: u32::max_value(),
        viewport_h: VIEWPORT_HEIGHT, // make const?
        viewport_w: VIEWPORT_WIDTH,
        header_h: VARIABLE_TOP as u32,
        footer_h: VIEWPORT_HEIGHT - VARIABLE_BOTTOM as u32,
        border_l: LEFT_BORDER as u32,
        border_r: RIGHT_BORDER as u32,
        day_header_height: (VARIABLE_HEADER_BOTTOM - VARIABLE_TOP) as u32,
        header_blend_start: HEADER_BLEND_START as u32,
        header_blend_end: HEADER_BLEND_END as u32,
        scroll_split_point: SCROLL_SPLIT_POINT as u32,
        col_divs: [TIME_COL_RIGHT as u32, (TIME_COL_RIGHT + (EVENT_MARKER_WIDTH.ceil() as i32)) as u32, VIEWPORT_WIDTH as u32],
        section_pad: SECTION_PAD as u32,
        scroll_height: u32::max_value(),
        scroll_tex_y: u32::max_value(),
        bg_sample_y: u32::max_value(),
        bg_sample_h: BG_SAMPLE_HEIGHT,
        header_tex_y: u32::max_value(),
        footer_tex_y: u32::max_value(),
        day_header_tex_x: u32::max_value(),
        day_header_tex_alpha_x: u32::max_value(),
        day_header_tex_y: u32::max_value(),
        day_header_side_width: u32::max_value(),
        day_header_true_width: u32::max_value(),
        vdata: vec![],
        palette: palette,

    }
}


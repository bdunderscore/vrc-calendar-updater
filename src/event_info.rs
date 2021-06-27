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

use super::{CalendarDay, CalendarEvent, Color};
use crate::render_prims::*;
use crate::config::*;

use cairo::Context;

enum ColorSwitch {
    Expired,
    Separator,
    Active
}

#[derive(Clone,Debug)]
pub struct ColorArray(Vec<RGBInt>);

// Indexed by:
// [Column] [Selector x 2]
pub fn color_array() -> ColorArray {
    ColorArray(vec![
        RGB_TIME_ENDED,
        RGB_TIME_DASH,
        RGB_TIME,
        RGB_TEXT_ENDED,
        RGB_DATE,
        RGB_TEXT,
        RGB_EVENT_MARKER,
        rgb(0xFF00FF)
    ])
}

impl Renderable for ColorArray {
    fn render_internal(&self, cr: &mut Context) -> anyhow::Result<()> {
        for i in 0..self.0.len() {
            let col : Color = self.0[i].into();
            cr.new_path();
            cr.rectangle(SWATCH_SIZE as f64 * ((i & 1) as f64), SWATCH_SIZE as f64 * ((i >> 1) as f64), SWATCH_SIZE as f64, SWATCH_SIZE as f64);
            cr.set_source_rgb(col.r, col.g, col.b);
            cr.fill();
        }

        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        (SWATCH_SIZE as f64 * 2.0, ((self.0.len() + 1) / 2) as f64 * (SWATCH_SIZE as f64))
    }
}

#[derive(Clone,Copy,Eq,PartialEq,Debug)]
enum SegmentType {
    DayHeader,
    Separator,
    Event
}

#[derive(Clone)]
struct Segment {
    left_col_color: Color,
    right_col_color: Color,
    renderable: RcRenderable,
    seg_type: SegmentType
}

impl std::fmt::Debug for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Segment {{ left_col_color: {:?}, right_col_color: {:?}, bounds: {:?}, seg_type: {:?} }}",
            self.left_col_color,
            self.right_col_color,
            self.renderable.bounds(),
            self.seg_type
        )
    }
}


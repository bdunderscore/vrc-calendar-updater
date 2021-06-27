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

use anyhow::{Context, Result};
use std::convert::TryInto;

use cairo::Rectangle;

use pango::{FontDescription, Layout};

use crate::config::FONT_SCALE;

pub type RGBInt = (u8, u8, u8);

pub const fn rgb(col: u32) -> RGBInt {
    let r = (col >> 16) as u8;
    let g = (col >> 8) as u8;
    let b = col as u8;

    (r, g, b)
}

const PANGO_SCALE: f64 = 1024.0;

use std::rc::Rc;

use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn debug_color(surf: &mut cairo::Context) {
    let mut c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let r = (1 + c % 4) as f64 / 4.0;
    c >>= 2;
    let g = (1 + c % 4) as f64 / 4.0;
    c >>= 2;
    let b = (1 + c % 4) as f64 / 4.0;

    surf.set_source_rgb(r, g, b);
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl From<RGBInt> for Color {
    fn from(rgb: RGBInt) -> Self {
        Color {
            r: (rgb.0 as f64 * (1.0 / 255.0)),
            g: (rgb.1 as f64 * (1.0 / 255.0)),
            b: (rgb.2 as f64 * (1.0 / 255.0)),
        }
    }
}

pub fn prepare_layout(
    context: &cairo::Context,
    font: &FontDescription,
    width: i32,
    text: &str,
) -> Result<Layout> {
    let layout = pangocairo::create_layout(context)
        .ok_or_else(|| anyhow::anyhow!("Failed to create pango layout"))?;

    layout.set_font_description(Some(&font));
    layout.set_text(text);
    layout.set_width(width.try_into()?);
    layout.set_wrap(pango::WrapMode::Word);

    let (_w, _h) = layout.get_size();
    Ok(layout)
}

pub fn layout_size_px(layout: &pango::Layout) -> (f64, f64) {
    let (w, h) = layout.get_size();
    (w as f64 / PANGO_SCALE, h as f64 / PANGO_SCALE)
}

pub struct TestBorder<R: Renderable> { r: R }

impl<R: Renderable> Renderable for TestBorder<R> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        let (w, h) = self.bounds();

        cr.save();
        self.r.render(cr)?;
        cr.restore();

        cr.new_path();
        cr.move_to(0.0,0.0);
        cr.line_to(w,0.0);
        cr.line_to(w,h);
        cr.line_to(0.0,h);
        cr.close_path();

        cr.set_line_width(2.0);
        cr.stroke();

        Ok(())
    }
    fn bounds(&self) -> (f64, f64) {
        self.r.bounds()
    }
}

#[derive(Clone)]
pub struct RcRenderable(pub Rc<dyn Renderable>);

impl Renderable for RcRenderable {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        self.0.render(cr)
    }
    fn bounds(&self) -> (f64, f64) {
        self.0.bounds()
    }
}

pub trait Renderable {
    fn test_border(self) -> TestBorder<Self> where Self: Sized {
        TestBorder { r: self }
    }

    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()>;

    fn render(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.save();
        cr.move_to(0.0, 0.0);
        let result = self.render_internal(cr);
        cr.restore();

        result
    }

    fn render_to(&self, cr: &mut cairo::Context, origin: (f64, f64)) -> Result<()> {
        cr.save();
        cr.translate(origin.0, origin.1);

        let result = self.render(cr);

        cr.restore();

        result
    }

    fn bounds(&self) -> (f64, f64);

    fn height(&self) -> f64 {
        self.bounds().1
    }
    fn width(&self) -> f64 {
        self.bounds().0
    }
}

impl Renderable for Rc<dyn Renderable> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        std::ops::Deref::deref(self).render(cr)
    }
    fn bounds(&self) -> (f64, f64) {
        std::ops::Deref::deref(self).bounds()
    }
}

impl Renderable for cairo::ImageSurface {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.move_to(0.0, 0.0);
        //cr.set_operator(cairo::Operator::Source);
        cr.set_source_surface(self, 0.0, 0.0);
        //debug_color(cr);
        cr.new_path();
        cr.rectangle(0.0, 0.0, self.get_width() as f64, self.get_height() as f64);
        cr.fill();

        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        (self.get_width() as f64, self.get_height() as f64)
    }
}

pub struct RenderTranslate {
    pub inner: Box<dyn Renderable>,
    pub offset: (f64, f64),
}

impl Renderable for RenderTranslate {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        self.inner.render_to(cr, (self.offset.0, self.offset.1))
    }
    fn bounds(&self) -> (f64, f64) {
        let (w, h) = self.inner.bounds();
        (w + self.offset.0, h + self.offset.1)
    }
}

pub trait RenderableEx: Renderable {
    fn clip_to(self, clip_bounds: Rectangle) -> Clip<Self>
    where
        Self: Sized,
    {
        Clip {
            inner: self,
            clip_bounds,
        }
    }

    fn scale_by(self, w: f64, h: f64) -> Scale<Self>
    where
        Self: Sized,
    {
        Scale::scale_by(self, w, h)
    }

    fn offset(self, x: f64, y: f64) -> RenderTranslate
    where
        Self: Sized + 'static,
    {
        RenderTranslate {
            inner: Box::new(self),
            offset: (x, y),
        }
    }

    fn into_rc(self) -> RcRenderable
    where
        Self: Sized + 'static,
    {
        RcRenderable(Rc::new(self))
    }

    fn center_in_front_of(self, other: impl Renderable + Sized + 'static) -> RcRenderable
    where
        Self: Sized + 'static,
    {
        assert!(self.width() <= other.width());
        assert!(self.height() <= other.height());

        let this_width = self.width();
        let this_height = self.height();

        let this = self.offset(
            (other.width() - this_width) / 2.0,
            (other.height() - this_height) / 2.0,
        );

        let mut layout = RenderGroup::new();
        layout.push(other);
        layout.push(this);

        layout.into_rc()
    }

    fn margin(self, m_w: f64, m_h: f64) -> RcRenderable
    where
        Self: Sized + 'static,
    {
        let this = self.offset(m_w, m_h);
        let (w, h) = this.bounds();

        Margin {
            inner: this,
            bounds: (w + m_w, h + m_h),
        }
        .into_rc()
    }

    fn with_operator(self, operator: cairo::Operator) -> RcRenderable
    where
        Self: Sized + 'static,
    {
        WithOperator {
            inner: self,
            operator,
        }
        .into_rc()
    }

    fn pad_vertical(self, pad_above: f64, pad_below: f64) -> RcRenderable
    where Self: Sized + 'static
    {
        pad_vertical(self.into_rc(), pad_above, pad_below)
    }

    fn pad_sides(self, pad_left: f64, pad_right: f64) -> RcRenderable
    where Self: Sized + 'static
    {
        pad_sides(self.into_rc(), pad_left, pad_right)
    }
}

impl<R: Renderable> RenderableEx for R {}

struct WithOperator<R: Renderable> {
    inner: R,
    operator: cairo::Operator,
}

impl<R: Renderable> Renderable for WithOperator<R> {
    fn render_internal(&self, cx: &mut cairo::Context) -> Result<()> {
        cx.save();
        cx.set_operator(self.operator);
        let result = self.inner.render_internal(cx);
        cx.restore();

        result
    }
    fn bounds(&self) -> (f64, f64) {
        self.inner.bounds()
    }
}

pub struct RenderGroup {
    pub items: Vec<Box<dyn Renderable>>,
}

impl RenderGroup {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn push(&mut self, item: impl Renderable + 'static) {
        self.items.push(Box::new(item));
    }
}

impl Renderable for RenderGroup {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        for item in self.items.iter() {
            item.render(cr)?;
        }

        Ok(())
    }
    fn bounds(&self) -> (f64, f64) {
        let mut w = 0.0;
        let mut h = 0.0;

        for item in self.items.iter() {
            let (iw, ih) = item.bounds();

            if iw > w {
                w = iw;
            }
            if ih > h {
                h = ih;
            }
        }

        (w, h)
    }
}

pub struct TextBox {
    text: String,
    original_width: i32,
    color: Color,
    font: FontDescription,
    width: f64,
    height: f64,

    // properties for query
    min_baseline: f64,
}

use std::collections::HashMap;
use std::cell::RefCell;
use std::borrow::BorrowMut;

thread_local! {
    static WIDTH_HISTOGRAM : RefCell<HashMap<u32, u32>> = std::cell::RefCell::new(HashMap::new());
    static TEXT_HISTOGRAM : RefCell<HashMap<String, u32>> = std::cell::RefCell::new(HashMap::new());
}

impl TextBox {
    pub fn new(
        context: &cairo::Context,
        text: String,
        width: f64,
        color: Color,
        font: &FontDescription,
        max_lines: usize,
    ) -> Result<TextBox> {
        let width = (width / FONT_SCALE).floor();
        let width = (width * PANGO_SCALE) as i32;
        let layout = prepare_layout(context, font, width, &text)?;
        let (w, h) = layout_size_px(&layout);

        let mut rv = TextBox {
            text: text.clone(),
            original_width: width,
            color,
            font: font.clone(),
            width: w,
            height: h,
            min_baseline: 0.0,
        };

        let iter = layout.get_iter();

        if iter.is_none() {
            return Ok(rv);
        }

        let mut iter = iter.unwrap();
        let mut index = 0;
        loop {
            let (ink, logical) = iter.get_cluster_extents();
            let has_more = iter.next_cluster();
            let end_index = if has_more {
                iter.get_index() as usize
            } else {
                text.len()
            };
            let snippet = text.get(index..end_index).unwrap_or("[error]");
            /*eprintln!("ink=({}, {}) logical=({}, {}) text={:?}",
                ink.width as f64 / PANGO_SCALE,
                ink.height as f64 / PANGO_SCALE,
                logical.width as f64 / PANGO_SCALE,
                logical.height as f64 / PANGO_SCALE,
                snippet
            );*/
            index = end_index;

            WIDTH_HISTOGRAM.with(|histo| {
                let mut histo = histo.borrow_mut();
                (*histo.entry(logical.width as u32 / (PANGO_SCALE as u32))
                    .or_insert(0)) += 1;
            });
            TEXT_HISTOGRAM.with(|histo| {
                let mut histo = histo.borrow_mut();
                (*histo.entry(snippet.to_string())
                    .or_insert(0)) += 1;
            });

            if !has_more {
                break;
            }
        }
        let mut iter = layout.get_iter().unwrap();

        let top = iter.get_line_yrange().0;
        for _ in 0..(max_lines - 1) {
            iter.next_line();
        }
        let bottom = iter.get_line_yrange().1;

        rv.width = (rv.width * FONT_SCALE).ceil();
        rv.height = ((((bottom - top) as f64) / PANGO_SCALE) * FONT_SCALE).ceil();
        rv.min_baseline = ((iter.get_baseline() as f64 / PANGO_SCALE) * FONT_SCALE).ceil();

        Ok(rv)
    }

    pub fn min_baseline(&self) -> f64 {
        self.min_baseline
    }
}

fn dump_histo<T: Clone + std::fmt::Debug>(h: &HashMap<T, u32>, cutoff: usize) {
    let pct : f64 =  (h.len() as f64 * 100.0) / h.iter().map(|(k, v)| *v as f64).sum::<f64>();
    eprintln!("  -> Total {} entries ({}% reused)", h.len(), pct);

    let mut v : Vec<(&T, u32)> = h.iter().map(|(k,v)| (k, *v)).collect();
    v.sort_by_key(|(k, v)| *v);

    if v.len() > cutoff * 2 {
        for (k, v) in v[..cutoff].iter() {
            eprintln!("K: {:?} V: {}", k, v);
        }

        eprintln!("   ...");

        for (k, v) in v[v.len() - cutoff..].iter() {
            eprintln!("K: {:?} V: {}", k, v);
        }
    } else {
        for (k, v) in v.iter() {
            eprintln!("K: {:?} V: {}", k, v);
        }
    }
}

pub fn dump_text_histograms() {
    eprintln!("=== Text histogram ===");
    TEXT_HISTOGRAM.with(|h| dump_histo(&*h.borrow(), 10));
    eprintln!("=== Width histogram ===");
    WIDTH_HISTOGRAM.with(|h| dump_histo(&*h.borrow(), 10));
}

impl Renderable for TextBox {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.move_to(0.0, 0.0);
        cr.new_path();
        cr.rectangle(0.0, 0.0, self.width, self.height);
        cr.clip();
        cr.scale(FONT_SCALE, FONT_SCALE);

        cr.new_path();

        cr.set_source_rgb(self.color.r, self.color.g, self.color.b);
        let layout = prepare_layout(cr, &self.font, self.original_width, &self.text)?;
        pangocairo::show_layout(cr, &layout);

        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FillRect {
    pub bounds: Rectangle,
    pub color: Color,
}

impl FillRect {
    pub fn rect(color: Color, w: f64, h: f64) -> Self {
        Self {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            },
            color,
        }
    }
}

impl Renderable for FillRect {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.move_to(0.0, 0.0);
        cr.set_source_rgb(self.color.r, self.color.g, self.color.b);
        //debug_color(cr);
        cr.new_path();
        cr.rectangle(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
        );
        cr.fill();
        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        (
            self.bounds.x + self.bounds.width,
            self.bounds.y + self.bounds.height,
        )
    }
}

pub struct RenderColumn {
    items: Vec<Box<dyn Renderable>>,
    height: f64,
    width: f64,
}

impl RenderColumn {
    pub fn new() -> Self {
        Self {
            items: vec![],
            height: 0.0,
            width: 0.0,
        }
    }

    pub fn push(&mut self, item: impl Renderable + 'static) -> f64 {
        let offset = self.height;

        let item = item.offset(0.0, offset);
        let (width, height) = item.bounds();

        self.height = height;
        if width > self.width {
            self.width = width;
        }

        self.items.push(Box::new(item));

        offset
    }
}

impl Renderable for RenderColumn {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        for item in self.items.iter() {
            item.render(cr)?;
        }

        Ok(())
    }
    fn bounds(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

pub fn load_png_surface(png_filename: &str) -> Result<cairo::ImageSurface> {
    let f = std::fs::File::open(png_filename)
        .context(format!("Loading PNG file {:?}", png_filename))?;
    let mut f = std::io::BufReader::new(f);

    cairo::ImageSurface::create_from_png(&mut f).map_err(Into::into)
}

pub struct Scale<R: Renderable> {
    inner: R,
    scale: (f64, f64),
}

impl<R: Renderable> Renderable for Scale<R> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.save();

        cr.scale(self.scale.0, self.scale.1);

        self.inner.render(cr)?;

        cr.restore();

        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        let (w, h) = self.inner.bounds();
        assert!(!w.is_nan() && !h.is_nan());

        (f64::max(0.0, (w * self.scale.0)), f64::max(0.0, (h * self.scale.1)))
    }
}

pub struct Pad {
    bounds: (f64, f64),
}

impl Pad {
    pub fn new(w: f64, h: f64) -> Self {
        Self { bounds: (w, h) }
    }
}

impl Renderable for Pad {
    fn render_internal(&self, _cr: &mut cairo::Context) -> Result<()> {
        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        self.bounds
    }
}

#[derive(Clone)]
pub struct SwapXY<R> {
    inner: R,
}

impl<R: Renderable> SwapXY<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: Renderable> Renderable for SwapXY<R> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.save();

        cr.transform(cairo::Matrix::new(0.0, 1.0, 1.0, 0.0, 0.0, 0.0));
        let r = self.inner.render_internal(cr);

        cr.restore();

        r
    }

    fn bounds(&self) -> (f64, f64) {
        let (w, h) = self.inner.bounds();
        (h, w)
    }
}

#[derive(Clone)]
pub struct Clip<R> {
    inner: R,
    clip_bounds: Rectangle,
}

impl<R: Renderable> Renderable for Clip<R> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.save();

        cr.translate(-self.clip_bounds.x, -self.clip_bounds.y);
        cr.new_path();
        cr.rectangle(
            self.clip_bounds.x,
            self.clip_bounds.y,
            self.clip_bounds.width,
            self.clip_bounds.height,
        );
        cr.clip();
        cr.new_path();
        let result = self.inner.render(cr);

        cr.restore();

        result
    }
    fn bounds(&self) -> (f64, f64) {
        (self.clip_bounds.width, self.clip_bounds.height)
    }
}

impl<R: Renderable> Scale<R> {
    fn scale_by(inner: R, w: f64, h: f64) -> Self {
        Self {
            inner,
            scale: (w, h),
        }
    }
}

pub struct Margin<R: Renderable> {
    inner: R,
    bounds: (f64, f64),
}

impl<R: Renderable> Renderable for Margin<R> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.rectangle(0.0, 0.0, self.bounds.0, self.bounds.1);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.fill();

        self.inner.render(cr)
    }
    fn bounds(&self) -> (f64, f64) {
        self.bounds
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Separator {
    pub color: Color,
    pub width: f64,
    pub thickness: f64,
    pub dash: f64,
    pub margin: f64,
}

impl Renderable for Separator {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        cr.new_path();
        cr.set_source_rgb(self.color.r, self.color.g, self.color.b);
        cr.move_to(0.0, self.margin);
        cr.set_dash(&[self.dash], self.dash * 1.5);
        cr.set_line_width(self.thickness);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.line_to(self.width, self.margin);
        cr.stroke();

        Ok(())
    }
    fn bounds(&self) -> (f64, f64) {
        (self.width, self.thickness + self.margin)
    }
}

struct Placement {
    scale: f64,
    offset: f64,
    clip_start: f64,
    clip_width: f64
}

pub fn pad_vertical(r: impl Renderable + Sized + 'static, pad_above: f64, pad_below: f64) -> RcRenderable {
    let r = r.into_rc();
    let mut render_group = RenderGroup::new();

    let (rw, rh) = r.bounds();
    for y in [-1, 0, 1].iter() {
        let (clip_height, clip_start, scale, offset) = match y {
            -1 => (1.0, 0.0, pad_above, 0.0),
            0 => (rh, 0.0, 1.0, pad_above),
            1 => (1.0, rh - 1.0, pad_below, pad_above + rh),
            _ => unreachable!()
        };

        dbg!(clip_height, clip_start, scale, offset);

        if scale > 0.0 {
            let clipped = r.clone().clip_to(Rectangle {
                x: 0.0,
                y: clip_start,
                width: rw,
                height: clip_height
            });
            let scaled = clipped.scale_by(1.0, scale);
            let placed = scaled.offset(0.0, offset);

            render_group.push(placed);
        }
    }

    render_group.into_rc()
}

pub fn pad_sides(r: RcRenderable, pad_left: f64, pad_right: f64) -> RcRenderable {
    let (rw, rh) = r.bounds();

    let mut render_group = RenderGroup::new();

    for y in [-1, 0, 1].iter() {
        let (clip_w, clip_start, scale, offset) = match y {
            -1 => (1.0, 0.0, pad_left, 0.0),
            0 => (rw, 0.0, 1.0, pad_left),
            1 => (1.0, rw - 1.0, pad_right, pad_left + rw),
            _ => unreachable!()
        };

        if scale > 0.0 {
            let clipped = r.clone().clip_to(Rectangle {
                x: clip_start,
                y: 0.0,
                width: clip_w,
                height: rh
            });
            let scaled = clipped.scale_by(scale, 1.0);
            let placed = scaled.offset(offset, 0.0);

            render_group.push(placed);
        }
    }

    render_group.into_rc()
}
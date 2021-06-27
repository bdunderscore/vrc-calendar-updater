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

#![allow(dead_code)]

mod calendar;
mod datastream;
mod render_prims;
mod event_info;

use anyhow::Result;
use thiserror::Error;

use pango::FontDescription;

use chrono::prelude::*;

use datastream::*;
use render_prims::*;

use cairo::Rectangle;
use std::rc::Rc;
use std::convert::{TryInto, TryFrom};

mod config;
use config::*;

use tracing::{debug, error, info, span, Level};

use clap::Clap;

#[derive(Clap)]
#[clap(version = "1.0", author = "bd_ <bdunderscore@fushizen.net>")]
struct Opts {
    #[clap(short, long)]
    branch_name: Option<String>,

    #[clap(short, long)]
    template_image: String,

    #[clap(short, long)]
    header_image: String,
    
    #[clap(short, long)]
    output: String,

    #[clap(short, long)]
    sample_data: bool,
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEvent {
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
    body: String,
}

#[derive(Clone, Debug)]
pub struct CalendarDay {
    date: Date<Local>,
    events: Vec<CalendarEvent>,
}

fn sample_data() -> Vec<CalendarDay> {
    vec![CalendarDay {
        date: Local.ymd(2020, 5, 30),
        events: vec![]
    }]
}

struct SetupInfo {
    branch_name: String,
    font_day_header: FontDescription,
    font_time: FontDescription,
    font_end_time: FontDescription,
    font_event_info: FontDescription,

    /// Template image used for the background
    template: RcRenderable,

    day_header_template: RcRenderable,

    /// Minimum amount of blank (background) space between the header and subsequent body data
    /// This is applied above and below the main event list, not to the header itself.
    header_template_margin: f64,
}

fn weekday_sigil(wd: chrono::Weekday) -> &'static str {
    match wd {
        Weekday::Mon => "月",
        Weekday::Tue => "火",
        Weekday::Wed => "水",
        Weekday::Thu => "木",
        Weekday::Fri => "金",
        Weekday::Sat => "土",
        Weekday::Sun => "日",
    }
}

fn format_start(event: &CalendarEvent) -> String {
    event.start_time.time().format("%H:%M").to_string()
}

fn format_end(event: &CalendarEvent) -> Option<String> {
    if event.end_time.is_none() {
        return None;
    }

    let end_time = event.end_time.unwrap();
    let start_date = event.start_time.date();
    let end_date = end_time.date();

    if start_date == end_date {
        Some(format!("~{}", end_time.time().format("%H:%M")))
    } else if start_date.succ() == end_date && end_time.time().hour() <= 3 {
        Some(format!("~{:02}:{:02}", end_time.time().hour() + 24, end_time.time().minute()))
    } else if start_date.succ() == end_date {
        Some(format!("~翌{}", end_time.time().format("%H:%M")))
    } else {
        Some(format!(
            "~{} ({}) {}",
            end_time.date().format("%m/%d"),
            weekday_sigil(end_date.weekday()),
            end_time.time().format("%H:%M")
        ))
    }
}

struct EventStackEntry {
    renderable: RcRenderable,
    colors: [u8; 4],
    is_day_header: bool
}

impl Renderable for EventStackEntry {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        self.renderable.render_internal(cr)
    }
    fn bounds(&self) -> (f64, f64) {
        self.renderable.bounds()
    }
}

impl Renderable for Vec<EventStackEntry> {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        let mut y = 0.0;

        for entry in self.iter() {
            entry.render_to(cr, (0.0, y))?;
            y += entry.height();
        }

        Ok(())
    }
    fn bounds(&self) -> (f64, f64) {
        let mut w = 0.0;
        let mut h = 0.0;

        for entry in self.iter() {
            let (ew, eh) = entry.bounds();
            w = f64::max(w, ew);
            h += eh;
        }

        return (w, h);
    }
}

struct EventMarker {
    is_ended: bool,
}

impl Renderable for EventMarker {
    fn render_internal(&self, cr: &mut cairo::Context) -> Result<()> {
        let marker_color: Color = if !self.is_ended {
            RGB_EVENT_MARKER.into()
        } else {
            RGB_TEXT_ENDED.into()
        };
        cr.translate(TIME_COL_RIGHT as f64, 0.0);

        // Set up clip mask first
        cr.new_path();
        cr.rectangle(
            EVENT_MARKER_CLIP - 0.1,
            -EVENT_MARKER_HEIGHT,
            EVENT_MARKER_WIDTH + 1.0,
            EVENT_MARKER_HEIGHT * 2.0,
        );
        cr.clip();

        cr.set_source_rgba(marker_color.r, marker_color.g, marker_color.b, 1.0);
        cr.new_path();
        cr.move_to(0.0, -EVENT_MARKER_HEIGHT / 2.0);
        cr.line_to(EVENT_MARKER_WIDTH as f64, 0.0);
        cr.line_to(0.0, EVENT_MARKER_HEIGHT / 2.0);
        cr.close_path();
        cr.fill();

        Ok(())
    }

    fn bounds(&self) -> (f64, f64) {
        (EVENT_MARKER_WIDTH, EVENT_MARKER_HEIGHT)
    }
}

use std::sync::atomic::{AtomicBool, Ordering};

fn layout_single_event(
    sample_context: &cairo::Context,
    setup: &SetupInfo,
    event: &CalendarEvent,
) -> Result<EventStackEntry> {
    let start_time_text = format_start(event);
    let end_time_text = format_end(event);

    let is_ended = event.end_time.map(|et| et < Local::now()).unwrap_or(false);

    let color_text: Color = if is_ended { RGB_TEXT_ENDED } else { RGB_TEXT }.into();
    let color_time: Color = if is_ended { RGB_TIME_ENDED } else { RGB_TIME }.into();

    let start_time_text = TextBox::new(
        sample_context,
        start_time_text,
        (TIME_COL_RIGHT - TIME_COL_LEFT) as f64,
        color_time,
        &setup.font_time,
        1,
    )?;

    let mut end_baseline = 0.0;
    let end_time_text = if let Some(end_time_text) = end_time_text {
        let text = TextBox::new(
            sample_context,
            end_time_text,
            (TIME_COL_RIGHT - TIME_COL_LEFT) as f64,
            color_time,
            &setup.font_end_time,
            1,
        )?;
        end_baseline = text.min_baseline();
        text.into_rc()
    } else {
        Pad::new(0.0, 0.0).into_rc()
    };

    let (start_width, _start_height) = start_time_text.bounds();
    let (end_width, _end_height) = end_time_text.bounds();

    let start_offset = TIME_COL_LEFT as f64 + 8.0;
    let end_offset = start_offset + start_width;

    let end_time_text = if end_offset + end_time_text.width() < TIME_COL_RIGHT as f64 {
        end_time_text.offset(end_offset, start_time_text.min_baseline() - end_baseline)
    } else {
        // Place this on the next line instead
        end_time_text.offset(
            TIME_COL_RIGHT as f64 - end_width,
            start_time_text.min_baseline(),
        )
    };
    let start_time_text = start_time_text.offset(start_offset, 0.0);

    let desc_text = TextBox::new(
        sample_context,
        event.body.clone(),
        (EVENT_INFO_RIGHT - EVENT_INFO_LEFT) as f64,
        color_text,
        &setup.font_event_info,
        2,
    )?;

    //let is_ended = desc_text.height() > 36.0; // XXX hack

    let mut render_group = RenderGroup::new();

    render_group.push(EventMarker { is_ended }.offset(0.0, start_time_text.height() / 2.0));
    render_group.push(start_time_text);
    render_group.push(end_time_text);
    render_group.push(desc_text.offset(EVENT_INFO_LEFT as f64, 0.0));

    Ok(EventStackEntry {
        renderable: render_group.into_rc(),
        is_day_header: false,
        colors: if is_ended {
            [PAL_TIME_ENDED, PAL_TEXT_ENDED, PAL_TEXT_ENDED, PAL_TEXT_ENDED]
        } else {
            [PAL_TIME, PAL_TEXT, PAL_TEXT, PAL_TEXT]
        }
    })
}


fn layout_day(
    sample_context: &cairo::Context,
    setup: &SetupInfo,
    day: &CalendarDay,
    mut entries: &mut Vec<EventStackEntry>,
) -> Result<()> {    
    let mut render_col = RenderColumn::new();

    let date_string = format!(
        "{} ({})",
        day.date.format("%m/%d"),
        weekday_sigil(day.date.weekday())
    );

    // First, slap down the header
    // TODO: Adjust x-pos

    let day_title = TextBox::new(
        sample_context,
        date_string,
        setup.day_header_template.width(),
        RGB_DATE.into(),
        &setup.font_day_header,
        1,
    )?;
    let center_width = (VARIABLE_OUTER_RIGHT - VARIABLE_OUTER_LEFT) as f64;
    let x_offset = (center_width - day_title.width()) / 2.0;
    let y_offset = (DAY_HEADER_HEIGHT as f64 - day_title.height()) / 2.0;

    let day_title = day_title
        .offset(VARIABLE_OUTER_LEFT as f64 + x_offset, y_offset);
    render_col.push(day_title);
    render_col.push(Pad::new(0.0, y_offset));

    entries.push(EventStackEntry {
        renderable: render_col.into_rc(),
        is_day_header: true,
        colors: [PAL_DATE; 4]
    });

    entries.push(
        EventStackEntry {
            renderable: Pad::new(0.0, setup.header_template_margin).into_rc(),
            is_day_header: false,
            colors: [PAL_TEXT;4]
        }
    );

    if day.events.is_empty() {
        let filler_text = TextBox::new(
            sample_context,
            "【イベント情報がありません】".into(),
            (VARIABLE_OUTER_RIGHT - VARIABLE_OUTER_LEFT) as f64,
            RGB_TEXT.into(),
            &setup.font_event_info,
            2,
        )?;

        let w = filler_text.width();

        let filler_text = filler_text.offset(
            VARIABLE_OUTER_LEFT as f64
                + ((VARIABLE_OUTER_RIGHT - VARIABLE_OUTER_LEFT) as f64 - w) / 2.0,
            0.0,
        );

        entries.push(EventStackEntry {
            renderable: filler_text.into_rc(),
            is_day_header: false,
            colors: [PAL_TEXT;4]
        });
    }

    // Render each event
    let mut prior_hour = None;
    for event in day.events.iter() {
        if let Some(prior_hour) = prior_hour {
            if prior_hour != event.start_time.hour() {
                entries.push(
                    EventStackEntry {
                        renderable: Separator {
                                color: RGB_TIME_DASH.into(),
                                width: (TIME_COL_RIGHT - TIME_COL_LEFT) as f64,
                                thickness: 2.0,
                                dash: 4.0,
                                margin: 4.0,
                            }
                            .offset(TIME_COL_LEFT as f64, 0.0)
                            .into_rc(),
                        is_day_header: false,
                        colors: [PAL_TIME_DASH;4]
                    }
                );
            }
        }
        prior_hour = Some(event.start_time.hour());

        entries.push(layout_single_event(sample_context, setup, event)?);
    }

    entries.push(
        EventStackEntry {
            renderable: Pad::new(0.0, setup.header_template_margin).into_rc(),
            is_day_header: false,
            colors: [PAL_TEXT;4]
        }
    );

    Ok(())
}

fn generate_variable_layout(
    sample_context: &cairo::Context,
    setup: &SetupInfo,
    days: &[CalendarDay],
    vdata: &mut Vec<VerticalData>,
    height_limit: usize
) -> Result<RcRenderable> {  
    let mut entries = vec![];
    let vdata_limit = height_limit;

    for day in days {
        layout_day(sample_context, setup, day, &mut entries)?;
    }

    let mut y : f64 = 0.0;
    vdata.reserve(entries.height().ceil() as usize);
    let mut prev_header = 0;

    'outer: for entry in entries.iter() {
        let initial_y = y.floor() as u32;
        y += entry.height();

        if entry.is_day_header {
            prev_header = vdata.len() as u32;
        }

        eprintln!("[{}..{}@{}] [dh={:?}] colors={:?}", initial_y, y, vdata.len(), entry.is_day_header, &entry.colors);

        while vdata.len() < y.ceil() as usize {
            if vdata.len() >= vdata_limit {
                break 'outer;
            }

            let col_info = if entry.is_day_header {
                let y : u32 = vdata.len().try_into()?;
                RowColorInfo::DayHeader { offset: y - initial_y }
            } else {
                RowColorInfo::Colors(entry.colors.clone())
            };

            vdata.push(VerticalData {
                prev_day_header: prev_header,
                col_info: col_info
            });
        }
    }

    Ok(entries.into_rc())
}

#[inline(never)]
fn squash_surface(mut surf: cairo::ImageSurface) -> Result<cairo::ImageSurface> {
    let tex_height_div = surf.get_height() / 3;

    let input_stride : usize = surf.get_stride().try_into()?;
    let width = surf.get_width();

    let input_chunk = input_stride * (usize::try_from(tex_height_div)?);

    let mut col_surf = cairo::ImageSurface::create(
        cairo::Format::Rgb24,
        width,
        tex_height_div
    ).map_err(convert_err)?;

    let width : usize = width.try_into()?;
    let output_stride : usize = col_surf.get_stride().try_into()?;

    let in_data = surf.get_data()?;
    let mut out_data = col_surf.get_data()?;

    dbg!(in_data.len());
    dbg!(tex_height_div);
    dbg!(input_stride);
    dbg!(tex_height_div as usize * input_stride);

    for (y, out_row) in out_data.chunks_exact_mut(output_stride).enumerate().take(tex_height_div as usize) {
        // B G R A
        for (x, px) in out_row.chunks_exact_mut(4).enumerate().take(width as usize) {
            px[0] = in_data[y * input_stride + x + 0 * input_chunk];
            px[1] = in_data[y * input_stride + x + 1 * input_chunk];
            px[2] = in_data[y * input_stride + x + 2 * input_chunk];
            px[3] = 0xFF;
        }
    }

    std::mem::drop(out_data);

    Ok(col_surf)
}

fn compute_layout(
    days: &[CalendarDay],
    setup: &SetupInfo,
    mut vdata: &mut Vec<VerticalData>,
    max_height: f64
) -> Result<RcRenderable> {
    let mut max_height = max_height.floor() as i32;

    info!("Generating layout");

    let tmp_surface =
        cairo::ImageSurface::create(cairo::Format::Rgb24, 512, 512).map_err(convert_err)?;
    let tmp_context = cairo::Context::new(&tmp_surface);

    let layout = generate_variable_layout(&tmp_context, setup, days, vdata, max_height as usize * 3)?;

    // Now render to a temporary image so we can split across RGB channels.
    let mut tex_height = layout.height().ceil() as i32;
    if tex_height % 3 < 0 {
        tex_height += 3 - (tex_height % 3);
    }

    let tex_height = std::cmp::min(tex_height, max_height * 3);

    let alpha_surf = cairo::ImageSurface::create(
        cairo::Format::A8,
        VIEWPORT_WIDTH as i32,
        tex_height
    ).map_err(convert_err)?;

    let mut context = cairo::Context::new(&alpha_surf);
    layout.render(&mut context)?;
    std::mem::drop(context);

    alpha_surf.flush();

    Ok(squash_surface(alpha_surf)?.into_rc())
}

fn setup_environment(opts: &Opts) -> Result<SetupInfo> {
    info!("Performing environment setup");

    let template = load_png_surface(&opts.template_image)?;
    let day_title = load_png_surface(&opts.header_image)?;

    // Determine scale factor
    let w_scale = 1024.0 / template.width();
    let template = template.scale_by(w_scale, w_scale);
    let day_title = day_title.scale_by(w_scale, w_scale);

    let template = template.into_rc();
    let day_title = day_title.into_rc();

    Ok(SetupInfo {
        branch_name: opts.branch_name.clone().unwrap_or("DEVEL".into()),
        font_day_header: FontDescription::from_string(FONT_DAY_HEADER),
        font_time: FontDescription::from_string(FONT_TIME),
        font_end_time: FontDescription::from_string(FONT_END_TIME),
        font_event_info: FontDescription::from_string(FONT_EVENT_INFO),
        template,
        day_header_template: day_title,
        header_template_margin: 16.0,
    })
}

fn info_text(setup: &SetupInfo, bounds: (f64, f64)) -> Result<RcRenderable> {
    dbg!(bounds);
    let info_str = format!("{} {}", Local::now().to_rfc3339(), &setup.branch_name);

    let tmp_surface =
    cairo::ImageSurface::create(cairo::Format::Rgb24, 512, 512).map_err(convert_err)?;
    let tmp_context = cairo::Context::new(&tmp_surface);

    let info_text = TextBox::new(
        &tmp_context,
        info_str,
        bounds.0,
        RGB_TEXT.into(),
        &FontDescription::from_string(FONT_CONFIG_INFO),
        1
    )?;
    let baseline = info_text.height();
    let info_text = info_text.offset(0.0, bounds.1 - baseline);

    Ok(info_text.into_rc())
}

fn template_column(setup: &SetupInfo, col: i32) -> (RcRenderable, f64, f64) {
    let clip = setup.template.clone().clip_to(Rectangle {
        x: (col * VARIABLE_OUTER_RIGHT) as f64,
        y: VARIABLE_TOP as f64,
        width: if col == 0 { LEFT_BORDER } else { RIGHT_BORDER } as f64,
        height: (VARIABLE_BOTTOM - VARIABLE_TOP) as f64
    });
    //let clip = FillRect::rect(Color { r: 1.0, g: col as f64, b: 1.0 }, LEFT_BORDER as f64, (VARIABLE_BOTTOM - VARIABLE_TOP) as f64);

    let (w, h) = clip.bounds();

    let clip = SwapXY::new(clip)
        .pad_vertical(SECTION_PAD, SECTION_PAD)
        .pad_sides(0.0, SECTION_PAD);

    (clip.into_rc(), w, h)
}

struct TemplateElementCoordinates {
    left_border: Rectangle,
    right_border: Rectangle,
    day_header_tex: Rectangle,
    day_header_true_size: (f64, f64),
    header: Rectangle,
    footer: Rectangle,
}

fn layout_template(setup: &SetupInfo, data: &mut DatastreamElements) -> Result<(RcRenderable, TemplateElementCoordinates)> {
    let template = &setup.template;

    let left_border;
    let right_border;

    let mut side_layout = RenderColumn::new();
    let (column, w, h) = template_column(&setup, 0);
    side_layout.push(column);
    left_border = Rectangle { x: 0.0, y: SECTION_PAD, width: w, height: h };

    let (column, w, h) = template_column(&setup, 1);
    right_border = Rectangle { x: 0.0, y: side_layout.height() + SECTION_PAD, width: w, height: h };
    side_layout.push(column);

    // Set up clipped day-header-template
    // TODO: Pad to line height
    let mut day_header = RenderGroup::new();

    let (w, h) = setup.day_header_template.bounds();
    let day_header_true_size = (w,h);
    const DAY_HEADER_CORNER_SIZE: f64 = 8.0;
    for cx in 0..2 {
        let cx : f64 = cx.into();

        let clip_x = (w - DAY_HEADER_CORNER_SIZE) * cx;
        let clip = setup.day_header_template.clone().clip_to(Rectangle {
            x: clip_x,
            width: DAY_HEADER_CORNER_SIZE,
            y: 0.0,
            height: h
        });
        
        let offset_x = DAY_HEADER_CORNER_SIZE * cx;
        day_header.push(clip.offset(offset_x, 0.0));
    }

    let day_header = day_header
        .pad_sides(SECTION_PAD, SECTION_PAD)
        .pad_vertical(0.0, SECTION_PAD);

        
    // Generate the alpha data as well
    let (dh_w, dh_h) = day_header.bounds();
    let mut day_header_alpha = RenderGroup::new();
    day_header_alpha.push(FillRect::rect(Color {r:1.0,g:1.0,b:1.0}, dh_w, dh_h));
    day_header_alpha.push(day_header.clone().with_operator(cairo::Operator::DestIn));

    let day_header_tex = Rectangle {
        x: SECTION_PAD + side_layout.width(),
        y: 0.0,
        height: h,
        width: DAY_HEADER_CORNER_SIZE * 2.0,
    };
    
    let mut init_seg = RenderGroup::new();
    let side_width = side_layout.width();
    init_seg.push(side_layout);
    init_seg.push(day_header.offset(side_width, 0.0));

    data.day_header_side_width = DAY_HEADER_CORNER_SIZE as u32;
    data.day_header_tex_x = (side_width + SECTION_PAD) as u32;
    data.day_header_tex_alpha_x = data.day_header_tex_x + data.day_header_side_width * 2 + (SECTION_PAD * 2.0) as u32;
    data.day_header_tex_y = 0;
    data.day_header_true_width = setup.day_header_template.width() as u32;
    data.day_header_height = setup.day_header_template.height() as u32;

    init_seg.push(day_header_alpha.offset(side_width + dh_w, 0.0));

    let (init_w, init_h) = init_seg.bounds();
    let init_w = init_w.ceil() as u32;
    let init_h = init_h.ceil() as u32;

    data.datastream_width = VIEWPORT_WIDTH - init_w;
    data.datastream_height = VIEWPORT_HEIGHT - init_h;

    let mut column = RenderColumn::new();
    column.push(init_seg);

    let y = column.height();
    let mut header_renderer = RenderGroup::new();
    header_renderer.push(template.clone().clip_to(Rectangle {
        x: 0.0,
        y: 0.0,
        width: template.width(),
        height: DAY_HEADER_CORNER_SIZE + VARIABLE_TOP as f64,
    }));
    header_renderer.push(setup.day_header_template.clone().clip_to(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: setup.day_header_template.width(),
            height: DAY_HEADER_CORNER_SIZE as f64
        }
        ).offset(
            LEFT_BORDER as f64,
            VARIABLE_TOP as f64
        )
    );

    column.push(pad_vertical(header_renderer, SECTION_PAD, SECTION_PAD));

    let header = Rectangle {
        x: 0.0,
        y: y + SECTION_PAD,
        width: template.width(),
        height: VARIABLE_TOP as f64,
    };

    data.header_tex_y = header.y as u32;
    
    let mut footer_tex = RenderGroup::new();
    let footer_height = template.height() - VARIABLE_BOTTOM as f64;
    footer_tex.push(template.clone().clip_to(Rectangle {
        x: 0.0,
        y: VARIABLE_BOTTOM as f64,
        width: template.width(),
        height: footer_height,
    }));
    footer_tex.push(info_text(setup, footer_tex.bounds())?);

    let y = column.height();
    column.push(footer_tex.pad_vertical(SECTION_PAD, SECTION_PAD));
    let footer = Rectangle {
        x: 0.0,
        y: y + SECTION_PAD,
        width: template.width(),
        height: footer_height
    };

    data.footer_tex_y = footer.y as u32;

    data.bg_sample_y = (column.height() + SECTION_PAD) as u32;
    let bg_sample_tex = setup.template.clone().clip_to(Rectangle {
        x: 0.0,
        y: VARIABLE_TEMPLATE_TOP as f64,
        height: data.bg_sample_h as f64,
        width: setup.template.width()
    });
    column.push(
        bg_sample_tex.pad_vertical(SECTION_PAD, SECTION_PAD));

    Ok((
        column.into_rc(),
        TemplateElementCoordinates {
            left_border,
            right_border,
            day_header_tex,
            day_header_true_size,
            header,
            footer
        }
    ))
}

fn compute_full_layout(setup: &SetupInfo, days: &Vec<CalendarDay>) -> Result<(RcRenderable, DatastreamElements)> {
    let mut data = config_datastream_info();

    let template = setup.template.clone();

    let mut layout = RenderColumn::new();

    let (header, coords) = layout_template(setup, &mut data)?;
    layout.push(header);

    let base_offset = layout.height();
    let event_info = compute_layout(&days, &setup, &mut data.vdata, TEXTURE_HEIGHT as f64 - layout.height() - SECTION_PAD)?;
    let (event_w, event_h) = event_info.bounds();

    data.scroll_height = event_h.ceil() as u32;
    data.scroll_tex_y = (base_offset + SECTION_PAD).ceil() as u32;

    layout.push(
        event_info
        .clip_to(Rectangle {
            x: LEFT_BORDER as f64,
            y: 0.0,
            width: event_w - ((LEFT_BORDER + RIGHT_BORDER) as f64),
            height: event_h
        })
        .pad_vertical(SECTION_PAD, 0.0)
        .pad_sides(0.0, SECTION_PAD)
    );

    let (_width, height) = layout.bounds();

    Ok((layout.into_rc(), data))
}

fn render_to_file(layout: &dyn Renderable, data: &DatastreamElements, filename: &str) -> anyhow::Result<()> {
    info!("Rendering...");

    let span = span!(Level::INFO, "render_to_file");
    let _enter = span.enter();

    let (width, height) = layout.bounds();
    let width = (width as usize).next_power_of_two();
    let height = (height as usize).next_power_of_two();

    let mut surface = cairo::ImageSurface::create(cairo::Format::Rgb24, width as i32, height as i32)
        .map_err(convert_err)?;
    let mut cairo_context = cairo::Context::new(&surface);

    // Fill background
    cairo_context.save();
    cairo_context.set_source_rgba(1.0, 0.0, 1.0, 1.0);
    cairo_context.rectangle(0.0, 0.0, width as f64, height as f64);
    cairo_context.set_operator(cairo::Operator::DestOver);
    cairo_context.fill();
    surface.flush();
    cairo_context.restore();
    cairo_context.reset_clip();
    cairo_context.new_path();

    layout.render_to(&mut cairo_context, (0.0, 0.0))?;

    // Render to file
    std::mem::drop(cairo_context);
    surface.flush();

    data.write(&mut surface)?;

    info!("Writing image...");

    let f = std::fs::File::create(filename)?;
    let mut f = std::io::BufWriter::new(f);

    surface.write_to_png(&mut f)?;

    Ok(())
}

fn print_char_stats(data: &[CalendarDay]) {
    use std::collections::HashMap;
    let mut map : HashMap<char, u32> = HashMap::new();

    for day in data.iter() {
        for event in day.events.iter() {
            for ch in event.body.chars() {
                (*map.entry(ch).or_insert(0)) += 1;
            }
        }
    }

    let count = map.len();
    map.retain(|k, v| *v > 1);

    let mut pairs : Vec<(char, u32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
    pairs.sort_by_key(|(_ch, count)| -(*count as i64));

    println!("Characters seen only once: {}", count - pairs.len());
    println!("Characters seen multiple times: {}", pairs.len());

    for (ch, count) in pairs.iter().copied() {
        println!("Character: {:?} count: {}", ch, count);
    }
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();

    tracing_subscriber::fmt::init();
    info!("Starting calendar generation");

    let setup = setup_environment(&opts)?;
    let days = if opts.sample_data { sample_data() } else { calendar::fetch_calendar()? };

    let (final_layout, data) = compute_full_layout(&setup, &days)?;
    dump_text_histograms();

    debug!("Final image size: {:?}", final_layout.bounds());

    render_to_file(&final_layout, &data, &opts.output)?;

    Ok(())
}

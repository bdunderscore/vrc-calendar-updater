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

use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;
use thiserror::Error;

use std::{collections::HashMap, convert::TryFrom};

use chrono::prelude::*;
use ical::parser::ical::component::IcalCalendar;

const CALENDAR_URL : &str = "https://calendar.google.com/calendar/ical/1b1et1slg27jm1rgdltu3mn2j4@group.calendar.google.com/public/basic.ics";

use super::CalendarEvent;

use tracing::{error, info};

#[derive(Error, Debug)]
enum CalendarFetchError {
    #[error("Calendar failed to parse: {0}")]
    ParserError(ical::parser::ParserError),
    #[error("Event is missing property: {0}")]
    MissingProperty(&'static str),
}

fn want_prop<'a>(
    map: &HashMap<&'a str, &'a ical::property::Property>,
    name: &'static str,
) -> Result<&'a str> {
    map.get(name)
        .and_then(|&p| p.value.as_ref())
        .map(|s| s.as_str())
        .ok_or(CalendarFetchError::MissingProperty(name).into())
}

fn parse_date(s: &str) -> Result<DateTime<Local>> {
    const ICAL_DATE_FMT: &'static str = "%Y%m%dT%H%M%S%#z";
    let fixed_date = DateTime::parse_from_str(s, ICAL_DATE_FMT)?;
    Ok(fixed_date.with_timezone(&Local))
}

fn want_date0<'a>(
    map: &HashMap<&'a str, &'a ical::property::Property>,
    name: &'static str,
) -> Result<DateTime<Local>> {
    const ICAL_DATE_FMT: &'static str = "%Y%m%dT%H%M%S%#z";
    let prop = want_prop(map, name)?;
    parse_date(prop)
}

fn want_date<'a>(
    map: &HashMap<&'a str, &'a ical::property::Property>,
    name: &'static str,
) -> Result<DateTime<Local>> {
    want_date0(map, name)
        .with_context(|| format!("Failed to parse or retrieve date property {:?}", name))
}

#[allow(dead_code)]
#[derive(Debug)]
struct ParsedEntry<'a> {
    dtstart: DateTime<Local>,
    dtend: Option<DateTime<Local>>,
    uid: &'a str,
    description: Option<&'a str>,
    summary: &'a str,
}

impl<'a> TryFrom<&'a ical::parser::ical::component::IcalEvent> for ParsedEntry<'a> {
    type Error = anyhow::Error;

    fn try_from(event: &'a ical::parser::ical::component::IcalEvent) -> Result<Self, Self::Error> {
        let mut hm = HashMap::with_capacity(event.properties.len());

        for prop in event.properties.iter() {
            hm.insert(prop.name.as_str(), prop);
        }

        Ok(ParsedEntry {
            dtstart: want_date(&hm, "DTSTART")?,
            dtend: hm
                .get("DTEND")
                .and_then(|p| p.value.as_ref())
                .map(|s| parse_date(&s))
                .transpose()
                .unwrap_or(None),
            uid: want_prop(&hm, "UID")?,
            description: hm
                .get("DESCRIPTION")
                .and_then(|e| e.value.as_ref())
                .map(|s| s.as_str()),
            summary: want_prop(&hm, "SUMMARY")?,
        })
    }
}

fn cal_error(e: ical::parser::ParserError) -> anyhow::Error {
    CalendarFetchError::ParserError(e).into()
}

#[tracing::instrument]
fn get_calendar_data() -> Result<IcalCalendar> {
    info!("Fetching ical data...");

    let data = reqwest::blocking::get(CALENDAR_URL)?
        .error_for_status()?
        .bytes()?;

    info!("Parsing ical data...");

    let mut ical = ical::IcalParser::new(&data[..]);

    ical.next()
        .ok_or_else(|| anyhow!("No calendars parsed"))?
        .map_err(cal_error)
}

fn unescape(s: &mut String) {
    use std::iter::Peekable;

    let mut s_tmp = String::with_capacity(s.len());
    let mut iter = s.chars();

    while let Some(c) = iter.next() {
        if c == '\\' {
            if let Some(c2) = iter.next() {
                if c2 == 'n' {
                    continue;
                } else {
                    s_tmp.push(c2);
                }
            }
        } else {
            s_tmp.push(c);
        }
    }

    *s = s_tmp;
}

pub(crate) fn fetch_calendar() -> Result<Vec<super::CalendarDay>> {
    let raw_data = get_calendar_data()?;

    let now = Local::now();
    let one_week_later = now
        .date()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("Date overflow")
        .and_hms(0, 0, 0);

    info!("Processing entries...");

    let mut events = Vec::with_capacity(raw_data.events.len());
    let mut parse_errors = 0;
    for event in raw_data.events.iter() {
        match ParsedEntry::try_from(event) {
            Ok(e) => events.push(e),
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse event: {}; raw event: {:?}",
                    e, event
                );
                parse_errors += 1;
                if parse_errors > 10 {
                    bail!("Too many parse errors");
                }
            }
        }
    }

    info!("Filtering entries...");

    let mut start_date = now.date();
    if now.time().hour() < 3 {
        start_date = start_date.pred();
    }

    events.retain(|ev| {
        (ev.dtstart.date() >= start_date && ev.dtstart < one_week_later)
            || ev
                .dtend
                .map(|end| ev.dtstart <= now && end >= now)
                .unwrap_or(false)
    });
    events.sort_by_key(|ev| (ev.dtstart, ev.dtend, ev.summary));

    info!("Generating final CalendarEvents...");

    let mut days = Vec::new();
    let group_by = events.iter().group_by(|&ev| ev.dtstart.date());
    for (date, daygroup) in &group_by {
        let mut events = Vec::new();

        for event in daygroup {
            let mut event = CalendarEvent {
                start_time: event.dtstart,
                end_time: event.dtend,
                body: event.summary.into(),
            };

            let prior_event = events.len().checked_sub(1)
                .map(|i| &events[i]);
            
            if Some(&event) == prior_event {
                continue;
            }

            unescape(&mut event.body);

            events.push(event);
        }

        days.push(super::CalendarDay { date, events });
    }

    Ok(days)
}

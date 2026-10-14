use chrono::{NaiveDate, TimeZone};
use chrono_tz::Tz;

pub(super) fn segment_stop(segment: &crate::storage::RecordingSegmentRow, now: i64) -> Option<i64> {
    segment
        .duration_ms
        .filter(|duration| *duration >= 0)
        .and_then(|duration| segment.started_at_ms.checked_add(duration))
        .or(segment.ended_at_ms)
        .or_else(|| (segment.state == "recording").then_some(now))
}

pub(crate) fn capture_dates(
    rows: &super::CaptureRows,
    input: &super::ListRecallDaysInput,
    now: i64,
) -> Result<std::collections::BTreeSet<String>, String> {
    let tz = input_timezone(input)?;
    let mut dates = std::collections::BTreeSet::new();
    for (_, segments) in rows {
        for segment in segments {
            let end = segment_stop(segment, now)
                .unwrap_or_else(|| segment.started_at_ms.saturating_add(1))
                .max(segment.started_at_ms.saturating_add(1));
            let Some(first) = chrono::Utc
                .timestamp_millis_opt(segment.started_at_ms)
                .single()
            else {
                continue;
            };
            let Some(last) = chrono::Utc
                .timestamp_millis_opt(end.saturating_sub(1))
                .single()
            else {
                continue;
            };
            let mut date = first.with_timezone(&tz).date_naive();
            let last = last.with_timezone(&tz).date_naive();
            if last.signed_duration_since(date).num_days() > 3660 {
                return Err("Invalid recording duration.".into());
            }
            while date <= last {
                let id = date.format("%Y-%m-%d").to_string();
                if input
                    .before_day_id
                    .as_ref()
                    .is_none_or(|before| &id < before)
                {
                    dates.insert(id);
                }
                date = date.succ_opt().ok_or("Invalid calendar day.")?;
            }
        }
    }
    Ok(dates)
}

pub(super) fn input_timezone(input: &super::ListRecallDaysInput) -> Result<Tz, String> {
    let timezone = input.timezone.parse().map_err(|_| "Invalid timezone.")?;
    if let Some(before) = &input.before_day_id {
        day_bounds(before, &input.timezone)?;
    }
    Ok(timezone)
}

pub fn day_bounds(day_id: &str, timezone: &str) -> Result<(i64, i64), String> {
    let date = NaiveDate::parse_from_str(day_id, "%Y-%m-%d")
        .map_err(|_| "Invalid calendar day.".to_string())?;
    if date.format("%Y-%m-%d").to_string() != day_id {
        return Err("Invalid calendar day.".into());
    }
    let tz: Tz = timezone
        .parse()
        .map_err(|_| "Invalid timezone.".to_string())?;
    let midnight = |date: NaiveDate| {
        // IANA zones can change at midnight or skip an entire date. The first
        // valid local instant, choosing the earlier fold, owns this boundary.
        for seconds in 0..86_400 {
            let local = date.and_hms_opt(0, 0, 0).unwrap() + chrono::Duration::seconds(seconds);
            if let Some(value) = tz.from_local_datetime(&local).earliest() {
                return Ok(value.timestamp_millis());
            }
        }
        Err("This calendar date does not exist in the selected timezone.".to_string())
    };
    let start = midnight(date)?;
    let mut next = date;
    for _ in 0..3 {
        next = next.succ_opt().ok_or("Invalid calendar day.")?;
        if let Ok(end) = midnight(next) {
            return Ok((start, end));
        }
    }
    Err("Calendar boundary is unavailable in the selected timezone.".into())
}

pub fn local_timezone() -> Result<String, String> {
    if let Ok(zone) = std::env::var("TZ") {
        if zone.parse::<Tz>().is_ok() {
            return Ok(zone);
        }
    }
    if let Ok(zone) = std::fs::read_to_string("/etc/timezone") {
        let zone = zone.trim();
        if zone.parse::<Tz>().is_ok() {
            return Ok(zone.into());
        }
    }
    if let Ok(path) = std::fs::canonicalize("/etc/localtime") {
        let path = path.to_string_lossy();
        if let Some((_, zone)) = path.split_once("/zoneinfo/") {
            if zone.parse::<Tz>().is_ok() {
                return Ok(zone.into());
            }
        }
    }
    Err("Device timezone is unavailable. Select a timezone for this day.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_day_preserves_dst_and_skipped_dates() {
        let (a, b) = day_bounds("2026-03-29", "Europe/Berlin").unwrap();
        assert_eq!(b - a, 23 * 3_600_000);
        let (a, b) = day_bounds("2026-10-25", "Europe/Berlin").unwrap();
        assert_eq!(b - a, 25 * 3_600_000);
        assert!(day_bounds("2026-02-30", "Europe/Berlin").is_err());
        let (a, b) = day_bounds("2011-12-29", "Pacific/Apia").unwrap();
        assert_eq!(b - a, 24 * 3_600_000);
        assert!(day_bounds("2011-12-30", "Pacific/Apia").is_err());
    }
}

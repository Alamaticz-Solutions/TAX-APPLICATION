use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};
use serde_json::{to_value, Value};

use crate::{model_metadata::RuntimeDataType, QueryBuildError, RuntimeError};

pub fn get_period(
    period: impl AsRef<str>,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    get_period_at(period, data_type, Utc::now().naive_local())
}

pub fn get_period_at(
    period: impl AsRef<str>,
    data_type: RuntimeDataType,
    now: NaiveDateTime,
) -> Result<(Value, Value), RuntimeError> {
    let period = period.as_ref();
    let today = now.date();

    match period {
        "_last_yr" => get_year_bound(today, Bound::Last, data_type),
        "_this_yr" => get_year_bound(today, Bound::This, data_type),
        "_next_yr" => get_year_bound(today, Bound::Next, data_type),

        "_last_qtr" => get_quarter_bound(today, Bound::Last, data_type),
        "_this_qtr" => get_quarter_bound(today, Bound::This, data_type),
        "_next_qtr" => get_quarter_bound(today, Bound::Next, data_type),

        "_last_mo" => get_month_bound(today, Bound::Last, data_type),
        "_this_mo" => get_month_bound(today, Bound::This, data_type),
        "_next_mo" => get_month_bound(today, Bound::Next, data_type),

        "_last_wk" => get_week_bound(today, Bound::Last, data_type),
        "_this_wk" => get_week_bound(today, Bound::This, data_type),
        "_next_wk" => get_week_bound(today, Bound::Next, data_type),

        "_yesterday" => get_day_bound(today, Bound::Last, data_type),
        "_today" => get_day_bound(today, Bound::This, data_type),
        "_tomorrow" => get_day_bound(today, Bound::Next, data_type),

        "_last_hr" | "_this_hr" | "_next_hr" => {
            if data_type != RuntimeDataType::DateTime {
                return Err(QueryBuildError::UnsupportedTimePeriod {
                    period: period.to_string(),
                    data_type: format!("{data_type:?}"),
                }
                .into());
            }

            let bound = match period {
                "_last_hr" => Bound::Last,
                "_this_hr" => Bound::This,
                "_next_hr" => Bound::Next,
                _ => unreachable!("matched hour periods above"),
            };
            get_hour_bound(now, bound)
        }

        _ => Err(QueryBuildError::InvalidTimePeriod(period.to_string()).into()),
    }
}

#[derive(Clone, Copy)]
enum Bound {
    Last,
    This,
    Next,
}

fn get_year_bound(
    today: NaiveDate,
    bound: Bound,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    let year = today.year() + bound_offset(bound);
    let start = date(year, 1, 1)?;
    let end = date(year + 1, 1, 1)?;
    date_values(start, end, data_type)
}

fn get_quarter_bound(
    today: NaiveDate,
    bound: Bound,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    let this_start_month = ((today.month() - 1) / 3) * 3 + 1;
    let this_start = date(today.year(), this_start_month, 1)?;
    let start = add_months(this_start, bound_offset(bound) * 3)?;
    let end = add_months(start, 3)?;
    date_values(start, end, data_type)
}

fn get_month_bound(
    today: NaiveDate,
    bound: Bound,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    let this_start = date(today.year(), today.month(), 1)?;
    let start = add_months(this_start, bound_offset(bound))?;
    let end = add_months(start, 1)?;
    date_values(start, end, data_type)
}

fn get_week_bound(
    today: NaiveDate,
    bound: Bound,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    let week_start = today.week(chrono::Weekday::Sun).first_day();
    let start = week_start + Duration::days((bound_offset(bound) * 7) as i64);
    let end = start + Duration::days(7);
    date_values(start, end, data_type)
}

fn get_day_bound(
    today: NaiveDate,
    bound: Bound,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    let start = today + Duration::days(bound_offset(bound) as i64);
    let end = start + Duration::days(1);
    date_values(start, end, data_type)
}

fn get_hour_bound(now: NaiveDateTime, bound: Bound) -> Result<(Value, Value), RuntimeError> {
    let this_hour_start = date(now.year(), now.month(), now.day())?
        .and_hms_opt(now.hour(), 0, 0)
        .ok_or_else(|| {
            QueryBuildError::InvalidDateBound(format!(
                "invalid hour start for {}-{}-{} {}:00:00",
                now.year(),
                now.month(),
                now.day(),
                now.hour()
            ))
        })?;
    let start = this_hour_start + Duration::hours(bound_offset(bound) as i64);
    let end = start + Duration::hours(1);
    datetime_values(start, end)
}

fn bound_offset(bound: Bound) -> i32 {
    match bound {
        Bound::Last => -1,
        Bound::This => 0,
        Bound::Next => 1,
    }
}

fn add_months(date_value: NaiveDate, offset: i32) -> Result<NaiveDate, RuntimeError> {
    let total_months = date_value.year() * 12 + date_value.month0() as i32 + offset;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) as u32 + 1;
    date(year, month, 1)
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate, RuntimeError> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        QueryBuildError::InvalidDateBound(format!("{year:04}-{month:02}-{day:02}")).into()
    })
}

fn date_values(
    start: NaiveDate,
    end: NaiveDate,
    data_type: RuntimeDataType,
) -> Result<(Value, Value), RuntimeError> {
    if data_type == RuntimeDataType::DateTime {
        return datetime_values(
            start
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time"),
            end.and_hms_opt(0, 0, 0).expect("midnight is a valid time"),
        );
    }
    values(start, end)
}

fn datetime_values(
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Result<(Value, Value), RuntimeError> {
    values(start.and_utc().to_rfc3339(), end.and_utc().to_rfc3339())
}

fn values<T>(start: T, end: T) -> Result<(Value, Value), RuntimeError>
where
    T: serde::Serialize,
{
    Ok((
        to_value(start).map_err(|e| QueryBuildError::SerializeValue(e.to_string()))?,
        to_value(end).map_err(|e| QueryBuildError::SerializeValue(e.to_string()))?,
    ))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    fn fixed_now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 5, 29)
            .expect("date")
            .and_hms_opt(15, 45, 0)
            .expect("time")
    }

    #[test]
    fn date_period_bounds_are_runtime_owned() {
        let (start, end) =
            get_period_at("_this_mo", RuntimeDataType::Date, fixed_now()).expect("month period");

        assert_eq!(start, json!("2026-05-01"));
        assert_eq!(end, json!("2026-06-01"));
    }

    #[test]
    fn datetime_period_bounds_are_runtime_owned() {
        let (start, end) =
            get_period_at("_last_hr", RuntimeDataType::DateTime, fixed_now()).expect("hour period");

        assert_eq!(start, json!("2026-05-29T14:00:00+00:00"));
        assert_eq!(end, json!("2026-05-29T15:00:00+00:00"));
    }

    #[test]
    fn invalid_periods_return_runtime_query_errors() {
        assert!(matches!(
            get_period_at("_last_hr", RuntimeDataType::Date, fixed_now()),
            Err(RuntimeError::QueryBuild(
                QueryBuildError::UnsupportedTimePeriod { .. }
            ))
        ));
        assert!(matches!(
            get_period_at("_sometimes", RuntimeDataType::Date, fixed_now()),
            Err(RuntimeError::QueryBuild(
                QueryBuildError::InvalidTimePeriod(_)
            ))
        ));
    }
}

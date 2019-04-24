
use std::time::SystemTime;

use chrono::offset::{Local, TimeZone, Utc};
use chrono::{NaiveDateTime, DateTime};
use rusqlite::types::ToSql;
use rusqlite::Connection;

use crate::errors::{AppError, AppResult};



#[allow(clippy::option_option)]
pub fn get_value(conn: &Connection, key: &str) -> AppResult<Option<Option<String>>> {
    get_value_with_default(conn, key, None)
}

#[allow(clippy::type_complexity)]
pub fn get_value_and_ttl(conn: &Connection, key: &str) -> AppResult<Option<(Option<String>, Option<DateTime<Utc>>)>> {
    let result = conn.query_row("SELECT value, expired_at FROM h WHERE key = ?;", &[key], |row| (row.get(0), row.get(1)));
    match result {
        Ok((value, expired_at)) => {
            if let Some(expired_at) = expired_at {
                if is_expired()(&expired_at) {
                    remove(conn, key)?;
                    return Ok(None);
                }
            }
            Ok(Some((value, expired_at)))
        },
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(AppError::Sql(err)),
    }
}

#[allow(clippy::option_option)]
pub fn get_value_with_default(conn: &Connection, key: &str, default: Option<&str>) -> AppResult<Option<Option<String>>> {
    if let Some((value, _)) = get_value_and_ttl(conn, key)? {
        Ok(Some(value.or_else(|| default.map(Into::into))))
    } else {
        Ok(None)
    }
}

pub fn is_expired() -> Box<Fn(&DateTime<Utc>) -> bool> {
    let now: DateTime<Utc> = SystemTime::now().into();
    Box::new(move |expired_at| *expired_at <= now)
}

pub fn modify_value(conn: &Connection, key: &str, delta: Option<&str>, minus: bool, ttl: Option<&str>) -> AppResult<f64> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get_value(conn, key)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, Some(&format!("{}", modified)), ttl)?;

    Ok(modified)
}

pub fn set(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    let expired_at = if let Some(ttl) = ttl {
        Some(parse_ttl(ttl)?)
    } else {
        None
    };
    set_value(conn, key, value, expired_at)
}

#[allow(clippy::option_option)]
pub fn p(found: &Option<Option<String>>) -> bool {
    if let Some(ref found) = *found {
        if let Some(ref value) = *found {
            println!("{}", value);
        }
        true
    } else {
        false
    }
}

pub fn parse_ttl(s: &str) -> AppResult<DateTime<Utc>> {
    let parse = |format: &'static str, suffix: &'static str| -> AppResult<DateTime<Local>> {
        let dt = NaiveDateTime::parse_from_str(&format!("{}{}", s, suffix), format)?;
        Ok(Local.from_local_datetime(&dt).unwrap())
    };

    parse("%Y-%m-%d %H:%M:%S", "")
        .or_else(|_| parse("%Y/%m/%d %H:%M:%S", ""))
        .or_else(|_| parse("%Y-%m-%d %H:%M:%S", " 00:00:00"))
        .or_else(|_| parse("%Y/%m/%d %H:%M:%S", " 00:00:00"))
        .or_else(|_| -> AppResult<DateTime<Local>>{
            let ttl = humantime::parse_duration(s)?;
            let now = SystemTime::now();
            let expired_at: SystemTime = now + ttl;
            Ok(expired_at.into())
        }).map(|it| it.with_timezone(&Utc))
}

pub fn set_value(conn: &Connection, key: &str, value: Option<&str>, expired_at: Option<DateTime<Utc>>) -> AppResult<bool> {
    let now: DateTime<Utc> = SystemTime::now().into();

    let updated = conn.execute(
        "UPDATE h SET value = ?, updated_at = ?, expired_at = ? WHERE key = ?",
        &[&value as &ToSql, &now as &ToSql, &expired_at as &ToSql, &key]
    )?;
    match updated {
        0 => {
            conn.execute(
                "INSERT INTO h SELECT ?, ?, ?, ?, ?",
                &[&key, &value as &ToSql, &now, &now, &expired_at as &ToSql]
            )?;
        },
        1 => (),
        n => panic!("UPDATE has returned: {}", n),
    }

    Ok(true)
}

#[allow(clippy::option_option)]
pub fn swap_values(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<Option<Option<String>>> {
    let result = get_value(conn, key)?;
    set(conn, key, value, ttl)?;
    Ok(result)
}

pub fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM h WHERE key = ?", &[&key])?;
    Ok(n == 1)
}



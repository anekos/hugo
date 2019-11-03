
use std::time::SystemTime;

use chrono::offset::Utc;
use chrono::DateTime;
use rusqlite::types::ToSql;
use rusqlite::Connection;

use crate::errors::{AppError, AppResult, AppResultU};
use super::Input;



#[allow(clippy::option_option)]
pub fn get_value(conn: &Connection, key: &str, expired_at: Option<DateTime<Utc>>, refresh: bool) -> AppResult<Option<Option<String>>> {
    get_value_with_default(conn, key, None, refresh, expired_at)
}

#[allow(clippy::type_complexity)]
pub fn get_value_and_expired_at(conn: &Connection, key: &str) -> AppResult<Option<(Option<String>, Option<DateTime<Utc>>)>> {
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
pub fn get_value_with_default(conn: &Connection, key: &str, default: Option<&str>, refresh: bool, expired_at: Option<DateTime<Utc>>) -> AppResult<Option<Option<String>>> {
    if let Some((value, _)) = get_value_and_expired_at(conn, key)? {
        if refresh {
            if let Some(expired_at) = expired_at {
                set_expired_at(conn, key, expired_at)?;
            } else {
                return Err(AppError::NoTtlForRefresh);
            }
        }
        Ok(Some(value.or_else(|| default.map(Into::into))))
    } else {
        Ok(None)
    }
}

pub fn is_expired() -> Box<dyn Fn(&DateTime<Utc>) -> bool> {
    let now: DateTime<Utc> = SystemTime::now().into();
    Box::new(move |expired_at| *expired_at <= now)
}

pub fn modify_value(conn: &Connection, key: &str, delta: Input, minus: bool, expired_at: Option<DateTime<Utc>>, refresh: bool) -> AppResult<f64> {
    let delta = delta.value()?;
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get_value(conn, key, expired_at, refresh)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, Input::new(Some(&format!("{}", modified)), false), expired_at)?;

    Ok(modified)
}

pub fn set(conn: &Connection, key: &str, input: Input, expired_at: Option<DateTime<Utc>>) -> AppResult<bool> {
    set_value(conn, key, input, expired_at)
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

pub fn set_expired_at(conn: &Connection, key: &str, expired_at: DateTime<Utc>) -> AppResultU {
    conn.execute(
        "UPDATE h SET expired_at = ? WHERE key = ?",
        &[&expired_at as &dyn ToSql, &key]
    )?;
    Ok(())
}

pub fn set_value(conn: &Connection, key: &str, input: Input, expired_at: Option<DateTime<Utc>>) -> AppResult<bool> {
    let now: DateTime<Utc> = SystemTime::now().into();
    let value = input.value()?;

    let updated = conn.execute(
        "UPDATE h SET value = ?, updated_at = ?, expired_at = ? WHERE key = ?",
        &[&value as &dyn ToSql, &now as &dyn ToSql, &expired_at as &dyn ToSql, &key]
    )?;
    match updated {
        0 => {
            conn.execute(
                "INSERT INTO h SELECT ?, ?, ?, ?, ?",
                &[&key, &value as &dyn ToSql, &now, &now, &expired_at as &dyn ToSql]
            )?;
        },
        1 => (),
        n => panic!("UPDATE has returned: {}", n),
    }

    Ok(true)
}

#[allow(clippy::option_option)]
pub fn swap_values(conn: &Connection, key: &str, input: Input, expired_at: Option<DateTime<Utc>>, refresh: bool) -> AppResult<Option<Option<String>>> {
    let result = get_value(conn, key, expired_at, refresh)?;
    set(conn, key, input, expired_at)?;
    Ok(result)
}

pub fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM h WHERE key = ?", &[&key])?;
    Ok(n == 1)
}



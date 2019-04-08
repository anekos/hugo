
use std::path::Path;
use std::time::SystemTime;

#[cfg(any(unix))] use std::os::unix::process::CommandExt;
#[cfg(any(unix))] use std::process::Command;

use chrono::{NaiveDateTime, DateTime};
use chrono::offset::{Local, TimeZone, Utc};
use if_let_return::if_let_some;
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};

use crate::errors::{AppError, AppResult, AppResultU};
use crate::types::*;



pub fn check(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    swap_values(conn, key, value, ttl).map(|it| it.is_some())
}

pub fn get(conn: &Connection, key: &str, default: Option<&str>) -> AppResult<bool> {
    Ok(p(&get_value_with_default(conn, key, default)?))
}

pub fn gc(conn: &Connection) -> AppResult<bool> {
    let mut stmt = conn.prepare("SELECT key, expired_at FROM h WHERE expired_at IS NOT NULL;")?;

    let entries = stmt.query_map(NO_PARAMS, |row| -> (String, DateTime<Utc>) {
        (row.get(0), row.get(1))
    })?;

    let is_expired = is_expired();

    for entry in entries {
        let (ref key, ref expired_at) = entry?;
        if is_expired(expired_at) {
            remove(conn, key)?;
        }
    }

    Ok(true)
}

pub fn has(conn: &Connection, key: &str) -> AppResult<bool> {
    get_value(conn, key).map(|it| it.is_some())
}

pub fn import(conn: &Connection, source_path: &str) -> AppResult<bool> {
    let source_conn = Connection::open(source_path)?;

    let mut stmt = source_conn.prepare("SELECT key, value, created_at, updated_at, expired_at FROM h;").unwrap();
    let entry_iter = stmt.query_map(NO_PARAMS, |row| {
        Entry {
            key: row.get(0),
            value: row.get(1),
            created_at: row.get(2),
            updated_at: row.get(3),
            expired_at: row.get(4),
        }
    }).unwrap();

    let mut result = true;
    for entry in entry_iter {
        let entry = entry?;
        result &= set_value(conn, &entry.key, entry.value.as_ref().map(String::as_ref), entry.expired_at)?;
    }

    Ok(result)
}

pub fn modify(conn: &Connection, key: &str, delta: Option<&str>, minus: bool, ttl: Option<&str>) -> AppResult<bool> {
    let result = modify_value(conn, key, delta, minus, ttl)?;
    println!("{}", result);
    Ok(true)
}

pub fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM h WHERE key = ?", &[&key])?;
    Ok(n == 1)
}

pub fn set(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    let expired_at = if let Some(ttl) = ttl {
        Some(parse_ttl(ttl)?)
    } else {
        None
    };
    set_value(conn, key, value, expired_at)
}

pub fn shell(path: &Path, args: Option<&[&str]>) -> AppResult<bool> {
    let mut command = Command::new("sqlite3");
    command.arg(path);
    if let Some(args) = args {
        command.args(args);
    }
    command.exec();
    Ok(true)
}

pub fn swap(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    Ok(p(&swap_values(conn, key, value, ttl)?))
}

pub fn ttl(conn: &Connection, key: &str, ttl: Option<&str>) -> AppResult<bool> {
    if_let_some!((_, expired_at) = get_value_and_ttl(conn, key)?, Ok(false));

    if let Some(ttl) = ttl {
        let expired_at = parse_ttl(ttl)?;
        let updated = conn.execute(
            "UPDATE h SET expired_at = ? WHERE key = ?",
            &[&expired_at as &ToSql, &key]
        )?;
        if updated != 1 {
            panic!("WTF!");
        }
    } else if let Some(expired_at) = expired_at {
        let expired_at = expired_at.with_timezone(&Local);
        println!("{}", expired_at.format("%Y-%m-%d %H:%M:%S"));
    }

    Ok(true)
}

pub fn vacuum(conn: &Connection) -> AppResultU {
    conn.execute("VACUUM", NO_PARAMS)?;
    Ok(())
}



#[allow(clippy::option_option)]
fn get_value(conn: &Connection, key: &str) -> AppResult<Option<Option<String>>> {
    get_value_with_default(conn, key, None)
}

#[allow(clippy::type_complexity)]
fn get_value_and_ttl(conn: &Connection, key: &str) -> AppResult<Option<(Option<String>, Option<DateTime<Utc>>)>> {
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
fn get_value_with_default(conn: &Connection, key: &str, default: Option<&str>) -> AppResult<Option<Option<String>>> {
    if let Some((value, _)) = get_value_and_ttl(conn, key)? {
        Ok(Some(value.or_else(|| default.map(Into::into))))
    } else {
        Ok(None)
    }
}

fn is_expired() -> Box<Fn(&DateTime<Utc>) -> bool> {
    let now: DateTime<Utc> = SystemTime::now().into();
    Box::new(move |expired_at| *expired_at <= now)
}

fn modify_value(conn: &Connection, key: &str, delta: Option<&str>, minus: bool, ttl: Option<&str>) -> AppResult<f64> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get_value(conn, key)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, Some(&format!("{}", modified)), ttl)?;

    Ok(modified)
}

#[allow(clippy::option_option)]
fn p(found: &Option<Option<String>>) -> bool {
    if let Some(ref found) = *found {
        if let Some(ref value) = *found {
            println!("{}", value);
        }
        true
    } else {
        false
    }
}

fn parse_ttl(s: &str) -> AppResult<DateTime<Utc>> {
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
fn swap_values(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<Option<Option<String>>> {
    let result = get_value(conn, key)?;
    set(conn, key, value, ttl)?;
    Ok(result)
}

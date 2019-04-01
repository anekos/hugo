
use std::path::Path;
use std::time::{Duration, SystemTime};

#[cfg(any(unix))] use std::os::unix::process::CommandExt;
#[cfg(any(unix))] use std::process::Command;

use chrono::DateTime;
use chrono::offset::Utc;
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};

use crate::errors::{AppResult, AppResultU};
use crate::types::*;



pub static USAGE: &'static str = include_str!("usage.txt");


pub fn get(conn: &Connection, key: &str, default: Option<&str>) -> AppResult<bool> {
    Ok(p(&get_value_with_default(conn, key, default)?))
}

#[allow(clippy::option_option)]
fn get_value(conn: &Connection, key: &str) -> AppResult<Option<Option<String>>> {
    get_value_with_default(conn, key, None)
}

#[allow(clippy::option_option)]
fn get_value_with_default(conn: &Connection, key: &str, default: Option<&str>) -> AppResult<Option<Option<String>>> {
    use rusqlite::Error::QueryReturnedNoRows;

    let result = conn.query_row("SELECT value, expired_at FROM flags WHERE key = ?;", &[key], |row| {
        (row.get(0), row.get(1))
    });

    match result {
        Ok((value, expired_at)) => {
            let now: DateTime<Utc> = SystemTime::now().into();
            let expired_at: Option<DateTime<Utc>> = expired_at;
            if let Some(expired_at) = expired_at {
                if expired_at <= now {
                    remove(conn, key)?;
                    return Ok(None)
                }
            }
            Ok(value)
        },
        Err(ref err) => {
            if let QueryReturnedNoRows = *err {
                Ok(default.map(|it| Some(it.to_owned())))
            } else {
                Ok(None)
            }
        }
    }
}

pub fn has(conn: &Connection, key: &str) -> AppResult<bool> {
    get_value(conn, key).map(|it| it.is_some())
}

pub fn set(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    let now = time::get_time();
    conn.execute(
        "UPDATE flags SET value = ?, updated_at = ? WHERE key = ?",
        &[&value as &ToSql, &now as &ToSql, &key]
    )?;
    conn.execute(
        "INSERT INTO flags SELECT ?, ?, ?, ?, NULL WHERE (SELECT changes() = 0)",
        &[&key, &value as &ToSql, &now, &now]
    )?;

    set_ttl_opt(conn, key, ttl)?;
    Ok(true)
}

pub fn modify(conn: &Connection, key: &str, delta: Option<&str>, minus: bool, ttl: Option<&str>) -> AppResult<bool> {
    let result = modify_value(conn, key, delta, minus, ttl)?;
    println!("{}", result);
    Ok(true)
}

fn modify_value(conn: &Connection, key: &str, delta: Option<&str>, minus: bool, ttl: Option<&str>) -> AppResult<f64> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get_value(conn, key)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, Some(&format!("{}", modified)), ttl)?;

    Ok(modified)
}

pub fn swap(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    Ok(p(&swap_values(conn, key, value, ttl)?))
}

#[allow(clippy::option_option)]
fn swap_values(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<Option<Option<String>>> {
    let result = get_value(conn, key)?;
    set(conn, key, value, ttl)?;
    Ok(result)
}

pub fn check(conn: &Connection, key: &str, value: Option<&str>, ttl: Option<&str>) -> AppResult<bool> {
    swap_values(conn, key, value, ttl).map(|it| it.is_some())
}

pub fn import(conn: &Connection, source_path: &str) -> AppResult<bool> {
    let source_conn = Connection::open(source_path)?;

    let mut stmt = source_conn.prepare("SELECT key, value, created_at, updated_at FROM flags;").unwrap();
    let entry_iter = stmt.query_map(NO_PARAMS, |row| {
        Entry {
            key: row.get(0),
            value: row.get(1),
            created_at: row.get(2),
            updated_at: row.get(3)
        }
    }).unwrap();

    let mut result = true;
    for entry in entry_iter {
        let entry = entry?;
        result &= set(conn, &entry.key, entry.value.as_ref().map(String::as_ref), None)?;
    }

    Ok(result)
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

pub fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM flags WHERE key = ?", &[&key])?;
    Ok(n == 1)
}

fn set_ttl(conn: &Connection, key: &str, ttl: &str) -> AppResultU {
    let now = SystemTime::now();
    let ttl: Duration = ttl.parse::<humantime::Duration>()?.into();
    let expired_at = now + ttl;
    let expired_at: DateTime<Utc> = expired_at.into();

    let updated = conn.execute(
        "UPDATE flags SET expired_at = ? WHERE key = ?",
        &[&expired_at as &ToSql, &key]
    )?;
    if updated != 1 {
        panic!("WTF!");
    }
    Ok(())
}

fn set_ttl_opt(conn: &Connection, key: &str, ttl: Option<&str>) -> AppResultU {
    if let Some(ttl) = ttl {
        set_ttl(conn, key, ttl)?;
    }
    Ok(())
}


pub fn usage() {
    eprintln!("{}", USAGE);
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

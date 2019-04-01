
use std::path::Path;

#[cfg(any(unix))] use std::os::unix::process::CommandExt;
#[cfg(any(unix))] use std::process::Command;

use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};

use crate::errors::AppResult;
use crate::types::*;



pub static USAGE: &'static str = include_str!("usage.txt");


#[allow(clippy::option_option)]
pub fn get(conn: &Connection, key: &str, default: Option<String>) -> AppResult<Option<Option<String>>> {
    use rusqlite::Error::QueryReturnedNoRows;

    let result = conn.query_row("SELECT value FROM flags WHERE key = ?;", &[&key], |row| {
        row.get(0)
    });

    if let Err(ref err) = result {
        if let QueryReturnedNoRows = *err {
             return Ok(default.map(Some));
        }
    }

    Ok(result.map(Some)?)
}

pub fn has(conn: &Connection, key: &str) -> AppResult<bool> {
    get(conn, key, None).map(|it| it.is_some())
}

pub fn set(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<bool> {
    let now = time::get_time();
    conn.execute(
        "UPDATE flags SET value = ?, updated_at = ? WHERE key = ?",
        &[value, &now as &ToSql, &key]
    )?;
    conn.execute(
        "INSERT INTO flags SELECT ?, ?, ?, ? WHERE (SELECT changes() = 0)",
        &[&key, value as &ToSql, &now, &now]
    )?;
    Ok(true)
}

pub fn modify(conn: &Connection, key: &str, delta: &Option<String>, minus: bool) -> AppResult<f64> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get(conn, key, None)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, &Some(format!("{}", modified)))?;

    Ok(modified)
}

#[allow(clippy::option_option)]
pub fn swap(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<Option<Option<String>>> {
    let result = get(conn, key, None)?;
    set(conn, key, value)?;
    Ok(result)
}

pub fn check(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<bool> {
    swap(conn, key, value).map(|it| it.is_some())
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
        result &= set(conn, &entry.key, &entry.value)?;
    }

    Ok(result)
}

pub fn shell(path: &Path, args: &[String]) -> AppResult<bool> {
    Command::new("sqlite3")
        .arg(path)
        .args(args)
        .exec();
    Ok(true)
}

pub fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM flags WHERE key = ?", &[&key])?;
    Ok(n == 1)
}


#[allow(clippy::option_option)]
pub fn print_value(found: &Option<Option<String>>) -> bool {
    if let Some(ref found) = *found {
        if let Some(ref value) = *found {
            println!("{}", value);
        }
        true
    } else {
        false
    }
}

pub fn usage() {
    eprintln!("{}", USAGE);
}


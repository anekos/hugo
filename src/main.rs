use std::env::args;
use std::fs::create_dir_all;
use std::io::sink;
use std::path::Path;
use std::process::exit;

#[cfg(any(unix))] use std::os::unix::process::CommandExt;
#[cfg(any(unix))] use std::process::Command;

use app_dirs::*;
use argparse::{ArgumentParser, StoreTrue, StoreOption, Store};
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};

mod errors;

use errors::{AppError, AppResult, AppResultU};



type Key = String;
const APP_INFO: AppInfo = AppInfo { name: "hugo", author: "anekos" };
pub static USAGE: &'static str = include_str!("usage.txt");

enum Operation {
    Has(Key),
    Get(Key, Option<String>),
    Check(Key, Option<String>),
    Set(Key, Option<String>),
    Swap(Key, Option<String>),
    Modify(Key, Option<String>, bool),
    Import(String),
    Remove(Key),
    Shell(Vec<String>),
}

#[derive(Debug)]
struct Entry {
    key: String,
    value: Option<String>,
    created_at: String,
    updated_at: String,
}





fn main() {
    match app() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}\n", e);
            usage();
            exit(2)
        },
    }
}


fn parse_args() -> AppResult<(String, bool, Operation)> {
    use self::Operation::*;

    let mut is_path = false;
    let mut name = "".to_owned();
    let mut op = "".to_owned();
    let mut key: Option<String> = None;
    let mut arg: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Hugo. Simple flag database");
        ap.refer(&mut is_path).add_option(&["-p", "--path"], StoreTrue, "Database name As a file path");
        ap.refer(&mut name).add_argument("Name", Store, "Database name").required();
        ap.refer(&mut op).add_argument("Operation", Store, "Operation (has/get/set/swap/check/import)").required();
        ap.refer(&mut key).add_argument("Key", StoreOption, "Data Key");
        ap.refer(&mut arg).add_argument("Argument", StoreOption, "The argument of operation");
        ap.parse(args().collect(), &mut sink(), &mut sink()).map_err(|_| AppError::InvalidArgument)?;
    }

    if &*op == "shell" {
        Ok((name, is_path, Shell(args().skip(3).collect())))
    } else if let Some(key) = key {
        let op = match &*op {
            "has" => Has(key),
            "get" => Get(key, arg),
            "set" => Set(key, arg),
            "unset" | "remove" => Remove(key),
            "swap" => Swap(key, arg),
            "check" => Check(key, arg),
            "inc" => Modify(key, arg, false),
            "dec" => Modify(key, arg, true),
            "import" => Import(key),
            _ => return Err(AppError::InvalidArgument)
        };
        Ok((name, is_path, op))
    } else {
        Err(AppError::InvalidArgument)?
    }
}


fn app() -> AppResultU {
    use self::Operation::*;

    let (name, is_path, op) = parse_args()?;
    let path = Path::new(&name);
    let path = if is_path {
        path.to_path_buf()
    } else {
        let mut path = get_app_dir(AppDataType::UserData, &APP_INFO, "db").unwrap();
        path.push(format!("{}.sqlite", name));
        path
    };
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let conn = Connection::open(&path)?;
    create_table(&conn)?;

    conn.execute("BEGIN;", NO_PARAMS)?;

    let ok = match op {
        Get(key, default) => print_value(&get(&conn, &key, default)?),
        Has(key) => has(&conn, &key)?,
        Modify(key, delta, minus) => {
            let modified = &modify(&conn, &key, &delta, minus)?;
            println!("{}", modified);
            true
        },
        Set(key, value) => set(&conn, &key, &value)?,
        Swap(key, value) => print_value(&swap(&conn, &key, &value)?),
        Check(key, value) => check(&conn, &key, &value)?,
        Import(ref source) => import(&conn, source)?,
        Shell(ref args) => shell(&path, args)?,
        Remove(key) => remove(&conn, &key)?,
    };

    conn.execute("COMMIT;", NO_PARAMS)?;

    exit(if ok { 0 } else { 1 })
}


fn create_table(conn: &Connection) -> AppResultU {
    conn.execute("CREATE TABLE IF NOT EXISTS flags (key TEXT PRIMARY KEY, value TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, expired_at TEXT);", NO_PARAMS).unwrap();
    Ok(())
}


#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn get(conn: &Connection, key: &str, default: Option<String>) -> AppResult<Option<Option<String>>> {
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

fn has(conn: &Connection, key: &str) -> AppResult<bool> {
    get(conn, key, None).map(|it| it.is_some())
}

fn set(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<bool> {
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

fn modify(conn: &Connection, key: &str, delta: &Option<String>, minus: bool) -> AppResult<f64> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get(conn, key, None)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, key, &Some(format!("{}", modified)))?;

    Ok(modified)
}

#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn swap(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<Option<Option<String>>> {
    let result = get(conn, key, None)?;
    set(conn, key, value)?;
    Ok(result)
}

fn check(conn: &Connection, key: &str, value: &Option<String>) -> AppResult<bool> {
    swap(conn, key, value).map(|it| it.is_some())
}

fn import(conn: &Connection, source_path: &str) -> AppResult<bool> {
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

fn shell(path: &Path, args: &[String]) -> AppResult<bool> {
    Command::new("sqlite3")
        .arg(path)
        .args(args)
        .exec();
    Ok(true)
}

fn remove(conn: &Connection, key: &str) -> AppResult<bool> {
    let n = conn.execute("DELETE FROM flags WHERE key = ?", &[&key])?;
    Ok(n == 1)
}


#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn print_value(found: &Option<Option<String>>) -> bool {
    if let Some(ref found) = *found {
        if let Some(ref value) = *found {
            println!("{}", value);
        }
        true
    } else {
        false
    }
}

fn usage() {
    eprintln!("{}", USAGE);
}

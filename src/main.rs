extern crate app_dirs;
extern crate argparse;
extern crate rusqlite;
extern crate time;

use std::env::args;
use std::error::Error;
use std::fmt;
use std::fs::create_dir_all;
use std::io::sink;
use std::path::Path;
use std::process::exit;

#[cfg(any(unix))] use std::os::unix::process::CommandExt;
#[cfg(any(unix))] use std::process::Command;

use app_dirs::*;
use argparse::{ArgumentParser, StoreTrue, StoreOption, Store};
use rusqlite::Connection;



type ID = String;
const APP_INFO: AppInfo = AppInfo { name: "hugo", author: "anekos" };
pub static USAGE: &'static str = include_str!("usage.txt");

enum Operation {
    Has(ID),
    Get(ID, Option<String>),
    Check(ID, Option<String>),
    Set(ID, Option<String>),
    Swap(ID, Option<String>),
    Modify(ID, Option<String>, bool),
    Import(String),
    Remove(ID),
    Shell(Vec<String>),
}

#[derive(Debug)]
struct Entry {
    id: String,
    content: Option<String>,
    created_at: String,
    updated_at: String,
}


#[derive(Debug)]
enum HugoError {
    InvalidArgument,
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


fn parse_args() -> Result<(String, bool, Operation), Box<Error>> {
    use self::Operation::*;

    let mut is_path = false;
    let mut name = "".to_owned();
    let mut op = "".to_owned();
    let mut id: Option<String> = None;
    let mut arg: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Hugo. Simple flag database");
        ap.refer(&mut is_path).add_option(&["-p", "--path"], StoreTrue, "Database name As a file path");
        ap.refer(&mut name).add_argument("Name", Store, "Database name").required();
        ap.refer(&mut op).add_argument("Operation", Store, "Operation (has/get/set/swap/check/import)").required();
        ap.refer(&mut id).add_argument("ID", StoreOption, "Data ID");
        ap.refer(&mut arg).add_argument("Argument", StoreOption, "The argument of operation");
        ap.parse(args().collect(), &mut sink(), &mut sink()).map_err(|_| HugoError::InvalidArgument)?;
    }

    if &*op == "shell" {
        Ok((name, is_path, Shell(args().collect())))
    } else if let Some(id) = id {
        let op = match &*op {
            "has" => Has(id),
            "get" => Get(id, arg),
            "set" => Set(id, arg),
            "unset" | "remove" => Remove(id),
            "swap" => Swap(id, arg),
            "check" => Check(id, arg),
            "inc" => Modify(id, arg, false),
            "dec" => Modify(id, arg, true),
            "import" => Import(id),
            _ => return Err(Box::new(HugoError::InvalidArgument))
        };
        Ok((name, is_path, op))
    } else {
        Err(HugoError::InvalidArgument)?
    }
}


fn app() -> Result<(), Box<Error>> {
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

    conn.execute("BEGIN;", &[])?;

    let ok = match op {
        Get(id, default) => print_content(&get(&conn, &id, default)?),
        Has(id) => has(&conn, &id)?,
        Modify(id, delta, minus) => {
            let modified = &modify(&conn, &id, &delta, minus)?;
            println!("{}", modified);
            true
        },
        Set(id, content) => set(&conn, &id, &content)?,
        Swap(id, content) => print_content(&swap(&conn, &id, &content)?),
        Check(id, content) => check(&conn, &id, &content)?,
        Import(ref source) => import(&conn, source)?,
        Shell(ref args) => shell(&path, args)?,
        Remove(id) => remove(&conn, &id)?,
    };

    conn.execute("COMMIT;", &[])?;

    exit(if ok { 0 } else { 1 })
}


fn create_table(conn: &Connection) -> Result<(), Box<Error>> {
    conn.execute("CREATE TABLE IF NOT EXISTS flags (id TEXT PRIMARY KEY, content TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);", &[]).unwrap();
    Ok(())
}


#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn get(conn: &Connection, id: &str, default: Option<String>) -> Result<Option<Option<String>>, rusqlite::Error> {
    use rusqlite::Error::QueryReturnedNoRows;

    let result = conn.query_row("SELECT content FROM flags WHERE id = ?;", &[&id], |row| {
        row.get(0)
    });

    if let Err(ref err) = result {
        if let QueryReturnedNoRows = *err {
             return Ok(default.map(Some));
        }
    }

    result.map(Some)
}

fn has(conn: &Connection, id: &str) -> Result<bool, rusqlite::Error> {
    get(conn, id, None).map(|it| it.is_some())
}

fn set(conn: &Connection, id: &str, content: &Option<String>) -> Result<bool, Box<Error>> {
    let now = time::get_time();
    conn.execute("UPDATE flags SET content = ?, updated_at = ? WHERE id = ?", &[content, &now, &id])?;
    conn.execute("INSERT INTO flags SELECT ?, ?, ?, ? WHERE (SELECT changes() = 0)", &[&id, content, &now, &now])?;
    Ok(true)
}

fn modify(conn: &Connection, id: &str, delta: &Option<String>, minus: bool) -> Result<f64, Box<Error>> {
    let delta = delta.as_ref().map(|it| it.parse()).unwrap_or(Ok(1.0))?;

    let found = get(conn, id, None)?;
    let current = found.and_then(|it| it.map(|it| it.parse())).unwrap_or(Ok(0.0))?;
    let modified = current + delta * if minus { -1.0 } else { 1.0 };

    set(conn, id, &Some(format!("{}", modified)))?;

    Ok(modified)
}

#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn swap(conn: &Connection, id: &str, content: &Option<String>) -> Result<Option<Option<String>>, Box<Error>> {
    let result = get(conn, id, None)?;
    set(conn, id, content)?;
    Ok(result)
}

fn check(conn: &Connection, id: &str, content: &Option<String>) -> Result<bool, Box<Error>> {
    swap(conn, id, content).map(|it| it.is_some())
}

fn import(conn: &Connection, source_path: &str) -> Result<bool, Box<Error>> {
    let source_conn = Connection::open(source_path)?;

    let mut stmt = source_conn.prepare("SELECT id, content, created_at, updated_at FROM flags;").unwrap();
    let entry_iter = stmt.query_map(&[], |row| {
        Entry {
            id: row.get(0),
            content: row.get(1),
            created_at: row.get(2),
            updated_at: row.get(3)
        }
    }).unwrap();

    let mut result = true;
    for entry in entry_iter {
        let entry = entry?;
        result &= set(conn, &entry.id, &entry.content)?;
    }

    Ok(result)
}

fn shell(path: &Path, args: &[String]) -> Result<bool, Box<Error>> {
    Command::new("sqlite3")
        .arg(path)
        .args(args)
        .exec();
    Ok(true)
}

fn remove(conn: &Connection, id: &str) -> Result<bool, Box<Error>> {
    let n = conn.execute("DELETE FROM flags WHERE id = ?", &[&id])?;
    Ok(n == 1)
}


impl fmt::Display for HugoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::HugoError::*;

        match *self {
            InvalidArgument => write!(f, "Invalid argument"),
        }
    }
}

impl Error for HugoError {
    fn description(&self) -> &str {
        use self::HugoError::*;

        match *self {
            InvalidArgument => "Invalid argument",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(option_option))]
fn print_content(found: &Option<Option<String>>) -> bool {
    if let Some(ref found) = *found {
        if let Some(ref content) = *found {
            println!("{}", content);
        }
        true
    } else {
        false
    }
}

fn usage() {
    eprintln!("{}", USAGE);
}

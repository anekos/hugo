use std::env::args;
use std::fs::create_dir_all;
use std::io::sink;
use std::path::Path;
use std::process::exit;

use app_dirs::*;
use argparse::{ArgumentParser, StoreTrue, StoreOption, Store};
use rusqlite::{Connection, NO_PARAMS};

mod command;
mod errors;
mod types;

use errors::{AppError, AppResult, AppResultU};
use types::*;



const APP_INFO: AppInfo = AppInfo { name: "hugo", author: "anekos" };



fn main() {
    match app() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}\n", e);
            command::usage();
            exit(2)
        },
    }
}


fn parse_args() -> AppResult<(String, bool, Operation)> {
    use Operation::*;

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
        Get(key, default) => command::print_value(&command::get(&conn, &key, default)?),
        Has(key) => command::has(&conn, &key)?,
        Modify(key, delta, minus) => {
            let modified = &command::modify(&conn, &key, &delta, minus)?;
            println!("{}", modified);
            true
        },
        Set(key, value) => command::set(&conn, &key, &value)?,
        Swap(key, value) => command::print_value(&command::swap(&conn, &key, &value)?),
        Check(key, value) => command::check(&conn, &key, &value)?,
        Import(ref source) => command::import(&conn, source)?,
        Shell(ref args) => command::shell(&path, args)?,
        Remove(key) => command::remove(&conn, &key)?,
    };

    conn.execute("COMMIT;", NO_PARAMS)?;

    exit(if ok { 0 } else { 1 })
}


fn create_table(conn: &Connection) -> AppResultU {
    conn.execute("CREATE TABLE IF NOT EXISTS flags (key TEXT PRIMARY KEY, value TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, expired_at TEXT);", NO_PARAMS).unwrap();
    Ok(())
}

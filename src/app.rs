
use std::fs::create_dir_all;
use std::path::PathBuf;

use app_dirs::*;
use clap::ArgMatches;
use rusqlite::{Connection, NO_PARAMS};

use crate::command::Command;
use crate::errors::{AppResult, AppResultU};



const APP_INFO: AppInfo = AppInfo { name: "hugo", author: "anekos" };


pub fn run(matches: &ArgMatches) -> AppResult<bool> {
    use crate::command::impls::*;

    let database_name = matches.value_of("database-name").unwrap(); // required

    let (path, conn) = initialize(database_name)?;

    let result = if let Some(matches) = matches.subcommand_matches("check") {
        Check::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("dec") {
        Decrement::new(matches).run(&conn, &path)?
    } else if matches.subcommand_matches("gc").is_some() {
        Gc::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("get") {
        Get::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("has") {
        Has::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("import") {
        Import::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("inc") {
        Increment::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("set") {
        Set::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("shell") {
        Shell::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("swap") {
        Swap::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("ttl") {
        Ttl::new(matches).run(&conn, &path)?
    } else if let Some(matches) = matches.subcommand_matches("unset") {
        Remove::new(matches).run(&conn, &path)?
    } else {
        Unknown::new(matches).run(&conn, &path)?
    };

    conn.execute("COMMIT", NO_PARAMS)?;

    if matches.subcommand_matches("gc").is_some() {
        conn.execute("VACUUM", NO_PARAMS)?;
    }

    Ok(result)
}

fn initialize(database_name: &str) -> AppResult<(PathBuf, Connection)> {
    let mut path = get_app_dir(AppDataType::UserData, &APP_INFO, "db").unwrap();
    path.push(format!("{}.sqlite", database_name));
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let conn = Connection::open(&path)?;
    create_table(&conn)?;
    conn.execute("BEGIN;", NO_PARAMS)?;
    Ok((path, conn))
}

fn create_table(conn: &Connection) -> AppResultU {
    conn.execute("CREATE TABLE IF NOT EXISTS h (key TEXT PRIMARY KEY, value TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, expired_at INTEGER);", NO_PARAMS).unwrap();
    Ok(())
}

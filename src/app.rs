
use std::fs::create_dir_all;
use std::path::PathBuf;

use app_dirs::*;
use clap::ArgMatches;
use rusqlite::{Connection, NO_PARAMS};

use crate::command;
use crate::errors::{AppError, AppResult, AppResultU};



const APP_INFO: AppInfo = AppInfo { name: "hugo", author: "anekos" };


pub fn run(matches: &ArgMatches) -> AppResult<bool> {
    let database_name = matches.value_of("database-name").unwrap(); // required

    let (path, conn) = initialize(&database_name)?;

    let result = if let Some(ref matches) = matches.subcommand_matches("has") {
        let key = matches.value_of("key").unwrap(); // required
        command::has(&conn, key)?
    } else if let Some(ref matches) = matches.subcommand_matches("get") {
        let key = matches.value_of("key").unwrap(); // required
        let default: Option<&str> = matches.value_of("default");
        command::get(&conn, key, default)?
    } else if let Some(ref matches) = matches.subcommand_matches("set") {
        let key = matches.value_of("key").unwrap(); // required
        let value = matches.value_of("value");
        let ttl = matches.value_of("ttl");
        command::set(&conn, key, value, ttl)?
    } else if let Some(ref matches) = matches.subcommand_matches("inc") {
        let key = matches.value_of("key").unwrap(); // required
        let value = matches.value_of("value");
        let ttl = matches.value_of("ttl");
        command::modify(&conn, key, value, false, ttl)?
    } else if let Some(ref matches) = matches.subcommand_matches("dec") {
        let key = matches.value_of("key").unwrap(); // required
        let value = matches.value_of("value");
        let ttl = matches.value_of("ttl");
        command::modify(&conn, key, value, true, ttl)?
    } else if let Some(ref matches) = matches.subcommand_matches("unset") {
        let key = matches.value_of("key").unwrap(); // required
        command::remove(&conn, key)?
    } else if let Some(ref matches) = matches.subcommand_matches("check") {
        let key = matches.value_of("key").unwrap(); // required
        let value = matches.value_of("value");
        let ttl = matches.value_of("ttl");
        command::check(&conn, key, value, ttl)?
    } else if let Some(ref matches) = matches.subcommand_matches("swap") {
        let key = matches.value_of("key").unwrap(); // required
        let value = matches.value_of("value");
        let ttl = matches.value_of("ttl");
        command::swap(&conn, key, value, ttl)?
    } else if let Some(ref matches) = matches.subcommand_matches("import") {
        let filepath = matches.value_of("file-path").unwrap(); // required
        command::import(&conn, filepath)?
    } else if let Some(ref matches) = matches.subcommand_matches("shell") {
        let command: Option<Vec<&str>> = matches.values_of("command").map(|it| it.collect());
        command::shell(&path, command.as_ref().map(|it| it.as_slice()))?
    } else if let Some(ref matches) = matches.subcommand_matches("ttl") {
        let key = matches.value_of("key").unwrap(); // required
        let ttl = matches.value_of("ttl");
        command::ttl(&conn, key, ttl)?
    } else if matches.subcommand_matches("gc").is_some() {
        command::gc(&conn)?
    } else {
        return Err(AppError::UnknownCommand);
    };

    conn.execute("COMMIT", NO_PARAMS)?;

    if matches.subcommand_matches("gc").is_some() {
        command::vacuum(&conn)?;
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
    conn.execute("CREATE TABLE IF NOT EXISTS flags (key TEXT PRIMARY KEY, value TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, expired_at INTEGER);", NO_PARAMS).unwrap();
    Ok(())
}

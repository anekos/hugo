
use std::iter::Iterator;

#[cfg(any(unix))] use std::os::unix::process::CommandExt;

use chrono::offset::{Local, Utc};
use chrono::{DateTime};
use clap::ArgMatches;
use if_let_return::if_let_some;
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};

use super::*;
use super::common::*;
use crate::errors::{AppError, AppResult};



defcmd!(Check, KeyArg, ValueArg, TtlArg => (self, conn) {
    swap_values(conn, self.key(), self.value(), self.ttl()).map(|it| it.is_some())
});

defcmd!(Get, KeyArg, DefaultValueArg => (self, conn) {
    Ok(p(&get_value_with_default(conn, self.key(), self.default())?))
});

defcmd!(Gc => (self, conn) {
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
});

defcmd!(Has, KeyArg => (self, conn) {
    get_value(conn, self.key()).map(|it| it.is_some())
});

defcmd!(Increment, KeyArg, ValueArg, TtlArg => (self, conn) {
    let result = modify_value(conn, self.key(), self.value(), false, self.ttl())?;
    println!("{}", result);
    Ok(true)
});

defcmd!(Decrement, KeyArg, ValueArg, TtlArg => (self, conn) {
    let result = modify_value(conn, self.key(), self.value(), true, self.ttl())?;
    println!("{}", result);
    Ok(true)
});

defcmd!(Import, FilepathArg => (self, conn) {
    let source_conn = Connection::open(self.filepath())?;

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
});

defcmd!(Remove, KeyArg => (self, conn) {
    let n = conn.execute("DELETE FROM h WHERE key = ?", &[self.key()])?;
    Ok(n == 1)
});

defcmd!(Set, KeyArg, TtlArg, ValueArg => (self, conn) {
    let expired_at = if let Some(ttl) = self.ttl() {
        Some(parse_ttl(ttl)?)
    } else {
        None
    };
    set_value(conn, self.key(), self.value(), expired_at)
});

defcmd!(Shell, KeyArg => (self, _conn, path) {
    let args: Option<Vec<&str>> = self.matches().values_of("command").map(Iterator::collect);
    let mut command = std::process::Command::new("sqlite3");
    command.arg(path.as_ref());
    if let Some(args) = args {
        command.args(args);
    }
    command.exec();
    Ok(true)
});


defcmd!(Swap, KeyArg, ValueArg, TtlArg => (self, conn) {
    Ok(p(&swap_values(conn, self.key(), self.value(), self.ttl())?))
});

defcmd!(Ttl, KeyArg, ValueArg, TtlArg => (self, conn) {
    if_let_some!((_, expired_at) = get_value_and_ttl(conn, self.key())?, Ok(false));

    if let Some(ttl) = self.ttl() {
        let expired_at = parse_ttl(ttl)?;
        let updated = conn.execute(
            "UPDATE h SET expired_at = ? WHERE key = ?",
            &[&expired_at as &ToSql, &self.key()]
        )?;
        if updated != 1 {
            panic!("WTF!");
        }
    } else if let Some(expired_at) = expired_at {
        let expired_at = expired_at.with_timezone(&Local);
        println!("{}", expired_at.format("%Y-%m-%d %H:%M:%S"));
    }

    Ok(true)
});

defcmd!(Unknown => (self, _conn) {
    Err(AppError::UnknownCommand)
});

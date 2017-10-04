extern crate app_dirs;
extern crate rusqlite;
extern crate time;

use std::env::args;
use std::error::Error;
use std::fmt;
use std::fs::create_dir_all;
use std::path::Path;
use std::process::exit;

use app_dirs::*;
use rusqlite::Connection;



type ID = String;
const APP_INFO: AppInfo = AppInfo { name: "chrysoberyl", author: "anekos" };
pub static USAGE: &'static str = include_str!("usage.txt");

enum Operation {
    Has(ID),
    Get(ID),
    Check(ID, Option<String>),
    Set(ID, Option<String>),
    Swap(ID, Option<String>),
}


#[derive(Debug)]
enum HugoError {
    TooFewArguments,
    TooManyArguments,
    Unknown(String),
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


fn parse_args() -> Result<(String, Operation), Box<Error>> {
    use self::Operation::*;
    use self::HugoError::TooFewArguments;

    let mut args = args();
    let _ = args.next();
    let file = args.next().ok_or(TooFewArguments)?;
    let op = args.next().ok_or(TooFewArguments)?;
    let id = args.next().ok_or(TooFewArguments)?;
    let id: ID = id.parse()?;

    let op = match &*op {
        "has" => Has(id),
        "get" => Get(id),
        "set" => Set(id, args.next()),
        "swap" => Swap(id, args.next()),
        "check" => Check(id, args.next()),
        unknown => return Err(Box::new(HugoError::Unknown(unknown.to_owned())))
    };

    if args.next().is_some() {
        return Err(Box::new(HugoError::TooManyArguments))
    }

    Ok((file, op))
}


fn app() -> Result<(), Box<Error>> {
    use self::Operation::*;

    let (name, op) = parse_args()?;
    let path = Path::new(&name);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let mut path = get_app_dir(AppDataType::UserCache, &APP_INFO, "db").unwrap();
        path.push(&name);
        path
    };
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    create_table(&conn)?;

    let ok = match op {
        Get(id) => print_content(&get(&conn, &id)?),
        Has(id) => has(&conn, &id)?,
        Set(id, content) => set(&conn, &id, &content)?,
        Swap(id, content) => print_content(&swap(&conn, &id, &content)?),
        Check(id, content) => check(&conn, &id, &content)?,
    };

    exit(if ok { 0 } else { 1 })
}


fn create_table(conn: &Connection) -> Result<(), Box<Error>> {
    conn.execute("CREATE TABLE IF NOT EXISTS flags (id TEXT PRIMARY KEY, content TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);", &[]).unwrap();
    Ok(())
}


fn get(conn: &Connection, id: &str) -> Result<Option<Option<String>>, rusqlite::Error> {
    use rusqlite::Error::QueryReturnedNoRows;

    let result = conn.query_row("SELECT content FROM flags WHERE id = ?;", &[&id], |row| {
        row.get(0)
    });

    if let Err(ref err) = result {
        if let QueryReturnedNoRows =  *err {
             return Ok(None);
        }
    }

    result.map(Some)
}

fn has(conn: &Connection, id: &str) -> Result<bool, rusqlite::Error> {
    get(conn, id).map(|it| it.is_some())
}

fn set(conn: &Connection, id: &str, content: &Option<String>) -> Result<bool, Box<Error>> {
    let now = time::get_time();
    conn.execute("UPDATE flags SET content = ?, updated_at = ? WHERE id = ?", &[content, &now, &id])?;
    conn.execute("INSERT INTO flags SELECT ?, ?, ?, ? WHERE (SELECT changes() = 0)", &[&id, content, &now, &now])?;
    Ok(true)
}

fn swap(conn: &Connection, id: &str, content: &Option<String>) -> Result<Option<Option<String>>, Box<Error>> {
    let result = get(conn, id)?;
    set(conn, id, content)?;
    Ok(result)
}

fn check(conn: &Connection, id: &str, content: &Option<String>) -> Result<bool, Box<Error>> {
    swap(conn, id, content).map(|it| it.is_some())
}


impl fmt::Display for HugoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::HugoError::*;

        match *self {
            TooFewArguments => write!(f, "Too few arguments"),
            TooManyArguments => write!(f, "Too many arguments"),
            Unknown(ref content) => write!(f, "Unknown operation: {}", content),
        }
    }
}

impl Error for HugoError {
    fn description(&self) -> &str {
        use self::HugoError::*;

        match *self {
            TooFewArguments => "Too few arguments",
            TooManyArguments => "Too many arguments",
            Unknown(_) => "Unknown operation",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

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

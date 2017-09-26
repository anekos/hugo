extern crate rusqlite;
extern crate time;

use std::env::args;
use std::fmt;
use std::error::Error;
use std::process::exit;

use rusqlite::Connection;



enum Operation {
    Has(i64),
    Get(i64),
    Set(i64, Option<String>),
}


#[derive(Debug)]
enum HugoError {
    NotEnoughArguments,
    Unknown(String),
}



fn main() {
    match app() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            usage();
            exit(2)
        },
    }
}


fn parse_args() -> Result<(String, Operation), Box<Error>> {
    use self::Operation::*;
    use self::HugoError::NotEnoughArguments;

    let mut args = args();
    let _ = args.next();
    let file = args.next().ok_or(NotEnoughArguments)?;
    let op = args.next().ok_or(NotEnoughArguments)?;
    let id = args.next().ok_or(NotEnoughArguments)?;
    let id: i64 = id.parse()?;

    let op = match &*op {
        "has" => Has(id),
        "get" => Get(id),
        "set" => Set(id, args.next()),
        unknown => return Err(Box::new(HugoError::Unknown(unknown.to_owned())))
    };

    Ok((file, op))
}


fn app() -> Result<(), Box<Error>> {
    use self::Operation::*;

    let (file, op) = parse_args()?;
    let conn = Connection::open(file)?;
    create_table(&conn)?;

    match op {
        Get(id) => exit({
            if let Some(content) = get(&conn, id)? {
                println!("{}", content);
                0
            } else {
                1

            }
        }),
        Has(id) => exit(if has(&conn, id)? { 0 } else { 1 }),
        Set(id, content) => set(&conn, id, &content),
    }
}


fn create_table(conn: &Connection) -> Result<(), Box<Error>> {
    conn.execute("CREATE TABLE IF NOT EXISTS flags (id INT8 PRIMARY KEY, content TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);", &[]).unwrap();
    Ok(())
}


fn get(conn: &Connection, id: i64) -> Result<Option<String>, rusqlite::Error> {
    use rusqlite::Error::QueryReturnedNoRows;

    let result = conn.query_row("SELECT content FROM flags WHERE id = ?;", &[&id], |row| {
        row.get(0)
    });

    if let Err(ref err) = result {
        if let QueryReturnedNoRows =  *err {
             return Ok(None);
        }
    }

    result
}

fn has(conn: &Connection, id: i64) -> Result<bool, rusqlite::Error> {
    get(conn, id).map(|it| it.is_some())
}

fn set(conn: &Connection, id: i64, content: &Option<String>) -> Result<(), Box<Error>> {
    let now = time::get_time();
    conn.execute("UPDATE flags SET content = ?, updated_at = ? WHERE id = ?", &[content, &now, &id])?;
    conn.execute("INSERT INTO flags SELECT ?, ?, ?, ? WHERE (SELECT changes() = 0)", &[&id, content, &now, &now])?;
    Ok(())
}



impl fmt::Display for HugoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::HugoError::*;

        match *self {
            NotEnoughArguments => write!(f, "Not enough arguments"),
            Unknown(ref content) => write!(f, "Unknown operation: {}", content),
        }
    }
}

impl Error for HugoError {
    fn description(&self) -> &str {
        use self::HugoError::*;

        match *self {
            NotEnoughArguments => "Not enough arguments",
            Unknown(_) => "Unknown operation",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

fn usage() {
    eprintln!("Usage: hugo has <FLAG_FILE> <ID>");
    eprintln!("       hugo get <FLAG_FILE> <ID>");
    eprintln!("       hugo set <FLAG_FILE> <ID> [<TEXT>]");
}

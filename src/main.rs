
use std::process::exit;

mod app;
mod args;
mod command;
mod errors;
mod types;



fn main() {
    let matches = crate::args::build_cli().get_matches();

    match app::run(&matches) {
        Ok(succeed) => exit(if succeed { 0 } else { 1 }),
        Err(e) => {
            eprintln!("{}\n", e);
            command::usage();
            exit(2)
        },
    }
}

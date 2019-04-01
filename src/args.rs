
use clap::*;


const TTL_HELP: &str = "TTL: `12 years 15days 2min 2s` / `2018-01-01T12:53:00Z` / `2018-01-01 12:53:00`";


pub fn build_cli() -> App<'static, 'static> {
    app_from_crate!()
        .arg(Arg::with_name("database-name")
             .help("Database name")
             .required(true))
        .subcommand(SubCommand::with_name("has")
                    .alias("h")
                    .about("This command succeeds, if the key is found")
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true)))
        .subcommand(SubCommand::with_name("get")
                    .alias("g")
                    .about("Get the value")
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("default")
                         .help("Print this value if key not found")
                         .required(false)))
        .subcommand(SubCommand::with_name("set")
                    .alias("s")
                    .about("Set the value")
                    .arg(Arg::with_name("ttl")
                         .short("t")
                         .long("ttl")
                         .help(TTL_HELP)
                         .takes_value(true))
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("value")
                         .help("Value")
                         .required(false)))
        .subcommand(SubCommand::with_name("inc")
                    .alias("i")
                    .about("Increment the value")
                    .arg(Arg::with_name("ttl")
                         .short("t")
                         .long("ttl")
                         .help(TTL_HELP)
                         .takes_value(true))
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("value")
                         .help("Value")
                         .required(false)))
        .subcommand(SubCommand::with_name("dec")
                    .alias("i")
                    .about("Increment the value")
                    .arg(Arg::with_name("ttl")
                         .short("t")
                         .long("ttl")
                         .help(TTL_HELP)
                         .takes_value(true))
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("value")
                         .help("Value")
                         .required(false)))
        .subcommand(SubCommand::with_name("unset")
                    .alias("i")
                    .alias("remove")
                    .about("Unset")
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true)))
        .subcommand(SubCommand::with_name("check")
                    .alias("c")
                    .about("`has` and `set`")
                    .arg(Arg::with_name("ttl")
                         .short("t")
                         .long("ttl")
                         .help(TTL_HELP)
                         .takes_value(true))
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("value")
                         .help("Value")
                         .required(false)))
        .subcommand(SubCommand::with_name("swap")
                    .alias("c")
                    .about("`has` and `set`")
                    .arg(Arg::with_name("ttl")
                         .short("t")
                         .long("ttl")
                         .help(TTL_HELP)
                         .takes_value(true))
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("value")
                         .help("Value")
                         .required(false)))
        .subcommand(SubCommand::with_name("import")
                    .about("Import from *.sqlite")
                    .arg(Arg::with_name("filepath")
                         .help("*.sqlite file which hugo created")
                         .required(true)))
        .subcommand(SubCommand::with_name("shell")
                    .about("Open SQLite shell or execute SQLite command")
                    .arg(Arg::with_name("command")
                         .help("SQLite command")
                         .min_values(0)))
        .subcommand(SubCommand::with_name("ttl")
                    .alias("s")
                    .about("Set the value")
                    .arg(Arg::with_name("key")
                         .help("Key name")
                         .required(true))
                    .arg(Arg::with_name("ttl")
                         .help(TTL_HELP)
                         .required(false)))

}

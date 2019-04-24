
use std::path::Path;

use clap::ArgMatches;
use rusqlite::Connection;

use crate::errors::AppResult;
use crate::types::*;

mod common;



pub trait Command {
    fn matches(&self) -> &ArgMatches;
    fn run<T: AsRef<Path>>(&self, connection: &Connection, path: &T) -> AppResult<bool>;
}

mod arg {
    use super::Command;

    pub trait ShellCommand: Command {
        fn shell_command(&self) -> Option<Vec<&str>> {
            self.matches().values_of("command").map(Iterator::collect)
        }
    }

    pub trait Filepath: Command {
        fn filepath(&self) -> &str {
            self.matches().value_of("filepath").unwrap()
        }
    }

    pub trait Ttl: Command {
        fn ttl(&self) -> Option<&str> {
            self.matches().value_of("ttl")
        }
    }

    pub trait Key: Command {
        fn key(&self) -> &str {
            self.matches().value_of("key").unwrap()
        }
    }

    pub trait Value: Command {
        fn value(&self) -> Option<&str> {
            self.matches().value_of("value")
        }
    }

    pub trait DefaultValue: Command {
        fn default(&self) -> Option<&str> {
            self.matches().value_of("default")
        }
    }
}


#[macro_export]
macro_rules! defcmd {
    ($name:ident $(, $arg:ident)* => ($self:ident, $connection:ident, $path:ident) $body:expr) => {
        pub struct $name<'a>{ _matches: &'a ArgMatches<'a> }

        impl<'a> $name<'a> {
            pub fn new(matches: &'a ArgMatches) -> Self {
                Self { _matches: matches }
            }
        }

        impl<'a> Command for $name<'a> {
            fn matches(&self) -> &ArgMatches {
                self._matches
            }

            fn run<T: AsRef<std::path::Path>>(&$self, $connection: &Connection, $path: &T) -> AppResult<bool> {
                #[allow(unused_imports)]
                use crate::command::arg::*;
                $body
            }
        }

        $(impl<'a> arg::$arg for $name<'a> {})*
    };

    ($name:ident $(, $arg:ident)* => ($self:ident, $connection:ident) $body:expr) => {
        defcmd!($name $(, $arg)* => ($self, $connection, _no) $body);
    };
}


pub mod impls;

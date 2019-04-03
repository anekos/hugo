
use failure::Fail;



pub type AppResult<T> = Result<T, AppError>;
pub type AppResultU = Result<(), AppError>;



#[derive(Fail, Debug)]
pub enum AppError {
    #[fail(display = "IO Error: {}", 0)]
    Io(std::io::Error),
    #[fail(display = "Number format Error: {}", 0)]
    NumberFormat(std::num::ParseFloatError),
    #[fail(display = "SQL Error: {}", 0)]
    Sql(rusqlite::Error),
    #[fail(display = "Time Calculation Error: {}", 0)]
    Time(std::time::SystemTimeError),
    #[fail(display = "TTL Format Error: {}", 0)]
    TtlFormatDateTime(chrono::format::ParseError),
    #[fail(display = "TTL Format Error: {}", 0)]
    TtlFormatDuration(humantime::DurationError),
    #[fail(display = "Unknown command")]
    UnknownCommand,
}


macro_rules! define_error {
    ($source:ty, $kind:ident) => {
        impl From<$source> for AppError {
            fn from(error: $source) -> AppError {
                AppError::$kind(error)
            }
        }
    }
}

define_error!(rusqlite::Error, Sql);
define_error!(std::io::Error, Io);
define_error!(std::num::ParseFloatError, NumberFormat);
define_error!(chrono::format::ParseError, TtlFormatDateTime);
define_error!(humantime::DurationError, TtlFormatDuration);
define_error!(std::time::SystemTimeError, Time);

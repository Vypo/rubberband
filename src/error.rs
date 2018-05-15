use std::fmt;
use std::error::Error as StdError;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    TooBig,
    Full,
    System(Box<StdError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::TooBig => "requested reservation is too big to fit",
            Error::Full => "not enough room available",
            Error::System(ref x) => x.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match self {
            Error::System(ref x) => Some(Box::as_ref(x)),
            _ => None,
        }
    }
}

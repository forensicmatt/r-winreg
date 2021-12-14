use winstructs::security::{SecDescError};
use winstructs::err::Error;
use std::string::FromUtf8Error;
use std::io;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub enum ErrorKind {
    IoError,
    AsciiError,
    Utf16Error,
    FromUtf8Error,
    ValidationError,
    SecDescParseError,
    UnknownAceType
}

#[derive(Debug)]
pub struct RegError {
    pub message: String,
    pub kind: ErrorKind,
    pub trace: String
}

impl RegError {
    #[allow(dead_code)]
    pub fn utf16_decode_error(err: String)->Self{
        RegError {
            message: format!("{}",err),
            kind: ErrorKind::Utf16Error,
            trace: backtrace!()
        }
    }

    #[allow(dead_code)]
    pub fn ascii_decode_error(err: String)->Self{
        RegError {
            message: format!("{}",err),
            kind: ErrorKind::AsciiError,
            trace: backtrace!()
        }
    }

    #[allow(dead_code)]
    pub fn validation_error(err: String)->Self{
        RegError {
            message: format!("{}",err),
            kind: ErrorKind::ValidationError,
            trace: backtrace!()
        }
    }
}
impl From<FromUtf8Error> for RegError {
    fn from(err: FromUtf8Error) -> Self {
        RegError {
            message: format!("{}",err),
            kind: ErrorKind::FromUtf8Error,
            trace: backtrace!()
        }
    }
}
impl From<io::Error> for RegError {
    fn from(err: io::Error) -> Self {
        RegError {
            message: format!("{}",err),
            kind: ErrorKind::IoError,
            trace: backtrace!()
        }
    }
}
impl From<SecDescError> for RegError {
    fn from(err: SecDescError) -> Self {
        RegError {
            message: format!("{:?}",err),
            kind: ErrorKind::SecDescParseError,
            trace: backtrace!()
        }
    }
}
impl From<Error> for RegError {
    fn from(err: Error) -> Self {
        match err {
            Error::IoError{source} => RegError::from(source),
            Error::UnknownAceType{ace_type} => Self {
                message: format!("Unknown AceType: {}", ace_type),
                kind: ErrorKind::UnknownAceType,
                trace: backtrace!()
            }
        }
    }
}
impl Display for RegError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::ValidationError => {
                write!(
                    f,
                    "{:?}: {}",
                    self.kind,self.message
                )
            },
            _ => {
                write!(
                    f,
                    "message: {}\nkind: {:?}\n{}",
                    self.message, self.kind, self.trace
                )
            }
        }
    }
}

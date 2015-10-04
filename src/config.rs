use std::convert::From;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;

use rustc_serialize::json;
use rustc_serialize::Decodable;
use rustc_serialize::json::{Json, Decoder};

#[derive(Debug, RustcDecodable)]
pub struct Config {
    pub download_dir: String,
    pub rpc_enabled: bool,
    pub rpc_bind_address: String,
    pub rpc_port: u32,
    pub rpc_authentication_required: bool,
    pub rpc_url: String,
    pub rpc_username: String,
    pub rpc_plain_password: Option<String>,
}

#[derive(Debug)]
pub enum ConfigReadingError {
    IoError(io::Error),
    ParseError(String),
    ValidationError(String),
}
use self::ConfigReadingError::*;

pub type Result<T> = ::std::result::Result<T, ConfigReadingError>;

pub fn read_config(path: &str) -> Result<Config> {
    let mut file = try!(File::open(path));

    let mut json = try!(Json::from_reader(&mut file));
    try!(preprocess_config(&mut json));

    let mut decoder = Decoder::new(json);
    let config: Config = try!(Decodable::decode(&mut decoder));
    try!(validate_config(&config));

    Ok(config)
}

fn preprocess_config(json: &mut Json) -> Result<()> {
    let mut obj = try!(json.as_object_mut().ok_or(
        ParseError(s!("JSON root element in not an object"))));

    for key in obj.keys().cloned().collect::<Vec<_>>() {
        if key.find("-").is_some() {
            let value = obj.remove(&key).unwrap();
            obj.insert(key.replace("-", "_"), value);
        }
    }

    Ok(())
}

fn validate_config(config: &Config) -> Result<()> {
    fn error(error: &str) -> Result<()> {
        Err(ValidationError(s!(error)))
    }

    if !config.download_dir.starts_with("/") {
        return error("Invalid 'download-dir' value: it must be an absolute path")
    }

    if !config.rpc_enabled {
        return error("RPC is disabled in config")
    }

    if config.rpc_bind_address.trim().is_empty() {
        return error("Invalid 'rpc-bind-address' value: it mustn't be empty")
    }

    if config.rpc_authentication_required && config.rpc_plain_password.is_none() {
        return error("'rpc-plain-password' is a required option when authentication is enabled")
    }

    Ok(())
}

impl fmt::Display for ConfigReadingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IoError(ref err) => write!(f, "{}", err),
            ParseError(ref err) | ValidationError(ref err) => write!(f, "{}", err),
        }
    }
}

impl Error for ConfigReadingError {
    fn description(&self) -> &str {
        "config reading error"
    }
}

impl From<json::ParserError> for ConfigReadingError {
    fn from(err: json::ParserError) -> ConfigReadingError {
        use rustc_serialize::json::ParserError;

        match err {
            ParserError::IoError(err) => IoError(err),
            ParserError::SyntaxError(code, _, _) => ParseError(s!(json::error_str(code))),
        }
    }
}

impl From<json::DecoderError> for ConfigReadingError {
    fn from(err: json::DecoderError) -> ConfigReadingError {
        use rustc_serialize::json::DecoderError;

        match err {
            DecoderError::ParseError(err) => From::from(err),
            DecoderError::ApplicationError(err) => ParseError(err),
            DecoderError::MissingFieldError(field) => ParseError(format!("'{}' option is missing", field)),
            _ => ParseError(s!("JSON validation error")),
        }
    }
}

impl From<io::Error> for ConfigReadingError {
    fn from(err: io::Error) -> ConfigReadingError {
        IoError(err)
    }
}

use std::{fs, path::Path};

use syn::{parse_file, File};

use crate::Error;

pub fn parse_rs<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    let content = fs::read_to_string(path.as_ref()).map_err(Error::IoError)?;
    parse_file(&content).map_err(Error::ParseError)
}

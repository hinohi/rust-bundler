use std::fmt::Debug;
use std::{
    fmt::{self, Display},
    fs, io,
    path::PathBuf,
};

use quote::ToTokens;
use syn::{parse, parse_file};

pub struct Bundler {
    pub target_project_root: PathBuf,
    pub target_bin: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    VagueBin,
    NoBin,
    IoError(io::Error),
    ParseError(parse::Error),
}

impl Bundler {
    pub fn dumps(&self) -> Result<String, Error> {
        let content = self.find_main()?;
        let ast = parse_file(&content).map_err(Error::ParseError)?;
        let mut ret = String::new();
        for attr in ast.attrs.iter() {
            ret.push_str(&attr.to_token_stream().to_string());
        }
        for item in ast.items.iter() {
            ret.push_str(&item.to_token_stream().to_string());
        }
        Ok(ret)
    }

    fn find_main(&self) -> Result<String, Error> {
        if let Some(ref bin) = self.target_bin {
            let bin = bin.to_string() + ".rs";
            return fs::read_to_string(self.target_project_root.join("src/bin").join(bin))
                .map_err(Error::IoError);
        }
        let a = self.target_project_root.join("src/main.rs");
        if a.is_file() {
            return fs::read_to_string(a).map_err(Error::IoError);
        }
        match fs::read_dir("src/bin") {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Error::VagueBin),
            Err(e) => Err(Error::IoError(e)),
            Ok(dir) => {
                let mut bins = Vec::new();
                for entry in dir {
                    let entry = entry.map_err(Error::IoError)?;
                    let file_type = entry.file_type().map_err(Error::IoError)?;
                    if file_type.is_file() && entry.file_name().to_string_lossy().ends_with(".rs") {
                        bins.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
                if bins.len() != 1 {
                    Err(Error::VagueBin)
                } else {
                    let bin = bins.pop().unwrap();
                    println!("{}", bin);
                    return Ok(bin);
                }
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::VagueBin => {
                f.write_str("rust-bundler could not determine which binary to bundle")
            }
            Error::NoBin => f.write_str("runt-bundler could not find any binary to bundle"),
            Error::IoError(e) => fmt::Debug::fmt(&e, f),
            Error::ParseError(e) => fmt::Debug::fmt(&e, f),
        }
    }
}

impl std::error::Error for Error {}

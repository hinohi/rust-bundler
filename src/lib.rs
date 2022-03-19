use std::{
    fmt::{self, Debug, Display},
    fs, io,
    path::PathBuf,
};

use quote::ToTokens;
use syn::{parse, parse_file, visit_mut::VisitMut, ItemMod};

pub struct Bundler {
    pub target_project_root: PathBuf,
    pub target_bin: Option<String>,
}

struct BundleVisitor {
    current_target_root: PathBuf,
    error: Option<Error>,
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
        let mut file = parse_file(&content).map_err(Error::ParseError)?;

        let mut visitor = BundleVisitor {
            current_target_root: self.target_project_root.join("src"),
            error: None,
        };
        syn::visit_mut::visit_file_mut(&mut visitor, &mut file);
        if let Some(e) = visitor.error {
            return Err(e);
        }

        let mut ret = String::new();
        for attr in file.attrs.iter() {
            ret.push_str(&attr.to_token_stream().to_string());
        }
        for item in file.items.iter() {
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
        match fs::read_dir(self.target_project_root.join("src/bin")) {
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

impl BundleVisitor {
    fn read_sibling_mod(&self, rs: &str) -> io::Result<String> {
        let path = self.current_target_root.join(rs);
        fs::read_to_string(path)
    }
}

impl VisitMut for BundleVisitor {
    fn visit_item_mod_mut(&mut self, i: &mut ItemMod) {
        if i.content.is_none() {
            let name = i.ident.to_string() + ".rs";
            let s = match self.read_sibling_mod(&name) {
                Err(e) => {
                    self.error = Some(Error::IoError(e));
                    return;
                }
                Ok(s) => s,
            };
            let file = match parse_file(&s) {
                Err(e) => {
                    self.error = Some(Error::ParseError(e));
                    return;
                }
                Ok(s) => s,
            };
            i.content = Some((syn::token::Brace::default(), file.items));
        }
        syn::visit_mut::visit_item_mod_mut(self, i);
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

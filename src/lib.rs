use std::{
    fmt::{self, Debug, Display},
    fs, io,
    path::PathBuf,
};

use quote::ToTokens;
use serde::Deserialize;
use syn::{parse, visit_mut::VisitMut, Ident};

mod utils;

pub struct Bundler {
    pub target_project_root: PathBuf,
    pub target_bin: Option<String>,
    pub test: bool,
}

#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

struct BundleVisitor {
    crate_name: String,
    test: bool,
    mod_tree: ModPathTree,
    expand_crate: bool,
    error: Option<Error>,
}

struct ModPathTree {
    root_path: PathBuf,
    mod_stack: Vec<String>,
}

#[derive(Debug)]
pub enum Error {
    NoCrateName,
    VagueBin,
    NoBin,
    ModNotFound,
    IoError(io::Error),
    ParseError(parse::Error),
}

impl Bundler {
    pub fn dumps(&self) -> Result<String, Error> {
        let content = self.find_main()?;
        let crate_name = self.crate_name()?;
        let (main, expand_crate) = self.parse_modify(&content, &crate_name)?;
        let lib = if expand_crate {
            let lib_path = self.target_project_root.join("src/lib.rs");
            let content = fs::read_to_string(lib_path).map_err(Error::IoError)?;
            let (lib, _) = self.parse_modify(&content, &crate_name)?;
            lib.items
        } else {
            Vec::new()
        };

        let mut ret = String::new();
        for attr in main.attrs.iter() {
            ret.push_str(&attr.to_token_stream().to_string());
        }

        if !lib.is_empty() {
            ret.push_str(&format!("mod {} {{", self.crate_name().unwrap()));
            for item in lib.iter() {
                ret.push_str(&item.to_token_stream().to_string());
            }
            ret.push('}');
        }

        for item in main.items.iter() {
            ret.push_str(&item.to_token_stream().to_string());
        }
        Ok(ret)
    }

    fn parse_modify(&self, content: &str, crate_name: &str) -> Result<(syn::File, bool), Error> {
        let mut file = syn::parse_file(content).map_err(Error::ParseError)?;
        let mut visitor = BundleVisitor {
            crate_name: crate_name.to_owned(),
            test: self.test,
            mod_tree: ModPathTree {
                root_path: self.target_project_root.join("src"),
                mod_stack: Vec::new(),
            },
            expand_crate: false,
            error: None,
        };
        visitor.visit_file_mut(&mut file);
        if let Some(e) = visitor.error {
            Err(e)
        } else {
            Ok((file, visitor.expand_crate))
        }
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
                    Ok(bin)
                }
            }
        }
    }

    fn crate_name(&self) -> Result<String, Error> {
        let cargo_path = self.target_project_root.join("Cargo.toml");
        let content = fs::read_to_string(cargo_path).map_err(Error::IoError)?;
        let config: CargoToml = toml::from_str(&content).map_err(|_| Error::NoCrateName)?;
        Ok(config.package.name.replace('-', "_"))
    }
}

impl ModPathTree {
    fn push(&mut self, name: &Ident) {
        self.mod_stack.push(name.to_string());
    }

    fn pop(&mut self) {
        self.mod_stack.pop();
    }

    fn current_path(&self) -> PathBuf {
        let mut path = self.root_path.clone();
        for m in self.mod_stack.iter() {
            path.push(m);
        }
        path
    }

    fn read_sibling_mod(&self, name: &Ident) -> Result<syn::File, Error> {
        let rs = name.to_string() + ".rs";
        let path = self.current_path().join(rs);
        if path.is_file() {
            let file = utils::parse_rs(path)?;
            return Ok(file);
        }

        let path = self.current_path().join(name.to_string());
        if path.is_dir() {
            let file = utils::parse_rs(path.join("mod.rs"))?;
            return Ok(file);
        }
        Err(Error::ModNotFound)
    }
}

impl BundleVisitor {
    fn is_test(&self, attr: &syn::Attribute) -> bool {
        if utils::path_is(&attr.path, "test") {
            true
        } else if utils::path_is(&attr.path, "cfg")
            && attr.tokens.to_token_stream().to_string() == "(test)"
        {
            true
        } else {
            false
        }
    }

    fn is_skip(&self, attrs: &[syn::Attribute]) -> bool {
        if self.test {
            return false;
        }
        for attr in attrs {
            if self.is_test(attr) {
                return true;
            }
        }
        false
    }

    fn filter_items(&self, items: &[syn::Item]) -> Vec<syn::Item> {
        let mut ret = Vec::new();
        for item in items {
            let skip = match item {
                syn::Item::Const(i) => self.is_skip(&i.attrs),
                syn::Item::Enum(i) => self.is_skip(&i.attrs),
                syn::Item::ExternCrate(i) => self.is_skip(&i.attrs),
                syn::Item::Fn(i) => self.is_skip(&i.attrs),
                syn::Item::ForeignMod(i) => self.is_skip(&i.attrs),
                syn::Item::Impl(i) => self.is_skip(&i.attrs),
                syn::Item::Macro(i) => self.is_skip(&i.attrs),
                syn::Item::Macro2(i) => self.is_skip(&i.attrs),
                syn::Item::Mod(i) => self.is_skip(&i.attrs),
                syn::Item::Static(i) => self.is_skip(&i.attrs),
                syn::Item::Struct(i) => self.is_skip(&i.attrs),
                syn::Item::Trait(i) => self.is_skip(&i.attrs),
                syn::Item::TraitAlias(i) => self.is_skip(&i.attrs),
                syn::Item::Type(i) => self.is_skip(&i.attrs),
                syn::Item::Union(i) => self.is_skip(&i.attrs),
                syn::Item::Use(i) => self.is_skip(&i.attrs),
                syn::Item::Verbatim(_) => false,
                _ => false,
            };
            if !skip {
                ret.push(item.clone());
            }
        }
        ret
    }
}

fn strip_doc(attrs: &[syn::Attribute]) -> Vec<syn::Attribute> {
    attrs
        .iter()
        .filter(|a| !utils::path_is(&a.path, "doc"))
        .cloned()
        .collect()
}

impl VisitMut for BundleVisitor {
    fn visit_file_mut(&mut self, i: &mut syn::File) {
        i.items = self.filter_items(&i.items);
        i.attrs = strip_doc(&i.attrs);
        syn::visit_mut::visit_file_mut(self, i);
    }

    fn visit_impl_item_mut(&mut self, i: &mut syn::ImplItem) {
        match i {
            syn::ImplItem::Const(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::ImplItem::Method(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::ImplItem::Type(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::ImplItem::Macro(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::ImplItem::Verbatim(_) => {}
            _ => (),
        }
        syn::visit_mut::visit_impl_item_mut(self, i);
    }

    fn visit_item_mut(&mut self, i: &mut syn::Item) {
        match i {
            syn::Item::Const(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Enum(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::ExternCrate(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Fn(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::ForeignMod(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Impl(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Macro(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Macro2(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Mod(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Static(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Struct(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Trait(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::TraitAlias(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Type(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Union(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::Item::Use(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            _ => (),
        }
        syn::visit_mut::visit_item_mut(self, i);
    }

    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        let brace = syn::token::Brace::default();
        if i.content.is_none() {
            let mut file = match self.mod_tree.read_sibling_mod(&i.ident) {
                Ok(file) => file,
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            };
            self.mod_tree.push(&i.ident);
            self.visit_file_mut(&mut file);
            self.mod_tree.pop();
            i.content = Some((brace, file.items));
        } else {
            i.content = Some((brace, self.filter_items(&i.content.as_ref().unwrap().1)));
            syn::visit_mut::visit_item_mod_mut(self, i);
        }
    }

    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        use syn::{UsePath, UseTree};

        match &i.tree {
            UseTree::Path(path) => {
                if path.ident == self.crate_name {
                    self.expand_crate = true;
                }
                if path.ident == "crate" {
                    // convert `use crate::[something]` -> `use crate::[crate-name]::[something]`
                    let colon2_token = path.colon2_token;
                    let tree = UseTree::Path(UsePath {
                        ident: Ident::new(&self.crate_name, path.ident.span()),
                        colon2_token,
                        tree: path.tree.clone(),
                    });
                    let path = UsePath {
                        ident: path.ident.clone(),
                        colon2_token,
                        tree: Box::new(tree),
                    };
                    i.tree = UseTree::Path(path);
                }
            }
            UseTree::Name(name) => {
                if name.ident == self.crate_name {
                    self.expand_crate = true;
                }
            }
            UseTree::Rename(name) => {
                if name.ident == self.crate_name {
                    self.expand_crate = true;
                }
            }
            _ => (),
        }
    }

    fn visit_trait_item_mut(&mut self, i: &mut syn::TraitItem) {
        match i {
            syn::TraitItem::Const(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::TraitItem::Method(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::TraitItem::Type(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::TraitItem::Macro(i) => {
                i.attrs = strip_doc(&i.attrs);
            }
            syn::TraitItem::Verbatim(_) => (),
            _ => (),
        }
        syn::visit_mut::visit_trait_item_mut(self, i);
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoCrateName => {
                f.write_str("rust-bundler could not find crate name in Cargo.toml")
            }
            Error::VagueBin => {
                f.write_str("rust-bundler could not determine which binary to bundle")
            }
            Error::NoBin => f.write_str("runt-bundler could not find any binary to bundle"),
            Error::ModNotFound => f.write_str("mod not found"),
            Error::IoError(e) => fmt::Debug::fmt(&e, f),
            Error::ParseError(e) => fmt::Debug::fmt(&e, f),
        }
    }
}

impl std::error::Error for Error {}

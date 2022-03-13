use quote::ToTokens;
use syn::{parse, parse_file};

pub fn parse_and_dumps(s: &str) -> parse::Result<String> {
    let ast = parse_file(s)?;
    let mut ret = String::new();
    for attr in ast.attrs.iter() {
        ret.push_str(&attr.to_token_stream().to_string());
    }
    for item in ast.items.iter() {
        ret.push_str(&item.to_token_stream().to_string());
    }
    Ok(ret)
}

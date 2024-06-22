use std::{io::Write, process::Stdio};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use sitter_ast::RuleJSON;

fn format_string(input: String) -> String {
    let mut cmd = std::process::Command::new("rustfmt");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    cmd.arg("-q");
    let mut v = cmd.spawn().unwrap();
    v.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();

    let output = v.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

pub fn test_ast(name: &str, source: &str, expected: TokenStream) -> bool {
    let ast: RuleJSON = serde_json::from_str(source).unwrap();
    let ident = syn::Ident::new(name, Span::call_site());

    let ast = ast.generate(&ident).unwrap();
    let generated = format_string(ast.to_string());
    let target = format_string(expected.to_string());

    generated == target
}

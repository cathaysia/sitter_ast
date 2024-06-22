#![allow(clippy::single_match)]
#![allow(unused_variables)]
#![allow(clippy::single_char_pattern)]
#![allow(unused_imports)]
#![allow(clippy::needless_borrow)]

use clap::Parser;
use proc_macro2::Span;
use quote::quote;
use sitter_ast::GrammarJSON;
use sitter_ast::RuleJSON;

#[derive(clap::Parser)]
struct Args {
    file: String,
    #[clap(short)]
    output: Option<String>,
}

fn main() {
    setup_log();
    let args = Args::parse();
    let contnet = std::fs::read_to_string(args.file).unwrap();

    let ast: GrammarJSON = serde_json::from_str(&contnet).unwrap();
    let mut res = quote! {};

    let snippet = ast.to_toke_stream().unwrap();
    res.extend(snippet);

    // println!("{:#?}", ast);
    if let Some(output) = args.output {
        std::fs::write(output, res.to_string()).unwrap();
    } else {
        println!("{:}", res);
    }
}

fn setup_log() {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().with_thread_names(true).with_file(true).with_line_number(true))
        .init();
}

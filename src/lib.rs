#![allow(clippy::single_match)]
#![allow(unused_variables)]
#![allow(clippy::single_char_pattern)]
#![allow(unused_imports)]
#![allow(clippy::needless_borrow)]

mod parse_grammar;
use std::collections::HashSet;

use log::*;
pub use parse_grammar::*;

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;
use tracing_subscriber::fmt::format;

macro_rules! ident {
    ($id:expr) => {
        syn::Ident::new($id, proc_macro2::Span::call_site())
    };
}
macro_rules! syn_lit {
    ($id:expr) => {
        syn::LitStr::new($id, proc_macro2::Span::call_site())
    };
}

const KW_MAPPING: [(&str, &str); 54] = [
    ("as", "KW_AS"),
    ("break", "KW_BREAK"),
    ("const", "KW_CONST"),
    ("continue", "KW_CONTINUE"),
    ("crate", "KW_CRATE"),
    ("else", "KW_ELSE"),
    ("enum", "KW_ENUM"),
    ("extern", "KW_EXTERN"),
    ("false", "KW_FALSE"),
    ("fn", "KW_FN"),
    ("for", "KW_FOR"),
    ("if", "KW_IF"),
    ("impl", "KW_IMPL"),
    ("in", "KW_IN"),
    ("let", "KW_LET"),
    ("loop", "KW_LOOP"),
    ("match", "KW_MATCH"),
    ("mod", "KW_MOD"),
    ("move", "KW_MOVE"),
    ("mut", "KW_MUT"),
    ("pub", "KW_PUB"),
    ("ref", "KW_REF"),
    ("return", "KW_RETURN"),
    ("self", "KW_SELFVALUE"),
    ("Self", "KW_SELFTYPE"),
    ("static", "KW_STATIC"),
    ("struct", "KW_STRUCT"),
    ("super", "KW_SUPER"),
    ("trait", "KW_TRAIT"),
    ("true", "KW_TRUE"),
    ("type", "KW_TYPE"),
    ("unsafe", "KW_UNSAFE"),
    ("use", "KW_USE"),
    ("where", "KW_WHERE"),
    ("while", "KW_WHILE"),
    ("async", "KW_ASYNC"),
    ("await", "KW_AWAIT"),
    ("dyn", "KW_DYN"),
    ("abstract", "KW_ABSTRACT"),
    ("become", "KW_BECOME"),
    ("box", "KW_BOX"),
    ("do", "KW_DO"),
    ("final", "KW_FINAL"),
    ("macro", "KW_MACRO"),
    ("override", "KW_OVERRIDE"),
    ("priv", "KW_PRIV"),
    ("typeof", "KW_TYPEOF"),
    ("unsized", "KW_UNSIZED"),
    ("virtual", "KW_VIRTUAL"),
    ("yield", "KW_YIELD"),
    ("try", "KW_TRY"),
    ("macro_rules", "KW_MACRO_RULES"),
    ("union", "KW_UNION"),
    ("dyn", "KW_DYN"),
];

pub fn to_ident(value: &str) -> String {
    if let Some(v) = KW_MAPPING.iter().find_map(|(k, v)| if *k == value { Some(v) } else { None }) {
        return v.to_case(Case::UpperCamel);
    }

    value
        .replace(" ", "_")
        .replace("&", "And")
        .replace("|", "Or")
        .replace("!", "Not")
        .replace("=", "Eq")
        .replace("<", "Lt")
        .replace(">", "Gt")
        .replace("+", "Add")
        .replace("-", "Sub")
        .replace("*", "Mul")
        .replace("/", "Div")
        .replace("~", "BitNot")
        .replace("%", "Mod")
        .replace("^", "BitXor")
        .replace("?", "Question")
        .replace(":", "Colon")
        .replace(".", "Dot")
        .replace(",", "Comma")
        .replace(";", "Semicolon")
        .replace("(", "LParen")
        .replace(")", "RParen")
        .replace("[", "LBracket")
        .replace("]", "RBracket")
        .replace("{", "LBrace")
        .replace("}", "RBrace")
        .replace("\\", "Backslash")
        .replace("'", "Quote")
        .replace("\"", "DoubleQuote")
        .replace("#", "Hash")
        .replace("@", "At")
        .replace("$", "Dollar")
        .replace("`", "Backtick")
        .replace(" ", "Space")
        .replace("\t", "Tab")
        .replace("\n", "Newline")
        .replace("\r", "CarriageReturn")
}

impl GrammarJSON {
    pub fn to_toke_stream(&self) -> anyhow::Result<TokenStream> {
        let mut res = TokenStream::new();
        for (name, rule) in &self.rules {
            let ident = syn::Ident::new(&name.to_case(Case::UpperCamel), Span::call_site());
            let kind = syn::LitStr::new(&name.to_case(Case::UpperCamel), Span::call_site());

            let snippet = rule.generate(&ident, &kind).unwrap();
            res.extend(snippet);
        }

        Ok(res)
    }
}

impl RuleJSON {
    fn generate(&self, ident: &Ident, kind: &syn::LitStr) -> anyhow::Result<TokenStream> {
        trace!("generate: {self:?}");

        let mut res = quote! {};

        match self {
            RuleJSON::ALIAS { content, named, value } => {}
            RuleJSON::BLANK => {}
            RuleJSON::STRING { value } => {
                let kind = syn::LitStr::new(&value, Span::call_site());

                res.extend(quote! {
                    #[derive(Debug, Default)]
                    pub struct #ident;

                    impl TSParser for #ident {
                        fn parse(node: tree_sitter::Node<'_>, source: &[u8]) -> anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            if node.kind() != #kind || node.is_error() {
                                return Err(anyhow::anyhow!("Bad Grammar"));
                            };

                            Ok(Self)
                        }
                    }
                })
            }
            RuleJSON::PATTERN { value, flags } => res.extend(quote! {
                #[derive(Debug, Default)]
                pub struct #ident{
                    value: String
                }

                impl TSParser for #ident {
                    fn parse(node: tree_sitter::Node<'_>, source: &[u8]) -> anyhow::Result<Self>
                    where
                        Self: Sized,
                    {
                        if node.kind() != #kind || node.is_error() {
                            return Err(anyhow::anyhow!("Bad Grammar"));
                        };

                        Ok(Self {
                            value: utf8_text(node, source)?.to_string(),
                        })
                    }
                }
            }),
            RuleJSON::SYMBOL { name } => {
                let target_ident =
                    syn::Ident::new(&name.to_case(Case::UpperCamel), Span::call_site());
                res.extend(quote! {
                    pub type #ident = #target_ident;
                })
            }
            RuleJSON::CHOICE { members } => {
                let mut mem = quote! {};
                let mut mid = quote! {};
                for (idx, item) in members.iter().enumerate() {
                    match item {
                        RuleJSON::STRING { value } => {
                            let condition = syn::LitStr::new(value, Span::call_site());
                            let ident = {
                                let value = to_ident(value);
                                syn::Ident::new(&value, Span::call_site())
                            };
                            mem.extend(quote! {
                                #ident,
                            });
                            mid.extend(quote! {
                                #condition => Self::#ident,
                            });
                        }
                        RuleJSON::SYMBOL { name } => {
                            let target_ident =
                                syn::Ident::new(&name.to_case(Case::UpperCamel), Span::call_site());
                            let target_kind = syn::LitStr::new(name, Span::call_site());
                            mem.extend(quote! {
                                #target_ident(#target_ident),
                            });
                            mid.extend(quote! {
                                #target_kind => Self::#target_ident(TSParser::parse(node, source)?),
                            });
                        }
                        RuleJSON::SEQ { members } => {
                            let mut mid = quote! {};
                            let mut childs = vec![];

                            for item in members {
                                match item {
                                    RuleJSON::SYMBOL { name } => {
                                        childs.push(name.clone());
                                        let ident = ident!(&name.to_case(Case::UpperCamel));
                                        mid.extend(quote! {
                                            #ident,
                                        });
                                        let sub = ident!(
                                            &format!("TOKEN_{idx}").to_case(Case::UpperCamel)
                                        );
                                        mem.extend(quote! {
                                            #sub(#mid),
                                        })
                                    }
                                    RuleJSON::CHOICE { members } => {
                                        let ident = ident!(&format!("{ident}_TOKEN_{idx}")
                                            .to_case(Case::UpperCamel));
                                        let kind = syn_lit!("");
                                        mem.extend(quote! {
                                            #ident(#ident),
                                        });
                                        res.extend(item.generate(&ident, &kind)?);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => mid.extend(quote! {
                            "todo" => { todo!() },
                        }),
                    }
                }

                res.extend(quote! {
                    #[derive(Debug)]
                    pub enum #ident {
                        #mem
                    }

                    impl TSParser for #ident {
                        fn parse(node: tree_sitter::Node<'_>, source: &[u8]) -> anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            if node.kind() != #kind || node.is_error() {
                                return Err(anyhow::anyhow!("Bad Grammar"));
                            };

                            let node = node.child(0).unwrap();
                            Ok(match node.kind() {
                                #mid
                                kind => {
                                    return Err(anyhow::anyhow!("Unpsected kind: {kind}"))
                                }
                            })
                        }
                    }

                });
            }
            RuleJSON::FIELD { name, content } => {}
            RuleJSON::SEQ { members } => {
                let mut mem = quote! {};
                let mut mid = quote! {};

                let mut alls = HashSet::new();

                for (idx, item) in members.iter().enumerate() {
                    match item {
                        RuleJSON::SYMBOL { name } => {
                            let member_ty =
                                syn::Ident::new(&name.to_case(Case::UpperCamel), Span::call_site());
                            let member_name = {
                                let mut name = name.clone();
                                if alls.contains(&name) {
                                    name += &format!("_{idx}");
                                } else {
                                    alls.insert(name.clone());
                                }

                                syn::Ident::new(&name.to_case(Case::Snake), Span::call_site())
                            };
                            let kind = syn::LitStr::new(name, Span::call_site());

                            mem.extend(quote! {
                                pub #member_name: #member_ty,
                            });
                            mid.extend(quote! {
                                #kind => {
                                    res.#member_name = TSParser::parse(node, source)?;
                                }
                            });
                        }
                        _ => {}
                    }
                }
                res.extend(quote! {
                    #[derive(Debug, Default)]
                    pub struct #ident {
                        #mem
                    }

                    impl TSParser for #ident {
                        fn parse(node: tree_sitter::Node<'_>, source: &[u8]) -> anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            if node.kind() != #kind || node.is_error() {
                                return Err(anyhow::anyhow!("Bad Grammar"));
                            };

                            let mut res = Self::default();

                            for item in node.children(&mut node.walk()) {
                                match node.kind () {
                                    #mid
                                    _ => {
                                        todo!()
                                    }
                                }
                            }

                            Ok(res)
                        }
                    }

                });
            }
            RuleJSON::REPEAT { content } => {
                let token_ident = syn::Ident::new(
                    &format!("{}_TOKEN", ident).to_case(Case::UpperCamel),
                    Span::call_site(),
                );
                let kind = syn::LitStr::new("", Span::call_site());

                res.extend(content.generate(&token_ident, &kind));

                res.extend(quote! {
                    #[derive(Debug, Default)]
                    pub struct #ident {
                        value: Vec<#token_ident>
                    }

                });
            }
            RuleJSON::REPEAT1 { content } => {
                let token_ident = syn::Ident::new(
                    &format!("{}_TOKEN", ident).to_case(Case::UpperCamel),
                    Span::call_site(),
                );
                let kind = syn::LitStr::new("", Span::call_site());

                res.extend(content.generate(&token_ident, &kind));

                res.extend(quote! {
                    #[derive(Debug, Default)]
                    pub struct #ident {
                        value: Vec<#token_ident>
                    }

                    impl TSParser for #ident {
                        fn parse(node: tree_sitter::Node<'_>, source: &[u8]) -> anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            if node.kind() != #kind || node.is_error() {
                                return Err(anyhow::anyhow!("Bad Grammar"));
                            };

                            let mut res = Self::default();

                            for node in node.children(&mut node.walk()) {
                                res.value.push(TSParser::parse(node, source)?);
                            }

                            Ok(res)
                        }
                    }
                });
            }
            RuleJSON::PREC_DYNAMIC { value, content } => {
                res.extend(content.generate(ident, kind)?);
            }
            RuleJSON::PREC_LEFT { value, content } => {
                res.extend(content.generate(ident, kind)?);
            }
            RuleJSON::PREC_RIGHT { value, content } => {
                res.extend(content.generate(ident, kind)?);
            }
            RuleJSON::PREC { value, content } => {
                res.extend(content.generate(ident, kind)?);
            }
            RuleJSON::TOKEN { content } => {
                res.extend(content.generate(ident, kind)?);
            }
            RuleJSON::IMMEDIATE_TOKEN { content } => {
                res.extend(content.generate(ident, kind)?);
            }
        }

        Ok(res)
    }
}

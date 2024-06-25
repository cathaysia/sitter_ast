#![allow(clippy::single_match)]
#![allow(clippy::single_char_pattern)]
#![allow(unused_imports)]
#![allow(clippy::needless_borrow)]

// mod f;
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
macro_rules! lit_str {
    ($id:expr) => {
        syn::LitStr::new($id, proc_macro2::Span::call_site())
    };
}
impl GrammarJSON {
    pub fn to_toke_stream(&self) -> anyhow::Result<TokenStream> {
        let mut res = quote! {
            use tree_sitter::Node as TSNode;

            pub type ParseResult<T> = anyhow::Result<T>;

            trait TSParser {
                fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self>
                where
                    Self: Sized;
            }

            fn utf8_text<'a>(node: TSNode<'_>, source: &'a [u8]) -> Result<&'a str, std::str::Utf8Error> {
                let start = node.start_byte();
                let end = node.end_byte();

                if end >= start {
                    std::str::from_utf8(&source[start..end])
                } else {
                    std::str::from_utf8(&source[start..])
                }
            }
        };
        for (name, rule) in &self.rules {
            let ident = ident!(&name.to_case(Case::UpperCamel));

            let snippet = rule.generate(&ident).unwrap();
            res.extend(snippet);
        }

        for item in &self.externals {
            if let RuleJSON::SYMBOL { name } = item {
                let ident = ident!(&name.to_case(Case::UpperCamel));
                let kind = lit_str!(name);
                res.extend(quote! {
                    pub struct #ident;

                    impl TSParser for #ident {
                        fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                            if root.kind() != #kind {
                                return Err(anyhow::anyhow!("bad kind"));
                            }
                            Ok(Self)
                        }
                    }
                })
            } else {
                warn!("unhanded case for externals: {item:?}");
            }
        }

        Ok(res)
    }
}

impl RuleJSON {
    pub fn generate(&self, ident: &Ident) -> anyhow::Result<TokenStream> {
        trace!("generate: {ident} - {self:?}");

        let mut res = quote! {};

        match self {
            RuleJSON::ALIAS { content, named, value: _ } => {
                if *named {
                    res.extend(content.generate(ident)?);
                }
            }
            RuleJSON::BLANK => {}
            RuleJSON::STRING { value } => {
                let kind = lit_str!(value);

                res.extend(quote! {
                    #[derive(Debug)]
                    pub struct #ident;

                    impl TSParser for #ident {
                        fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                            if root.kind() != #kind {
                                return Err(anyhow::anyhow!("bad kind"));
                            }
                            Ok(Self)
                        }
                    }
                })
            }
            RuleJSON::PATTERN { value: _, flags: _ } => res.extend(quote! {
                #[derive(Debug)]
                pub struct #ident{
                    value: String
                }

                impl TSParser for #ident {
                    fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                        Ok(Self {
                            value: utf8_text(root, source)?.to_string()
                        })
                    }
                }
            }),
            RuleJSON::SYMBOL { name } => {
                let target_ident = ident!(&name.to_case(Case::UpperCamel));
                res.extend(quote! {
                    pub type #ident = #target_ident;
                })
            }
            RuleJSON::CHOICE { members } => {
                let mut mem = quote! {};
                for (idx, item) in members.iter().enumerate() {
                    match item {
                        RuleJSON::STRING { value: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::UpperCamel));
                            mem.extend(quote! {
                                #field_name,
                            });
                        }
                        RuleJSON::SYMBOL { name } => {
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            let need_box = is_recursive_type(&ident.to_string(), item);
                            if need_box {
                                mem.extend(quote! {
                                    #field_type(Box<#field_type>),
                                });
                            } else {
                                mem.extend(quote! {
                                    #field_type(#field_type),
                                });
                            }
                        }
                        RuleJSON::SEQ { members } => {
                            let is_consis_by_symbol = members.iter().all(|item| {
                                matches!(
                                    item,
                                    RuleJSON::STRING { value: _ } | RuleJSON::SYMBOL { name: _ }
                                )
                            });

                            if is_consis_by_symbol {
                                let mut mid = quote! {};
                                let mut childs = vec![];

                                for item in members {
                                    match item {
                                        RuleJSON::SYMBOL { name } => {
                                            childs.push(name.clone());
                                            let need_box =
                                                is_recursive_type(&ident.to_string(), item);
                                            let ident = ident!(&name.to_case(Case::UpperCamel));
                                            if need_box {
                                                mid.extend(quote! {
                                                    Box<#ident>,
                                                });
                                            } else {
                                                mid.extend(quote! {
                                                    #ident,
                                                });
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                let sub = ident!(
                                    &format!("{ident}_TOKEN_{idx}").to_case(Case::UpperCamel)
                                );
                                mem.extend(quote! {
                                    #sub(#mid),
                                });
                            } else {
                                let field_type = ident!(
                                    &format!("{ident}_TOKEN_{idx}").to_case(Case::UpperCamel)
                                );
                                res.extend(item.generate(&field_type));
                                mem.extend(quote! {
                                    #field_type(#field_type),
                                });
                            }
                        }
                        RuleJSON::ALIAS { content: _, named, value: _ } => {
                            if !named {
                                continue;
                            }
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(item.generate(&field_type)?);

                            mem.extend(quote! {
                                #field_type(#field_type),
                            });
                        }
                        RuleJSON::BLANK => mem.extend(quote! {
                            Blank,
                        }),
                        RuleJSON::CHOICE { members: _ }
                        | RuleJSON::PATTERN { value: _, flags: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(item.generate(&field_type)?);

                            mem.extend(quote! {
                                #field_type(#field_type),
                            });
                        }
                        _ => {
                            warn!("unhandled case for CHOICE: {item:?}");
                        }
                    }
                }

                res.extend(quote! {
                    #[derive(Debug)]
                    pub enum #ident {
                        #mem
                    }

                    impl TSParser for #ident {
                        fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                            todo!()
                        }
                    }

                });
            }
            RuleJSON::FIELD { name, content } => {
                let ident = ident!(&format!("{ident}_{name}").to_case(Case::UpperCamel));
                res.extend(content.generate(&ident)?);
                let field_name = lit_str!(name);

                res.extend(quote! {
                    impl #ident {
                        pub fn field_name() -> &str {
                            #field_name
                        }
                    }
                })
            }
            RuleJSON::SEQ { members } => {
                let mut mem = quote! {};

                let mut alls = HashSet::new();

                for (idx, item) in members.iter().enumerate() {
                    match item {
                        RuleJSON::SYMBOL { name } => {
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            let field_name = {
                                let mut name = name.clone();
                                if alls.contains(&name) {
                                    name += &format!("_{idx}");
                                } else {
                                    alls.insert(name.clone());
                                }

                                ident!(&name.to_case(Case::Snake))
                            };

                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            });
                        }
                        RuleJSON::FIELD { name: _, content } => match content.as_ref() {
                            RuleJSON::SYMBOL { name } => {
                                let field_name = ident!(&name.to_case(Case::Snake));
                                let field_type = ident!(&name.to_case(Case::UpperCamel));
                                mem.extend(quote! {
                                    pub #field_name: #field_type,
                                })
                            }
                            RuleJSON::CHOICE { members: _ } => {
                                let name = format!("{ident}_TOKEN_{idx}");

                                let field_name = ident!(&name.to_case(Case::Snake));
                                let field_type = ident!(&name.to_case(Case::UpperCamel));
                                res.extend(content.generate(&field_type));
                                mem.extend(quote! {
                                    pub #field_name: #field_type,
                                });
                            }
                            _ => {}
                        },
                        RuleJSON::CHOICE { members: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            let field_name = ident!(&name.to_case(Case::Snake));
                            res.extend(item.generate(&field_type));
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            })
                        }
                        RuleJSON::STRING { value: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(item.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            });
                        }
                        RuleJSON::PATTERN { value: _, flags: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(item.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            });
                        }
                        RuleJSON::IMMEDIATE_TOKEN { content } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(content.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            });
                        }
                        RuleJSON::REPEAT1 { content } | RuleJSON::REPEAT { content } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            res.extend(content.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: Vec<#field_type>,
                            });
                        }
                        RuleJSON::SEQ { members: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));

                            res.extend(item.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            })
                        }
                        RuleJSON::TOKEN { content: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));

                            res.extend(item.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            })
                        }
                        RuleJSON::ALIAS { content: _, named: _, value: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));

                            res.extend(item.generate(&field_type)?);
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            })
                        }
                        _ => {
                            warn!("unhandled case for SEQ: {item:?}");
                        }
                    }
                }
                res.extend(quote! {
                    #[derive(Debug)]
                    pub struct #ident {
                        #mem
                    }

                    impl TSParser for #ident {
                        fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                            todo!()
                        }
                    }
                });
            }
            RuleJSON::REPEAT1 { content } | RuleJSON::REPEAT { content } => {
                let field_type = ident!(&format!("{}_TOKEN", ident).to_case(Case::UpperCamel));

                res.extend(content.generate(&field_type));

                res.extend(quote! {
                    #[derive(Debug)]
                    pub struct #ident {
                        value: Vec<#field_type>
                    }

                    impl TSParser for #ident {
                        fn parse(root: TSNode<'_>, source: &[u8]) -> ParseResult<Self> {
                            let mut value = vec![];

                            for node in root.child(&mut root.walk()){
                                value.push(TSParser::parser(node, source)?);
                            }

                            Ok(Self {
                                value
                            })
                        }
                    }
                });
            }
            RuleJSON::PREC_DYNAMIC { value: _, content }
            | RuleJSON::PREC_LEFT { value: _, content }
            | RuleJSON::PREC_RIGHT { value: _, content }
            | RuleJSON::PREC { value: _, content }
            | RuleJSON::TOKEN { content }
            | RuleJSON::IMMEDIATE_TOKEN { content } => {
                res.extend(content.generate(ident)?);
            }
        }

        Ok(res)
    }
}

fn is_recursive_type(ident: &str, value: &RuleJSON) -> bool {
    match value {
        RuleJSON::SYMBOL { name } => ident == name.to_case(Case::UpperCamel),
        RuleJSON::SEQ { members } => members.iter().all(|item| !is_recursive_type(ident, item)),
        RuleJSON::ALIAS { content, named: _, value: _ } => is_recursive_type(ident, content),
        _ => false,
    }
}

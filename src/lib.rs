#![allow(clippy::single_match)]
#![allow(clippy::single_char_pattern)]
#![allow(unused_imports)]
#![allow(clippy::needless_borrow)]

// mod f;
mod parse_grammar;
use std::collections::HashSet;
mod tools;
pub use tools::*;

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
        let mut res = TokenStream::new();
        for (name, rule) in &self.rules {
            let ident = syn::Ident::new(&name.to_case(Case::UpperCamel), Span::call_site());

            let snippet = rule.generate(&ident).unwrap();
            res.extend(snippet);
        }

        Ok(res)
    }
}

impl RuleJSON {
    pub fn generate(&self, ident: &Ident) -> anyhow::Result<TokenStream> {
        trace!("generate: {self:?}");

        let mut res = quote! {};

        match self {
            RuleJSON::ALIAS { content: _, named: _, value: _ } => {}
            RuleJSON::BLANK => {}
            RuleJSON::STRING { value: _ } => res.extend(quote! {
                #[derive(Debug)]
                pub struct #ident;
            }),
            RuleJSON::PATTERN { value: _, flags: _ } => res.extend(quote! {
                #[derive(Debug)]
                pub struct #ident{
                    value: String
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
                                let value = to_ident(value).to_case(Case::UpperCamel);
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
                            mem.extend(quote! {
                                #target_ident(#target_ident),
                            });
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
                                            let ident = ident!(&name.to_case(Case::UpperCamel));
                                            mid.extend(quote! {
                                                #ident,
                                            });
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
                        RuleJSON::BLANK => mem.extend(quote! {
                            Blank,
                        }),
                        _ => {
                            debug!("unhandled case for CHOICE: {item:?}");
                            mid.extend(quote! {
                                "todo" => { todo!() },
                            })
                        }
                    }
                }

                res.extend(quote! {
                    #[derive(Debug)]
                    pub enum #ident {
                        #mem
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
                        RuleJSON::STRING { value } => {
                            let name = to_ident(value);
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
                            mem.extend(quote! {
                                pub #field_name: #field_type,
                            });
                        }
                        RuleJSON::PATTERN { value, flags: _ } => {
                            let name = format!("{ident}_TOKEN_{idx}");
                            let field_name = ident!(&name.to_case(Case::Snake));
                            let field_type = ident!(&name.to_case(Case::UpperCamel));
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
                        _ => {
                            debug!("unhandled case for SEQ: {item:?}");
                        }
                    }
                }
                res.extend(quote! {
                    #[derive(Debug)]
                    pub struct #ident {
                        #mem
                    }

                });
            }
            RuleJSON::REPEAT { content } => {
                let token_ident = syn::Ident::new(
                    &format!("{}_TOKEN", ident).to_case(Case::UpperCamel),
                    Span::call_site(),
                );

                res.extend(content.generate(&token_ident));

                res.extend(quote! {
                    #[derive(Debug)]
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

                res.extend(content.generate(&token_ident));

                res.extend(quote! {
                    #[derive(Debug)]
                    pub struct #ident {
                        value: Vec<#token_ident>
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

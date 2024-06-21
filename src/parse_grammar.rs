#![allow(unused_variables)]
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum RuleJSON {
    ALIAS {
        content: Box<RuleJSON>,
        named: bool,
        value: String,
    },
    BLANK,
    STRING {
        value: String,
    },
    PATTERN {
        value: String,
        flags: Option<String>,
    },
    SYMBOL {
        name: String,
    },
    CHOICE {
        members: Vec<RuleJSON>,
    },
    FIELD {
        name: String,
        content: Box<RuleJSON>,
    },
    SEQ {
        members: Vec<RuleJSON>,
    },
    REPEAT {
        content: Box<RuleJSON>,
    },
    REPEAT1 {
        content: Box<RuleJSON>,
    },
    PREC_DYNAMIC {
        value: i32,
        content: Box<RuleJSON>,
    },
    PREC_LEFT {
        value: PrecedenceValueJSON,
        content: Box<RuleJSON>,
    },
    PREC_RIGHT {
        value: PrecedenceValueJSON,
        content: Box<RuleJSON>,
    },
    PREC {
        value: PrecedenceValueJSON,
        content: Box<RuleJSON>,
    },
    TOKEN {
        content: Box<RuleJSON>,
    },
    IMMEDIATE_TOKEN {
        content: Box<RuleJSON>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PrecedenceValueJSON {
    Integer(i32),
    Name(String),
}

#[derive(Debug, Deserialize)]
pub struct GrammarJSON {
    pub name: String,
    pub rules: HashMap<String, RuleJSON>,
    #[serde(default)]
    pub precedences: Vec<Vec<RuleJSON>>,
    #[serde(default)]
    pub conflicts: Vec<Vec<String>>,
    #[serde(default)]
    pub externals: Vec<RuleJSON>,
    #[serde(default)]
    pub extras: Vec<RuleJSON>,
    #[serde(default)]
    pub inline: Vec<String>,
    #[serde(default)]
    pub supertypes: Vec<String>,
    pub word: Option<String>,
}

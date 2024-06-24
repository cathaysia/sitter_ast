use convert_case::{Case, Casing};

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

    let value = value
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
        .replace("\r", "CarriageReturn");

    if value.starts_with(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']) {
        return "N_".to_owned() + &value;
    }

    value
}

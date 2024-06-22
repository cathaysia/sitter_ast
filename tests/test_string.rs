include!("./src/utils.rs");

#[test]
fn test_string() {
    test_ast(
        "unsigned_tiny_int",
        r#"
    {
      "type": "STRING",
      "value": "uint8"
    }
    "#,
        quote! {
            #[derive(Debug)]
            pub struct UnsignedTinyInt;
        },
    );
}

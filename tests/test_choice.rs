include!("./src/utils.rs");

#[test]
fn test_choice() {
    test_ast(
        "signed_short_int",
        r#"
    {
      "type": "CHOICE",
      "members": [
        {
          "type": "STRING",
          "value": "short"
        },
        {
          "type": "STRING",
          "value": "int16"
        }
      ]
    }
    "#,
        quote! {
            #[derive(Debug)]
            pub enum SignedShortInt {
                Short,
                Int16,
            }
        },
    );
}

include!("./src/utils.rs");

#[test]
fn test_seq() {
    assert!(test_ast(
        "ScopedName",
        r#"
    {
      "type": "CHOICE",
      "members": [
        {
          "type": "SYMBOL",
          "name": "identifier"
        },
        {
          "type": "SEQ",
          "members": [
            {
              "type": "STRING",
              "value": "::"
            },
            {
              "type": "SYMBOL",
              "name": "identifier"
            }
          ]
        },
        {
          "type": "SEQ",
          "members": [
            {
              "type": "SYMBOL",
              "name": "scoped_name"
            },
            {
              "type": "STRING",
              "value": "::"
            },
            {
              "type": "SYMBOL",
              "name": "identifier"
            }
          ]
        }
      ]
    }
        "#,
        quote! {
            #[derive(Debug)]
            pub enum ScopedName {
                Identifier(Identifier),
                Token1(Identifier),
                Token2(ScopedName, Identifier),
            }
        }
    ));
}

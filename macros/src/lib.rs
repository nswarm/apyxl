use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ValidateFeatures)]
pub fn derive_validate_features(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => panic!("ValidateFeatures can only be derived for enums"),
    };

    let test_fns: Vec<_> = variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let variant_name = &v.ident;
            let test_name = format_ident!("{}", variant_name.to_string().to_snake_case());
            quote! {
                #[test]
                fn #test_name() -> Result<()> {
                    let input = #enum_name::#variant_name.to_pyx_str();
                    let parser = Pyx::default();
                    let config = Config::default();
                    let mut input = input::Buffer::new(input);
                    let mut builder = model::Builder::default();
                    parser.parse(&config, &mut input, &mut builder)?;
                    let api = builder.into_api();
                    insta::assert_yaml_snapshot!(api);
                    Ok(())
                }
            }
        })
        .collect();

    let expanded = quote! {
        #[cfg(test)]
        mod feature_validation_tests {
            use super::#enum_name;
            use crate::parser::Pyx;
            use crate::parser::Config;
            use crate::{input, model, Parser};
            use anyhow::Result;

            #(#test_fns)*
        }
    };

    TokenStream::from(expanded)
}


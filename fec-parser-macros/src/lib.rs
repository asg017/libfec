extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

const MAPPINGS_JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/mappings2.json");

#[proc_macro]
pub fn gen_form_types(_: TokenStream) -> TokenStream {
    let json_data: serde_json::Value = {
        let contents =
            std::fs::read_to_string(MAPPINGS_JSON_PATH).expect("Unable to read the JSON file");
        serde_json::from_str(&contents).expect("JSON parsing error")
    };
    let keys: Vec<String> = json_data
        .as_object()
        .expect("JSON is not an object")
        .keys()
        .map(|key| key.to_string())
        .collect();

    let output = quote! {
        [
            #( #keys ),*
        ]
    };

    output.into()
}

#[proc_macro]
pub fn gen_form_type_version_set(_: TokenStream) -> TokenStream {
    let json_data: serde_json::Value = {
        let contents =
            std::fs::read_to_string(MAPPINGS_JSON_PATH).expect("Unable to read the JSON file");
        serde_json::from_str(&contents).expect("JSON parsing error")
    };
    let values = json_data
        .as_object()
        .expect("JSON is not an object")
        .values();

    let mut result = Vec::new();
    for value in values {
        let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

        let item = quote! {
          RegexSetBuilder::new([
            #( #keys ),*
          ])
            .case_insensitive(true)
            .build()
            .unwrap()
        };
        result.push(item);
    }

    let output = quote! {
      vec![
            #( #result ),*

        ]
    };

    output.into()
}
#[proc_macro]
pub fn gen_column_names(_: TokenStream) -> TokenStream {
    let json_data: serde_json::Value = {
        let contents =
            std::fs::read_to_string(MAPPINGS_JSON_PATH).expect("Unable to read the JSON file");
        serde_json::from_str(&contents).expect("JSON parsing error")
    };
    let mut form_types = vec![];

    for (_, value) in json_data.as_object().unwrap().iter() {
        let mut list_of_columns = vec![];

        for (_, item) in value.as_object().unwrap().iter() {
            let column_names: Vec<String> = item
                .as_array()
                .unwrap()
                .into_iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect();

            list_of_columns.push(quote! {
              vec![
                #( #column_names.to_string() ),*
              ]
            })
        }

        form_types.push(quote! {
          vec![
            #( #list_of_columns ),*
          ]
        })
    }

    let output = quote! {
      vec![
          #( #form_types ),*
        ]
    };

    output.into()
}

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

mod columnar;

/// `#[derive(Columnar)]`: implements `fec_parser::columnar::Columnar` (the
/// flattened column definitions plus a per-row push into column builders)
/// for a struct with named fields, or `fec_parser::columnar::ColumnarEnum`
/// (a per-variant dispatch) for an enum of single-field tuple variants. Only
/// usable inside fec-parser with its `columnar` feature; see
/// `fec_parser::columnar`.
#[proc_macro_derive(Columnar)]
pub fn derive_columnar(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    columnar::derive(input).into()
}

const DATE_COLUMNS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/date_columns.txt");
#[proc_macro]
pub fn gen_date_columns(_: TokenStream) -> TokenStream {
    let keys: Vec<String> = std::fs::read_to_string(DATE_COLUMNS_PATH)
        .expect("unable to read date-columns.txt")
        .lines()
        .map(|v| v.to_string())
        .collect();

    let output = quote! {
      [
          #( #keys.to_string() ),*
      ]
    };

    output.into()
}
const FLOAT_COLUMNS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/float-columns.txt");
#[proc_macro]
pub fn gen_float_columns(_: TokenStream) -> TokenStream {
    let keys: Vec<String> = std::fs::read_to_string(FLOAT_COLUMNS_PATH)
        .expect("unable to read date-columns.txt")
        .lines()
        .map(|v| v.to_string())
        .collect();

    let output = quote! {
      [
          #( #keys.to_string() ),*
      ]
    };

    output.into()
}

const MAPPINGS_JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/mappings2.json");

/// Rewrites a mappings2.json version key into the regex actually compiled.
///
/// The JSON keys are written loosely (`^8.5|8.4`, `^5.3|5.2|5.1|5.0|^3`): `.`
/// is unescaped and only the first alternative is anchored, so `8.4` would
/// match anywhere in the version string. The keys are kept as-is in the JSON
/// (same text as fecfile/FastFEC) and hardened here:
/// - every `.` becomes `\.`;
/// - a key with a top-level `|` (outside `(...)` / `[...]`) becomes
///   `^(?:alt1|alt2|...)`, with a leading `^` stripped from each alternative.
///
/// Keys whose alternatives are already grouped (`^(P3.4|P3.3)`,
/// `^P(3.1|3.0)`) only get the escaping.
fn harden_version_regex(key: &str) -> String {
    let escaped = key.replace('.', "\\.");
    let mut alternatives = Vec::new();
    let mut depth = 0i32;
    let mut in_class = false;
    let mut start = 0;
    for (i, c) in escaped.char_indices() {
        match c {
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => depth += 1,
            ')' if !in_class => depth -= 1,
            '|' if !in_class && depth == 0 => {
                alternatives.push(&escaped[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if alternatives.is_empty() {
        return escaped;
    }
    alternatives.push(&escaped[start..]);
    let stripped: Vec<&str> = alternatives
        .iter()
        .map(|alt| alt.strip_prefix('^').unwrap_or(alt))
        .collect();
    format!("^(?:{})", stripped.join("|"))
}

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
        let keys: Vec<String> = value
            .as_object()
            .unwrap()
            .keys()
            .map(|key| harden_version_regex(key))
            .collect();

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
                .iter()
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

#[cfg(test)]
mod tests {
    use super::harden_version_regex;

    #[test]
    fn harden_version_regex_rewrites() {
        for (key, expected) in [
            ("^8.5|8.4", r"^(?:8\.5|8\.4)"),
            ("^5.3|5.2|5.1|5.0|^3", r"^(?:5\.3|5\.2|5\.1|5\.0|3)"),
            ("^P3.2|^P3.3|^P3.4", r"^(?:P3\.2|P3\.3|P3\.4)"),
            ("^(P3.4|P3.3|P3.2)", r"^(P3\.4|P3\.3|P3\.2)"),
            ("^P(3.1|3.0|2.6)", r"^P(3\.1|3\.0|2\.6)"),
            ("^(5.1|5.0|3|2|1)", r"^(5\.1|5\.0|3|2|1)"),
            ("^[6-8]", "^[6-8]"),
            ("^1", "^1"),
            ("^(P3|P2.6)", r"^(P3|P2\.6)"),
            ("^3|^2", "^(?:3|2)"),
        ] {
            assert_eq!(harden_version_regex(key), expected, "{key}");
        }
    }
}

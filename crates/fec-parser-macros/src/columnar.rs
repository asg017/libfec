//! `#[derive(Columnar)]`: see `fec_parser::columnar` for the runtime half
//! (the traits, `ColumnDef`, `ColumnBuilder` and the leaf impls).
//!
//! The output names `crate::columnar::…`, so the derive is only usable inside
//! fec-parser, under its `columnar` feature.
//!
//! - A struct with named fields implements `Columnar`: one or more columns per
//!   field, in declaration order, nested struct fields flattened with their
//!   names joined by `_`. A `Vec<T>` field is **skipped** (Phase 0: no
//!   column) and reported by `Columnar::append_skipped`.
//! - An enum of single-field tuple variants implements `ColumnarEnum`: a
//!   dispatch on the variant, keyed by the variant's `#[serde(rename = "…")]`
//!   if it has one (`Itemization`: `SA`, `SC1`, …), else its name (`Cover`:
//!   `Form3X`, …).

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument, Lit, Meta, NestedMeta, PathArguments,
    Type,
};

const SUPPORTED: &str = "supported: String, &'static str, f64, bool, i16, u16, \
     jiff::civil::Date (`Date`), Option<T>, Box<T>, a struct deriving Columnar, \
     or (skipped, no column) a top-level Vec<T> field";

pub fn derive(input: DeriveInput) -> TokenStream {
    match expand(&input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

/// The `///` doc text of `attrs`: one line per `#[doc]`, with the single
/// leading space rustdoc adds stripped.
fn doc_text(attrs: &[Attribute]) -> String {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path.is_ident("doc"))
        .filter_map(|a| match a.parse_meta() {
            Ok(Meta::NameValue(nv)) => match nv.lit {
                Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        })
        .flat_map(|s| s.split('\n').map(str::to_owned).collect::<Vec<_>>())
        .map(|l| l.strip_prefix(' ').map(str::to_owned).unwrap_or(l))
        .collect();
    lines.join("\n")
}

/// `#[serde(rename = "…")]` on a variant, if any.
fn serde_rename(attrs: &[Attribute]) -> Option<String> {
    attrs
        .iter()
        .filter(|a| a.path.is_ident("serde"))
        .filter_map(|a| match a.parse_meta() {
            Ok(Meta::List(list)) => Some(list.nested),
            _ => None,
        })
        .flatten()
        .find_map(|nested| match nested {
            NestedMeta::Meta(Meta::NameValue(nv)) if nv.path.is_ident("rename") => match nv.lit {
                Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        })
}

/// `Some(inner)` if `ty` is `Vec<inner>`.
fn vec_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(tp) = ty else { return None };
    let seg = tp.path.segments.last()?;
    if seg.ident != "Vec" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    match args.args.iter().collect::<Vec<_>>().as_slice() {
        [GenericArgument::Type(inner)] => Some(inner),
        _ => None,
    }
}

/// Accept only field types `Columnar` is implemented for (or a plausible
/// struct name, whose `Columnar` impl the trait bound then checks), so an
/// unsupported type fails here with the field's name. `Vec` is handled by
/// the caller (top-level only).
fn check_ty(ty: &Type) -> Result<(), ()> {
    if let Type::Reference(r) = ty {
        // `&'static str` (e.g. a state name from a table) is Text.
        return match (&r.lifetime, r.mutability, &*r.elem) {
            (Some(l), None, Type::Path(p)) if l.ident == "static" && p.path.is_ident("str") => {
                Ok(())
            }
            _ => Err(()),
        };
    }
    let Type::Path(tp) = ty else { return Err(()) };
    if tp.qself.is_some() {
        return Err(());
    }
    let Some(seg) = tp.path.segments.last() else {
        return Err(());
    };
    let name = seg.ident.to_string();
    match (name.as_str(), &seg.arguments) {
        ("String" | "f64" | "bool" | "i16" | "u16" | "Date", PathArguments::None) => Ok(()),
        ("Option" | "Box", PathArguments::AngleBracketed(args)) => {
            match args.args.iter().collect::<Vec<_>>().as_slice() {
                [GenericArgument::Type(inner)] => check_ty(inner),
                _ => Err(()),
            }
        }
        (
            "Vec" | "i8" | "i32" | "i64" | "i128" | "isize" | "u8" | "u32" | "u64" | "u128"
            | "usize" | "f32" | "char" | "str" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet"
            | "IndexMap" | "IndexSet" | "Rc" | "Arc" | "Cow" | "Result" | "DateTime" | "Timestamp"
            | "Time" | "Zoned",
            _,
        ) => Err(()),
        (_, PathArguments::None) if name.starts_with(|c: char| c.is_ascii_uppercase()) => Ok(()),
        _ => Err(()),
    }
}

fn unsupported(ty: &Type, what: String) -> syn::Error {
    let ty_s = quote!(#ty)
        .to_string()
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace("& '", "&'");
    syn::Error::new_spanned(
        ty,
        format!("Columnar: {what} has unsupported type `{ty_s}` ({SUPPORTED})"),
    )
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let name = ident.to_string();
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Columnar: generic types are not supported",
        ));
    }
    let c = quote!(crate::columnar);

    match &input.data {
        Data::Struct(s) => {
            let Fields::Named(named) = &s.fields else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Columnar: only structs with named fields are supported",
                ));
            };
            let mut defs = Vec::new();
            let mut pushes = Vec::new();
            let mut absents = Vec::new();
            let mut skipped = Vec::new();
            for f in &named.named {
                let Some(fid) = f.ident.as_ref() else {
                    continue;
                };
                let fname = fid.to_string();
                let fname = fname.strip_prefix("r#").unwrap_or(&fname).to_owned();
                let ty = &f.ty;
                if let Some(inner) = vec_inner(ty) {
                    // Phase 0: a `Vec<Struct>` field gets no column; it is
                    // listed by `append_skipped` instead.
                    check_ty(inner)
                        .map_err(|_| unsupported(ty, format!("field `{fname}` of `{name}`")))?;
                    skipped.push(quote! {
                        out.push(#c::join_name(prefix, #fname));
                    });
                    continue;
                }
                check_ty(ty)
                    .map_err(|_| unsupported(ty, format!("field `{fname}` of `{name}`")))?;
                let fdoc = doc_text(&f.attrs);
                defs.push(quote! {
                    <#ty as #c::Columnar>::append_columns(
                        &#c::join_name(prefix, #fname), #fdoc, out,
                    );
                });
                pushes.push(quote! { #c::Columnar::push(&self.#fid, builders, at); });
                absents.push(quote! { <#ty as #c::Columnar>::push_absent(builders, at); });
                skipped.push(quote! {
                    <#ty as #c::Columnar>::append_skipped(&#c::join_name(prefix, #fname), out);
                });
            }
            Ok(quote! {
                #[allow(unused_variables)]
                impl #c::Columnar for #ident {
                    fn append_columns(
                        prefix: &str,
                        doc: &'static str,
                        out: &mut ::std::vec::Vec<#c::ColumnDef>,
                    ) {
                        #( #defs )*
                    }
                    fn push(&self, builders: &mut [#c::ColumnBuilder], at: &mut usize) {
                        #( #pushes )*
                    }
                    fn push_absent(builders: &mut [#c::ColumnBuilder], at: &mut usize) {
                        #( #absents )*
                    }
                    fn append_skipped(prefix: &str, out: &mut ::std::vec::Vec<::std::string::String>) {
                        #( #skipped )*
                    }
                }
            })
        }
        Data::Enum(e) => {
            let mut keys = Vec::new();
            let mut key_arms = Vec::new();
            let mut col_arms = Vec::new();
            let mut skip_arms = Vec::new();
            let mut push_arms = Vec::new();
            for v in &e.variants {
                let vid = &v.ident;
                let vname = vid.to_string();
                let ty = match &v.fields {
                    Fields::Unnamed(u) if u.unnamed.len() == 1 => &u.unnamed[0].ty,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            v,
                            format!(
                                "Columnar: variant `{name}::{vname}` must be a single-field \
                                 tuple variant (`{vname}(T)`); other enum shapes are not supported"
                            ),
                        ))
                    }
                };
                check_ty(ty).map_err(|_| unsupported(ty, format!("variant `{name}::{vname}`")))?;
                let key = serde_rename(&v.attrs).unwrap_or(vname);
                keys.push(key.clone());
                key_arms.push(quote! { #ident::#vid(_) => #key, });
                col_arms.push(quote! {
                    #key => ::std::option::Option::Some(<#ty as #c::Columnar>::columns("")),
                });
                skip_arms.push(quote! {
                    #key => ::std::option::Option::Some(<#ty as #c::Columnar>::skipped("")),
                });
                push_arms.push(quote! {
                    #ident::#vid(payload) => #c::Columnar::push(payload, builders, &mut at),
                });
            }
            Ok(quote! {
                impl #c::ColumnarEnum for #ident {
                    const KEYS: &'static [&'static str] = &[ #( #keys ),* ];
                    fn key(&self) -> &'static str {
                        match self { #( #key_arms )* }
                    }
                    fn columns_for(key: &str) -> ::std::option::Option<::std::vec::Vec<#c::ColumnDef>> {
                        match key {
                            #( #col_arms )*
                            _ => ::std::option::Option::None,
                        }
                    }
                    fn skipped_for(key: &str) -> ::std::option::Option<::std::vec::Vec<::std::string::String>> {
                        match key {
                            #( #skip_arms )*
                            _ => ::std::option::Option::None,
                        }
                    }
                    fn push_payload(&self, builders: &mut [#c::ColumnBuilder]) {
                        let mut at = 0usize;
                        match self { #( #push_arms )* }
                        #c::check_pushed(at, builders.len(), self.key());
                    }
                }
            })
        }
        Data::Union(_) => Err(syn::Error::new_spanned(
            ident,
            "Columnar: unions are not supported",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(src: &str) -> String {
        let input: DeriveInput = syn::parse_str(src).unwrap();
        expand(&input)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    #[test]
    fn rejects_unsupported_field_types() {
        let e = err("struct X { a: String, count: i32 }");
        assert!(
            e.contains("field `count` of `X`") && e.contains("`i32`"),
            "{e}"
        );
        assert!(err("struct X { a: Option<u64> }").contains("`Option<u64>`"));
        assert!(err("struct X { a: (String, String) }").contains("field `a`"));
        assert!(err("struct X { a: HashMap<String, String> }").contains("field `a`"));
        assert!(err("struct X { a: &'a str }").contains("field `a`"));
        assert!(err("struct X { a: Option<Vec<Address>> }").contains("field `a`"));
        assert!(err("struct X { a: Vec<Vec<Address>> }").contains("field `a`"));
        assert!(err("struct X { a: Vec<u64> }").contains("field `a`"));
        assert!(err("struct X { a: &'static str }").is_empty());
        assert!(err("struct X { a: Option<String>, b: Vec<Box<Address>>, d: Date }").is_empty());
        assert!(err("struct X { a: Option<i16>, b: Option<u16>, c: Option<bool> }").is_empty());
    }

    #[test]
    fn rejects_other_shapes() {
        assert!(err("enum E { A, B(String) }").contains("`E::A`"));
        assert!(err("enum E { A(String, String) }").contains("`E::A`"));
        assert!(err("enum E { A { x: String } }").contains("`E::A`"));
        assert!(err("struct X(String);").contains("named fields"));
        assert!(err("struct X<T> { a: T }").contains("generic"));
        assert!(err("enum E { A(Box<X>), B(Y) }").is_empty());
    }

    #[test]
    fn enum_keys_use_serde_rename() {
        let input: DeriveInput = syn::parse_str(
            "enum E { #[serde(rename = \"SA\")] ScheduleA(Box<ScheduleA>), Form3X(Form3X) }",
        )
        .unwrap();
        let out = expand(&input).unwrap().to_string();
        assert!(out.contains("& [\"SA\" , \"Form3X\"]"), "{out}");
    }

    #[test]
    fn doc_text_strips_one_space() {
        let input: DeriveInput =
            syn::parse_str("/// Line one.\n///   indented\n///\nstruct X { a: String }").unwrap();
        assert_eq!(doc_text(&input.attrs), "Line one.\n  indented\n");
    }
}

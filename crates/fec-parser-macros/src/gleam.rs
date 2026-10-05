//! `#[derive(GleamType)]`: see `fec_parser::gleam` for the runtime half
//! (the traits, the description types and the per-type encoders).
//!
//! The output names `crate::gleam::…`, so the derive is only usable inside
//! fec-parser, under its `gleam` feature.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, GenericArgument, Lit, Meta, PathArguments, Type};

/// Gleam reserved words; must match `fec_parser::gleam::GLEAM_KEYWORDS`.
const GLEAM_KEYWORDS: &[&str] = &[
    "as",
    "assert",
    "auto",
    "case",
    "const",
    "delegate",
    "derive",
    "echo",
    "else",
    "fn",
    "if",
    "implement",
    "import",
    "let",
    "macro",
    "opaque",
    "panic",
    "pub",
    "test",
    "todo",
    "type",
    "use",
];

const SUPPORTED: &str =
    "supported: String, &'static str, f64, bool, i16, u16, jiff::civil::Date (`Date`), \
     Option<T>, Vec<T>, Box<T>, or a struct deriving GleamType";

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

/// Accept only field types `GleamValue` is implemented for (or a plausible
/// struct name, whose `GleamValue` impl the trait bound then checks), so an
/// unsupported type fails here with the field's name.
fn check_ty(ty: &Type) -> Result<(), ()> {
    if let Type::Reference(r) = ty {
        // `&'static str` (e.g. a state code from a table) encodes like `String`.
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
        ("Option" | "Vec" | "Box", PathArguments::AngleBracketed(args)) => {
            match args.args.iter().collect::<Vec<_>>().as_slice() {
                [GenericArgument::Type(inner)] => check_ty(inner),
                _ => Err(()),
            }
        }
        (
            "i8" | "i32" | "i64" | "i128" | "isize" | "u8" | "u32" | "u64" | "u128" | "usize"
            | "f32" | "char" | "str" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "IndexMap"
            | "IndexSet" | "Rc" | "Arc" | "Cow" | "Result" | "DateTime" | "Timestamp" | "Time"
            | "Zoned",
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
        format!("GleamType: {what} has unsupported type `{ty_s}` ({SUPPORTED})"),
    )
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let name = ident.to_string();
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "GleamType: generic types are not supported",
        ));
    }
    let doc = doc_text(&input.attrs);
    let g = quote!(crate::gleam);
    let rustler = quote!(crate::gleam::rustler);

    let (shape, encode_body) = match &input.data {
        Data::Struct(s) => {
            let Fields::Named(named) = &s.fields else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "GleamType: only structs with named fields are supported",
                ));
            };
            let mut field_defs = Vec::new();
            let mut encodes = Vec::new();
            for f in &named.named {
                let Some(fid) = f.ident.as_ref() else {
                    continue;
                };
                let fname = fid.to_string();
                let fname = fname.strip_prefix("r#").unwrap_or(&fname).to_owned();
                if GLEAM_KEYWORDS.contains(&fname.as_str()) {
                    return Err(syn::Error::new_spanned(
                        fid,
                        format!(
                            "GleamType: field `{fname}` of `{name}` is a Gleam keyword; \
                             map it to a different label in the GleamType derive"
                        ),
                    ));
                }
                let ty = &f.ty;
                check_ty(ty)
                    .map_err(|_| unsupported(ty, format!("field `{fname}` of `{name}`")))?;
                let fdoc = doc_text(&f.attrs);
                field_defs.push(quote! {
                    #g::GleamField {
                        name: #fname,
                        ty: <#ty as #g::GleamValue>::gleam_ty(),
                        doc: #fdoc,
                    }
                });
                encodes.push(quote! { #g::GleamValue::encode_gleam(&self.#fid, env) });
            }
            let shape = quote! {
                #g::GleamShape::Record { fields: ::std::vec![ #( #field_defs ),* ] }
            };
            let encode = quote! {
                static ATOM: ::std::sync::OnceLock<#rustler::Atom> = ::std::sync::OnceLock::new();
                let terms = [ #g::__atom(env, &ATOM, #name), #( #encodes ),* ];
                #rustler::types::tuple::make_tuple(env, &terms)
            };
            (shape, encode)
        }
        Data::Enum(e) => {
            let mut variant_defs = Vec::new();
            let mut arms = Vec::new();
            for v in &e.variants {
                let vid = &v.ident;
                let vname = vid.to_string();
                let ty = match &v.fields {
                    Fields::Unnamed(u) if u.unnamed.len() == 1 => &u.unnamed[0].ty,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            v,
                            format!(
                                "GleamType: variant `{name}::{vname}` must be a single-field \
                                 tuple variant (`{vname}(T)`); other enum shapes are not supported"
                            ),
                        ))
                    }
                };
                check_ty(ty).map_err(|_| unsupported(ty, format!("variant `{name}::{vname}`")))?;
                let vdoc = doc_text(&v.attrs);
                variant_defs.push(quote! {
                    #g::GleamVariant {
                        name: #vname,
                        payload: <#ty as #g::GleamValue>::gleam_ty(),
                        doc: #vdoc,
                    }
                });
                arms.push(quote! {
                    #ident::#vid(payload) => {
                        static ATOM: ::std::sync::OnceLock<#rustler::Atom> =
                            ::std::sync::OnceLock::new();
                        #rustler::types::tuple::make_tuple(
                            env,
                            &[
                                #g::__atom(env, &ATOM, #vname),
                                #g::GleamValue::encode_gleam(payload, env),
                            ],
                        )
                    }
                });
            }
            let shape = quote! {
                #g::GleamShape::Variants { variants: ::std::vec![ #( #variant_defs ),* ] }
            };
            let encode = quote! { match self { #( #arms )* } };
            (shape, encode)
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                ident,
                "GleamType: unions are not supported",
            ))
        }
    };

    Ok(quote! {
        impl #g::GleamType for #ident {
            const RUST_NAME: &'static str = #name;
            const RUST_MODULE: &'static str = ::std::module_path!();
            fn gleam_def() -> #g::GleamDef {
                #g::GleamDef {
                    rust_name: #name,
                    rust_module: ::std::module_path!(),
                    doc: #doc,
                    shape: #shape,
                }
            }
        }

        impl #g::GleamValue for #ident {
            fn gleam_ty() -> #g::GleamTy {
                #g::GleamTy::Named(#g::GleamRef {
                    rust_name: #name,
                    rust_module: ::std::module_path!(),
                    def: <#ident as #g::GleamType>::gleam_def,
                })
            }
            fn encode_gleam<'a>(&self, env: #rustler::Env<'a>) -> #rustler::Term<'a> {
                #encode_body
            }
        }

        impl #rustler::Encoder for #ident {
            fn encode<'a>(&self, env: #rustler::Env<'a>) -> #rustler::Term<'a> {
                #g::GleamValue::encode_gleam(self, env)
            }
        }
    })
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
        assert!(err("struct X { a: &'static str }").is_empty());
        assert!(err("struct X { a: Option<String>, b: Vec<Box<Address>> }").is_empty());
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
    fn rejects_gleam_keywords() {
        assert!(err("struct X { r#type: String }").contains("`type` of `X` is a Gleam keyword"));
        assert!(err("struct X { todo: String }").contains("Gleam keyword"));
    }

    #[test]
    fn doc_text_strips_one_space() {
        let input: DeriveInput =
            syn::parse_str("/// Line one.\n///   indented\n///\nstruct X { a: String }").unwrap();
        assert_eq!(doc_text(&input.attrs), "Line one.\n  indented\n");
    }
}

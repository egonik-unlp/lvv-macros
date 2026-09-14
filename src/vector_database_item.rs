use proc_macro2::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, parse_quote, Data, DeriveInput, Fields};

use crate::attributes::{parse_attributes, serde_key, FieldWithAttributes};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(data_struct) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "Only structs can be VectorDatabaseItem",
        ));
    };
    let fields = parse_attributes(data_struct, &["description", "skip", "rename"])?;
    let ident = &input.ident;
    let category = ident.unraw().to_string();

    let description = description_impl(&input, &fields)?;
    let payload = payload_impl(&input, &data_struct.fields, &fields)?;

    // `Self: Serialize + DeserializeOwned` is the trait's supertrait bound, and every
    // description field must be describable. Both matter for generic items.
    let mut generics = input.generics.clone();
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let predicates = &mut generics.make_where_clause().predicates;
    predicates.push(parse_quote! {
        #ident #ty_generics: ::lvv::__private::serde::Serialize
            + ::lvv::__private::serde::de::DeserializeOwned
    });
    for fwa in fields.iter().filter(|fwa| fwa.is_description()) {
        let ty = &fwa.field.ty;
        predicates.push(parse_quote!(#ty: ::lvv::transform::transform::IntoDescriptionValue));
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::lvv::transform::transform::VectorDatabaseItem for #ident #ty_generics #where_clause {
            fn category(&self) -> ::std::string::String {
                ::std::string::String::from(#category)
            }

            #description

            #payload
        }
    })
}

/// One line per non-empty `#[lvv(description)]` field.
fn description_impl(
    input: &DeriveInput,
    fields: &[FieldWithAttributes],
) -> syn::Result<TokenStream> {
    let members: Vec<_> = fields
        .iter()
        .enumerate()
        .filter(|(_, fwa)| fwa.is_description())
        .map(|(index, fwa)| fwa.member(index))
        .collect();
    if members.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "VectorDatabaseItem needs at least one `#[lvv(description)]` field",
        ));
    }

    Ok(quote! {
        fn into_description(&self) -> ::std::string::String {
            let parts: ::std::vec::Vec<::std::string::String> = ::std::vec![
                #(::lvv::transform::transform::IntoDescriptionValue::into_description_value(&self.#members)),*
            ];
            parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<::std::vec::Vec<_>>()
                .join("\n")
        }
    })
}

/// Serializes `self` with its own `Serialize` impl, so serde attributes keep
/// working, then drops `#[lvv(skip)]` keys, applies `#[lvv(rename)]` and adds
/// the category.
fn payload_impl(
    input: &DeriveInput,
    shape: &Fields,
    fields: &[FieldWithAttributes],
) -> syn::Result<TokenStream> {
    let category = input.ident.unraw().to_string();

    let mut removed = Vec::new();
    let mut renamed_from = Vec::new();
    let mut renamed_to = Vec::new();
    for (index, fwa) in fields.iter().enumerate() {
        let key = match &fwa.field.ident {
            Some(ident) => serde_key(&ident.unraw().to_string(), &fwa.field.attrs, &input.attrs),
            None => index.to_string(),
        };
        if fwa.is_skipped() {
            removed.push(key);
            continue;
        }
        let target = fwa.rename().map_or_else(|| key.clone(), str::to_owned);
        if target == "category" {
            return Err(syn::Error::new_spanned(
                &fwa.field,
                "payload key `category` is reserved for the item category; \
                 use `#[lvv(rename = \"...\")]` or `#[lvv(skip)]`",
            ));
        }
        if target != key {
            renamed_from.push(key);
            renamed_to.push(target);
        }
    }

    let object = match shape {
        Fields::Named(_) => quote! {
            let Value::Object(mut object) = value else {
                ::lvv::__private::anyhow::bail!("`{}` did not serialize to a JSON object", #category);
            };
        },
        // Newtypes serialize as their inner value.
        Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => quote! {
            let mut object = Map::new();
            object.insert(::std::string::String::from("0"), value);
        },
        Fields::Unnamed(_) => quote! {
            let Value::Array(values) = value else {
                ::lvv::__private::anyhow::bail!("`{}` did not serialize to a JSON array", #category);
            };
            let mut object: Map<::std::string::String, Value> = values
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value))
                .collect();
        },
        Fields::Unit => quote! {
            let _ = value;
            let mut object = Map::new();
        },
    };

    Ok(quote! {
        fn into_payload(&self) -> ::lvv::__private::anyhow::Result<::lvv::__private::Payload> {
            use ::lvv::__private::serde_json::{self, Map, Value};

            let value = serde_json::to_value(self)?;
            #object
            #(object.remove(#removed);)*
            #(
                if let ::std::option::Option::Some(value) = object.remove(#renamed_from) {
                    object.insert(::std::string::String::from(#renamed_to), value);
                }
            )*
            object.insert(
                ::std::string::String::from("category"),
                Value::String(::std::string::String::from(#category)),
            );
            let payload = <::lvv::__private::Payload as ::std::convert::TryFrom<Value>>::try_from(
                Value::Object(object),
            )?;
            ::std::result::Result::Ok(payload)
        }
    })
}

#[cfg(test)]
pub mod test {
    use syn::parse_quote;

    use super::*;

    fn research_article() -> DeriveInput {
        parse_quote! {
            #[derive(Debug, Clone, Serialize, Deserialize)]
            struct ResearchArticle<'a> {
                /// Upstream identifier, never embedded
                #[lvv(skip)]
                id: u64,
                #[lvv(rename = "headline")]
                #[lvv(description)]
                title: String,
                #[serde(rename = "abstract")]
                #[lvv(description)]
                abstract_text: Option<String>,
                #[serde(default)]
                authors: Vec<Author>,
                #[lvv(rename = "keywords")]
                tags: Option<Vec<String>>,
                #[lvv(skip)]
                #[serde(skip_serializing)]
                raw_html: &'a str,
                published_at: chrono::DateTime<chrono::Utc>,
                #[lvv(rename = "citations")]
                citation_counts: std::collections::HashMap<String, u32>,
                #[lvv(skip)]
                #[lvv(rename = "vector")]
                embedding: [f32; 768],
                doi: Option<&'a str>,
                related: Vec<(u64, f32)>,
            }
        }
    }

    #[test]
    fn lets_see_more() -> syn::Result<()> {
        let res = expand(research_article())?;

        match syn::parse2::<syn::File>(res.clone()) {
            Ok(file) => println!("{}", prettyplease::unparse(&file)),
            Err(e) => println!("--- unparseable: {e} ---\n{res}"),
        };
        Ok(())
    }

    #[test]
    fn requires_a_description_field() {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                name: String,
            }
        };
        assert!(expand(input).is_err());
    }

    #[test]
    fn rejects_reserved_category_key() {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                #[lvv(description)]
                name: String,
                category: String,
            }
        };
        assert!(expand(input).is_err());
    }
}

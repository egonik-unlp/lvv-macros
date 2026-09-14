use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, GenericArgument, PathArguments, PathSegment, Type};

use crate::{
    attributes::parse_attributes,
    types::{RootField, RootFieldType},
};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(data_struct) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "Only structs can be VectorDatabase",
        ));
    };
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let field_code: Vec<_> = parse_attributes(data_struct, &["skip"])?
        .iter()
        .enumerate()
        .filter(|(_, fwa)| !fwa.is_skipped())
        .map(|(index, fwa)| RootField {
            member: fwa.member(index),
            field_type: resolve_point_field_type(&fwa.field.ty),
        })
        .map(generate_field_code)
        .collect();

    Ok(quote! {
        impl #impl_generics ::lvv::transform::transform::VectorDatabase for #name #ty_generics #where_clause {
            fn point_drafts(
                &self,
            ) -> ::lvv::__private::anyhow::Result<::std::vec::Vec<::lvv::transform::transform::VectorPointDraft>> {
                // Method syntax (auto-deref through `&`, `Box`, ...) without making
                // the caller import the trait.
                use ::lvv::transform::transform::VectorDatabaseItem as _;
                let mut points = ::std::vec::Vec::new();
                #(#field_code)*
                ::std::result::Result::Ok(points)
            }
        }
    })
}

fn is_map(ty: &Type) -> bool {
    last_segment(ty).is_some_and(|segment| {
        (segment.ident == "BTreeMap" || segment.ident == "HashMap")
            && matches!(segment.arguments, PathArguments::AngleBracketed(_))
    })
}

fn resolve_point_field_type(ty: &Type) -> RootFieldType {
    if is_vec(ty) {
        RootFieldType::Collection
    } else if is_map(ty) {
        RootFieldType::Map
    } else if let Some(inner) = option_inner(ty) {
        if is_vec(inner) {
            RootFieldType::OptionCollection
        } else if is_map(inner) {
            RootFieldType::OptionMap
        } else {
            RootFieldType::OptionScalar
        }
    } else {
        RootFieldType::Scalar
    }
}

fn generate_field_code(point_field: RootField) -> TokenStream {
    let RootField { member, field_type } = point_field;
    match field_type {
        RootFieldType::Collection => quote! {
            for item in &self.#member {
                points.push(item.try_into_database_item()?);
            }
        },
        RootFieldType::Scalar => quote! {
            points.push(self.#member.try_into_database_item()?);
        },
        RootFieldType::OptionScalar => quote! {
            if let ::std::option::Option::Some(item) = &self.#member {
                points.push(item.try_into_database_item()?);
            }
        },
        RootFieldType::OptionCollection => quote! {
            if let ::std::option::Option::Some(items) = &self.#member {
                for item in items {
                    points.push(item.try_into_database_item()?);
                }
            }
        },
        RootFieldType::Map => quote! {
            for item in self.#member.values() {
                points.push(item.try_into_database_item()?);
            }
        },
        RootFieldType::OptionMap => quote! {
            if let ::std::option::Option::Some(items) = &self.#member {
                for item in items.values() {
                    points.push(item.try_into_database_item()?);
                }
            }
        },
    }
}

fn last_segment(ty: &Type) -> Option<&PathSegment> {
    let Type::Path(tp) = ty else {
        return None;
    };
    tp.path.segments.last()
}

fn is_vec(ty: &Type) -> bool {
    last_segment(ty).is_some_and(|segment| {
        segment.ident == "Vec" && matches!(segment.arguments, PathArguments::AngleBracketed(_))
    })
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let segment = last_segment(ty).filter(|segment| segment.ident == "Option")?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn classifies_field_shapes() {
        let shape = |ty: Type| resolve_point_field_type(&ty);

        assert!(matches!(shape(parse_quote!(Item)), RootFieldType::Scalar));
        assert!(matches!(shape(parse_quote!(Vec<Item>)), RootFieldType::Collection));
        assert!(matches!(
            shape(parse_quote!(std::vec::Vec<Item>)),
            RootFieldType::Collection
        ));
        assert!(matches!(
            shape(parse_quote!(BTreeMap<String, Item>)),
            RootFieldType::Map
        ));
        assert!(matches!(
            shape(parse_quote!(std::collections::HashMap<u16, Item>)),
            RootFieldType::Map
        ));
        assert!(matches!(
            shape(parse_quote!(Option<Item>)),
            RootFieldType::OptionScalar
        ));
        assert!(matches!(
            shape(parse_quote!(Option<Vec<Item>>)),
            RootFieldType::OptionCollection
        ));
        assert!(matches!(
            shape(parse_quote!(Option<BTreeMap<String, Item>>)),
            RootFieldType::OptionMap
        ));
    }
}

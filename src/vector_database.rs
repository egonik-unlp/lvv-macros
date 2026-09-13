use quote::quote;
use syn::{Data::Struct, DeriveInput, Field, GenericArgument, PathArguments, Type};

use crate::types::{RootField, RootFieldType};

pub fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let data_struct = match &input.data {
        Struct(data) => data,
        _ => {
            return Err(
                syn::Error::new_spanned(&input, "Only structs can be VectorDatabase").into(),
            )
        }
    };
    let field_code: Vec<_> = data_struct
        .fields
        .iter()
        .map(|field| build_representation_from_field(field))
        .map(|vpf| generate_field_code(vpf))
        .collect();

    let codegen = quote! {
        impl ::lvv::transform::transform::VectorDatabase for #name {
            fn point_drafts(&self) -> anyhow::Result<Vec<lvv::transform::transform::VectorPointDraft>> {
                let mut points = Vec::new();
                #(#field_code)*
                Ok(points)
            }
        }
    };
    Ok(codegen)
}

fn resolve_point_field_type(field: &Field) -> RootFieldType {
    let field_ty = &field.ty;
    if is_vec(field_ty) {
        RootFieldType::Collection
    } else if is_option_vec(field_ty) {
        RootFieldType::OptionCollection
    } else {
        RootFieldType::Scalar
    }
}

fn build_representation_from_field(field: &Field) -> RootField {
    let ty = (&field.ty).to_owned();
    let ident = field.ident.clone().unwrap();
    let field_type = resolve_point_field_type(field);
    RootField {
        ty,
        ident,
        field_type,
    }
}

fn generate_field_code(point_field: RootField) -> proc_macro2::TokenStream {
    let RootField {
        ident, field_type, ..
    } = point_field;
    match field_type {
        RootFieldType::Collection => quote! {
            for item in &self.#ident {
                points.push(item.try_into_database_item()?);
            }
        },
        RootFieldType::Scalar => quote! {
            points.push(self.#ident.try_into_database_item()?);
        },
        RootFieldType::OptionCollection => quote! {
            if let Some(items) = &self.#ident {
                for item in items {
                    points.push(item.try_into_database_item()?);
                }
            }
        },
    }
}

fn is_vec(field_type: &Type) -> bool {
    let Type::Path(tp) = field_type else {
        return false;
    };
    let segment = tp.path.segments.last().unwrap();
    matches!(&segment.arguments, PathArguments::AngleBracketed(_)) && segment.ident.eq("Vec")
}

fn is_option_vec(field_type: &Type) -> bool {
    let Type::Path(tp) = field_type else {
        return false;
    };
    let segment = tp.path.segments.last().unwrap();
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    let GenericArgument::Type(inner_ty) = args.args.first().unwrap() else {
        return false;
    };
    segment.ident.eq("Option") && is_vec(inner_ty)
}

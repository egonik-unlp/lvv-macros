use quote::quote;
use syn::{Data::Struct, DeriveInput, Field, GenericArgument, Ident, PathArguments, Type};

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
        impl lvv::transform::transform::VectorDatabase for #name {
            fn point_drafts(&self) -> anyhow::Result<Vec<lvv::transform::transform::VectorPointDraft>> {
                let mut points = Vec::new();
                #(#field_code)*
                Ok(points)
            }
        }
    };
    Ok(codegen)
}

enum VectorPointFieldType {
    Collection,
    OptionCollection,
    Scalar,
}

struct VectorPointField {
    #[allow(dead_code)]
    ty: Type,
    ident: Ident,
    field_type: VectorPointFieldType,
}

fn resolve_point_field_type(field: &Field) -> VectorPointFieldType {
    let field_ty = &field.ty;
    if is_vec(field_ty) {
        VectorPointFieldType::Collection
    } else if is_option_vec(field_ty) {
        VectorPointFieldType::OptionCollection
    } else {
        VectorPointFieldType::Scalar
    }
}

fn build_representation_from_field(field: &Field) -> VectorPointField {
    let ty = (&field.ty).to_owned();
    let ident = field.ident.clone().unwrap();
    let field_type = resolve_point_field_type(field);
    VectorPointField {
        ty,
        ident,
        field_type,
    }
}
fn generate_field_code(point_field: VectorPointField) -> proc_macro2::TokenStream {
    let VectorPointField {
        ident, field_type, ..
    } = point_field;
    match field_type {
        VectorPointFieldType::Collection => quote! {
            for item in &self.#ident {
                points.push(item.try_into_database_item()?);
            }
        },
        VectorPointFieldType::Scalar => quote! {
            points.push(&self.#ident.try_into_database_item()?);
        },
        VectorPointFieldType::OptionCollection => quote! {
            if let Some(items) = &self.#ident {
                for item in items {
                    points.push(items.try_into_database_item()?);
                }
            }
        },
    }
}

fn is_vec(field_type: &Type) -> bool {
    let Type::Path(tp) = &field_type else {
        panic!("chau")
    };
    let segment = tp.path.segments.last().unwrap();
    matches!(&segment.arguments, PathArguments::AngleBracketed(_)) && segment.ident.eq("Vec")
}

fn is_option_vec(field_type: &Type) -> bool {
    let Type::Path(tp) = &field_type else {
        panic!("chau")
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

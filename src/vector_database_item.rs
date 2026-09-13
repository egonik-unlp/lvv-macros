use crate::{
    attributes::{parse_attributes, FieldWithAttributes},
    types::Command,
};
use quote::quote;
use syn::{Data::Struct, DeriveInput};
// pub trait VectorDatabaseItem: DeserializeOwned + Serialize {
//     fn category(&self) -> &'static str;
//    fn into_description(&self) -> String;
//     fn into_payload(&self) -> anyhow::Result<Payload> {
//         let payload: Payload = serde_json::to_value(self)
//             .map_err(|err| anyhow::anyhow!(err))?
//             .try_into()?;
//         Ok(payload)
//     }
//     fn try_into_database_item(&self) -> anyhow::Result<VectorPointDraft> {
//         let payload = self.into_payload()?;
//         Ok(VectorPointDraft {
//             category: self.category(),
//             description: self.into_description(),
//             payload: payload,
//         })
//     }
// }
pub fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let data_struct = match &input.data {
        Struct(data) => data,
        _ => {
            return Err(
                syn::Error::new_spanned(&input, "Only structs can be VectorDatabase").into(),
            );
        }
    };
    let parsed_attributes = parse_attributes(data_struct, &["description", "skip", "rename"])?;
    let temp_struct = generate_shadow_struct(parsed_attributes.clone(), &input);
    let temp_struct_description = generate_description_shadow_struct(parsed_attributes, &input);
    let category_impl = category_impl(&input);
    Ok(quote! {
        #category_impl
        #temp_struct_description
        #temp_struct
    })
}

fn category_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let ident = input.clone().ident;
    let ident_string = ident.to_string();
    quote! {
        impl #ident {
            fn category() -> String {
                #ident_string
            }
        }
    }
}

fn generate_description_shadow_struct(
    fields: Vec<FieldWithAttributes>,
    input: &DeriveInput,
) -> proc_macro2::TokenStream {
    let ident = input.clone().ident;
    let shadow_name = quote! {ShadowStruct};
    let mut shadow_generics = input.generics.clone();
    shadow_generics
        .params
        .insert(0, syn::GenericParam::Lifetime(syn::parse_quote!( '__l   )));
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let (shadow_impl, shadow_ty, _) = shadow_generics.split_for_impl();
    let fields_with_types = fields
        .clone()
        .into_iter()
        .filter(|f| !f.attributes.contains(&Command::Skip))
        .filter(|field| {
            field
                .attributes
                .iter()
                .any(|attr| matches!(attr, Command::Description))
        })
        .flat_map(|fwa| {
            fwa.field.ident.map(move |i| {
                let ty = fwa.field.ty;
                quote! { #i: &'__l #ty, }
            })
        })
        .collect::<Vec<_>>();
    let fields_assigned = fields
        .into_iter()
        .filter(|f| !f.attributes.contains(&Command::Skip))
        .filter(|field| {
            field
                .attributes
                .iter()
                .any(|attr| matches!(attr, Command::Description))
        })
        .flat_map(|fwa| fwa.field.ident.map(move |i| quote! { #i: &self.#i, }))
        .collect::<Vec<_>>();
    let converted_value = quote! {
            &#shadow_name {
                #(#fields_assigned)*
            }
    };
    quote! {
        let _: () = {
            #[derive(Serialize, Deserialize)]
            struct #shadow_name #shadow_impl #where_clause {
                category: String,
                #(#fields_with_types)*
            }

            impl #ident {
                fn into_description(&self) -> anyhow::Result<Payload> {
                    let converted_value = #converted_value;
                    let payload: Payload = serde_json::to_value(converted_value)
                    .map_err(|err| anyhow::anyhow!(err))?
                    .try_into()?;
                Ok(payload)
            }



            }
        };
    }
}

fn generate_shadow_struct(
    fields: Vec<FieldWithAttributes>,
    input: &DeriveInput,
) -> proc_macro2::TokenStream {
    let ident = input.clone().ident;
    let shadow_name = quote! {ShadowStruct};
    let mut shadow_generics = input.generics.clone();
    shadow_generics
        .params
        .insert(0, syn::GenericParam::Lifetime(syn::parse_quote!( '__l   )));
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let (shadow_impl, shadow_ty, _) = shadow_generics.split_for_impl();
    let fields_with_types = fields
        .clone()
        .into_iter()
        .filter(|f| !f.attributes.contains(&Command::Skip))
        .flat_map(|fwa| {
            fwa.field.ident.map(move |i| {
                let ty = fwa.field.ty;
                quote! { #i: &'__l #ty, }
            })
        })
        .collect::<Vec<_>>();
    let fields_assigned = fields
        .into_iter()
        .filter(|f| !f.attributes.contains(&Command::Skip))
        .flat_map(|fwa| fwa.field.ident.map(move |i| quote! { #i: &self.#i, }))
        .collect::<Vec<_>>();
    let converted_value = quote! {
            &#shadow_name {
                category: self.category(),
                #(#fields_assigned)*
            }
    };
    quote! {
        let _: () = {
            #[derive(Serialize, Deserialize)]
            struct #shadow_name #shadow_impl #where_clause {
                category: String,
                #(#fields_with_types)*
            }

            impl #ident {
                fn into_payload(&self) -> anyhow::Result<Payload> {
                    let converted_value = #converted_value;
                    let payload: Payload = serde_json::to_value(converted_value)
                    .map_err(|err| anyhow::anyhow!(err))?
                    .try_into()?;
                Ok(payload)
            }



            }
        };
    }
}

#[cfg(test)]
pub mod test {
    use syn::{parse_quote, Data};

    use super::*;
    fn helper_struct() -> syn::Result<(Vec<FieldWithAttributes>, DeriveInput)> {
        let input: DeriveInput = parse_quote! {
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
        };
        let Data::Struct(data) = input.clone().data else {
            panic!()
        };

        parse_attributes(&data, &["rename", "skip", "description"]).map(|f| (f, input))
    }

    #[test]
    fn lets_see() {
        let (fields, input) = helper_struct().unwrap();
        let res = generate_shadow_struct(fields, &input);

        let wrapped = quote! { fn __preview() { #res } };

        match syn::parse2::<syn::File>(wrapped) {
            Ok(file) => println!("{}", prettyplease::unparse(&file)),
            Err(e) => println!("--- unparseable: {e} ---\n{res}"),
        }
    }

    #[test]
    fn lets_see_more() -> syn::Result<()> {
        let (_, input) = helper_struct().unwrap();
        let res = expand(input)?;

        let wrapped = quote! { fn __preview() { #res } };

        match syn::parse2::<syn::File>(wrapped) {
            Ok(file) => println!("{}", prettyplease::unparse(&file)),
            Err(e) => println!("--- unparseable: {e} ---\n{res}"),
        };
        Ok(())
    }
}

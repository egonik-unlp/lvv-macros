use crate::{
    attributes::parse_attributes,
    types::{Command, InnerField, Modifier, IDENTS},
};
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use std::todo;
use syn::{
    parse_macro_input, spanned::Spanned, token::In, Data::Struct, DeriveInput, Field, LitStr,
};
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
    let name = &input.ident;
    let data_struct = match &input.data {
        Struct(data) => data,
        _ => {
            return Err(
                syn::Error::new_spanned(&input, "Only structs can be VectorDatabase").into(),
            );
        }
    };
    let parsed_fields: Vec<_> = data_struct
        .fields
        .iter()
        .map(|field| inspect_field(field))
        .collect();
    let parsed_attributes = parse_attributes(data_struct, &["description", "skip"])?;
    todo!()
}

fn inspect_field(field: &Field) -> syn::Result<InnerField> {
    let mut parsed_commands = vec![];
    let ident = field.ident.clone().unwrap();
    field
        .attrs
        .clone()
        .into_iter()
        .filter(|attr| attr.path().is_ident("lvv"))
        .try_for_each(|attr| {
            attr.parse_nested_meta(|meta| {
                let Some(comm) = IDENTS.into_iter().find(|ident| meta.path.is_ident(ident)) else {
                    return Err(meta.error("unsupported vector_database command"));
                };
                match comm {
                    "skip" => parsed_commands.push(Command::Skip),
                    "rename" => {
                        let value: LitStr = meta.value()?.parse()?;
                        parsed_commands.push(Command::Rename(value.value()));
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })
        })?;
    // handle this differently because i removed the enum in the type
    let skip = parsed_commands
        .iter()
        .any(|command| matches!(command, Command::Skip));
    let rename = parsed_commands.iter().find_map(|command| match command {
        Command::Rename(new_name) => Some(new_name.to_string()),
        _ => None,
    });
    Ok(InnerField {
        ident,
        skip,
        rename,
    })
}

// Final
fn category_impl(input: &InnerField) -> proc_macro2::TokenStream {
    let cat = if let Some(new_name) = input.rename.clone() {
        new_name
    } else {
        input.ident.to_string().to_snake_case()
    };
    quote! {
        fn category(&self) -> String {
            #cat
        }
    }
}
//TODO: Write
fn description_impl(input: &InnerField) -> proc_macro2::TokenStream {
    quote! {
        fn into_description(&self) -> String {

        }
    }
}
// TODO: Solve
fn into_payload() -> proc_macro2::TokenStream {
    todo!()
}

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data::Struct, DeriveInput};

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
    todo!()
}

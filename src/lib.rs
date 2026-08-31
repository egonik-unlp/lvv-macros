use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
mod types;
mod vector_database;
mod vector_database_item;

#[proc_macro_derive(VectorDatabase, attributes(vector_database))]
pub fn vector_database(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match vector_database::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(VectorDatabaseItem, attributes(vector_database_item))]
pub fn vector_database_item(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match vector_database_item::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

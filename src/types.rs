use syn::{Ident, Type};

pub enum RootFieldType {
    Collection,
    OptionCollection,
    Scalar,
}

pub struct RootField {
    #[allow(dead_code)]
    pub ty: Type,
    pub ident: Ident,
    pub field_type: RootFieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Description,
    Rename(String),
    Skip,
}

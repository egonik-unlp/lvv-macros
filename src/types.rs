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

pub struct InnerField {
    pub ident: Ident,
    pub rename: Option<String>,
    pub skip: bool,
}
pub struct ParsedInnerField {
    pub ident: Ident,
    pub commands: Vec<Command>,
}

pub enum Modifier {
    Ignore,
    EndPoint,
}

pub const IDENTS: [&'static str; 2] = ["skip", "rename"];

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Flatten,
    Description,
    Rename(String),
    Skip,
}

use syn::Member;

pub enum RootFieldType {
    Collection,
    OptionCollection,
    OptionScalar,
    Scalar,
    Map,
    OptionMap,
}

pub struct RootField {
    pub member: Member,
    pub field_type: RootFieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Description,
    Rename(String),
    Skip,
}

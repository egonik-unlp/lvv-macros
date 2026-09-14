use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToUpperCamelCase,
};
use syn::{
    punctuated::Punctuated, Attribute, DataStruct, Expr, ExprLit, Field, Index, Lit, LitStr,
    Member, Meta, Token,
};

use crate::types::Command;

#[derive(Clone)]
pub struct FieldWithAttributes {
    pub field: Field,
    pub attributes: Vec<Command>,
}

impl FieldWithAttributes {
    pub fn is_skipped(&self) -> bool {
        self.attributes.contains(&Command::Skip)
    }

    /// `skip` wins over `description`.
    pub fn is_description(&self) -> bool {
        !self.is_skipped() && self.attributes.contains(&Command::Description)
    }

    /// The last `rename` wins when several are given.
    pub fn rename(&self) -> Option<&str> {
        self.attributes.iter().rev().find_map(|command| match command {
            Command::Rename(name) => Some(name.as_str()),
            _ => None,
        })
    }

    /// `self.name` for named fields, `self.0` for tuple fields.
    pub fn member(&self, index: usize) -> Member {
        match &self.field.ident {
            Some(ident) => Member::Named(ident.clone()),
            None => Member::Unnamed(Index::from(index)),
        }
    }
}

pub(crate) fn parse_attributes(
    input: &DataStruct,
    valid_attributes: &[&str],
) -> syn::Result<Vec<FieldWithAttributes>> {
    input
        .fields
        .iter()
        .map(|field| {
            let mut attributes = Vec::new();
            for attr in field.attrs.iter().filter(|attr| attr.path().is_ident("lvv")) {
                // One attribute may hold several commands: `#[lvv(description, rename = "x")]`.
                attr.parse_nested_meta(|meta| {
                    let Some(command) = valid_attributes
                        .iter()
                        .find(|name| meta.path.is_ident(name))
                    else {
                        return Err(meta.error("unsupported lvv command"));
                    };
                    attributes.push(match *command {
                        "skip" => Command::Skip,
                        "description" => Command::Description,
                        "rename" => Command::Rename(meta.value()?.parse::<LitStr>()?.value()),
                        _ => unreachable!(),
                    });
                    Ok(())
                })?;
            }
            Ok(FieldWithAttributes {
                field: field.clone(),
                attributes,
            })
        })
        .collect()
}

/// The key serde gives a named field: its `#[serde(rename)]`, else the
/// container's `#[serde(rename_all)]` applied to the field name.
pub(crate) fn serde_key(
    field_name: &str,
    field_attrs: &[Attribute],
    container_attrs: &[Attribute],
) -> String {
    if let Some(rename) = serde_str_value(field_attrs, "rename") {
        return rename;
    }
    match serde_str_value(container_attrs, "rename_all").as_deref() {
        Some("lowercase") => field_name.to_lowercase(),
        Some("UPPERCASE") => field_name.to_uppercase(),
        Some("PascalCase") => field_name.to_upper_camel_case(),
        Some("camelCase") => field_name.to_lower_camel_case(),
        Some("SCREAMING_SNAKE_CASE") => field_name.to_shouty_snake_case(),
        Some("kebab-case") => field_name.to_kebab_case(),
        Some("SCREAMING-KEBAB-CASE") => field_name.to_shouty_kebab_case(),
        _ => field_name.to_string(),
    }
}

/// `#[serde(key = "value")]`. Serde attributes we can't parse are left to serde.
fn serde_str_value(attrs: &[Attribute], key: &str) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .find_map(|meta| match meta {
            Meta::NameValue(name_value) if name_value.path.is_ident(key) => {
                match name_value.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(value),
                        ..
                    }) => Some(value.value()),
                    _ => None,
                }
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use syn::{parse_quote, Data, DeriveInput};

    use super::*;

    #[test]
    fn parses_attributes() -> syn::Result<()> {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                #[lvv(rename = "bar")]
                field: String,
            }
        };
        let Data::Struct(data) = input.data else {
            panic!()
        };

        let result = parse_attributes(&data, &["rename", "skip"])?;
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].attributes[0], Command::Rename(_)));
        Ok(())
    }

    #[test]
    fn keeps_every_command_in_one_attribute() -> syn::Result<()> {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                #[lvv(description, rename = "role")]
                title: String,
            }
        };
        let Data::Struct(data) = input.data else {
            panic!()
        };

        let result = parse_attributes(&data, &["rename", "skip", "description"])?;
        assert_eq!(
            result[0].attributes,
            vec![Command::Description, Command::Rename("role".into())]
        );
        Ok(())
    }

    #[test]
    fn doesnt_parse_illegal_attributes() -> syn::Result<()> {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                #[lvv(va_cualquiera = "bar")]
                field: String,
            }
        };
        let Data::Struct(data) = input.data else {
            panic!()
        };

        let result =
            parse_attributes(&data, &["rename", "skip"]).inspect_err(|err| println!("{}", err));
        assert!(result.is_err());
        Ok(())
    }
    #[test]
    fn ignores_invalid_attr_name() -> syn::Result<()> {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                #[arre(rename = "bar")]
                field: String,
            }
        };
        let Data::Struct(data) = input.data else {
            panic!()
        };

        let result =
            parse_attributes(&data, &["rename", "skip"]).inspect_err(|err| println!("{}", err))?;
        assert_eq!(result.first().unwrap().attributes.len(), 0);
        Ok(())
    }

    #[test]
    fn no_attributes() -> syn::Result<()> {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                field: String,
            }
        };
        let Data::Struct(data) = input.data else {
            panic!()
        };

        let result =
            parse_attributes(&data, &["rename", "skip"]).inspect_err(|err| println!("{}", err))?;
        assert_eq!(result.first().unwrap().attributes.len(), 0);
        Ok(())
    }

    #[test]
    fn resolves_serde_keys() {
        let input: DeriveInput = parse_quote! {
            #[serde(rename_all = "camelCase")]
            struct Foo {
                #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
                abstract_text: Option<String>,
                years_of_experience: u8,
            }
        };
        let Data::Struct(data) = &input.data else {
            panic!()
        };

        let keys: Vec<_> = data
            .fields
            .iter()
            .map(|field| {
                let name = field.ident.as_ref().unwrap().to_string();
                serde_key(&name, &field.attrs, &input.attrs)
            })
            .collect();
        assert_eq!(keys, ["abstract", "yearsOfExperience"]);
    }
}

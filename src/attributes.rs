use syn::{DataStruct, Field, LitStr};

use crate::types::Command;

#[derive(Clone)]
pub struct FieldWithAttributes {
    pub field: Field,
    pub attributes: Vec<Command>,
}

pub(crate) fn parse_attributes(
    input: &DataStruct,
    valid_attributes: &'static [&'static str],
) -> syn::Result<Vec<FieldWithAttributes>> {
    input
        .fields
        .clone()
        .into_iter()
        .map(|field| {
            let attributes = field
                .attrs
                .clone()
                .into_iter()
                .filter(|attr| attr.path().is_ident("lvv"))
                .map(|attr| {
                    let mut attribute = None;

                    attr.parse_nested_meta(|meta| {
                        let Some(comm) = valid_attributes
                            .into_iter()
                            .find(|ident| meta.path.is_ident(ident))
                        else {
                            return Err(meta.error("unsupported vector_database command"));
                        };

                        attribute = match *comm {
                            "skip" => Some(Command::Skip),
                            "rename" => {
                                let value: LitStr = meta.value()?.parse()?;
                                Some(Command::Rename(value.value()))
                            }
                            "description" => Some(Command::Description),
                            _ => unreachable!(),
                        };
                        Ok(())
                    })
                    .map(|_| attribute)
                })
                .collect::<syn::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            Ok(FieldWithAttributes { attributes, field })
        })
        .collect::<syn::Result<Vec<_>>>()
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
}

#![warn(missing_docs)]

//! Derive macros for [`lvv`](https://docs.rs/lvv), turning Rust structs into
//! vector points.
//!
//! - [`VectorDatabaseItem`] makes a record type produce one point: the text to
//!   embed, a category, and the payload stored next to the vector.
//! - [`VectorDatabase`] makes a container struct produce the points of every
//!   item it holds.
//!
//! Both implement traits from
//! [`lvv::points`](https://docs.rs/lvv/latest/lvv/points/index.html).
//! The generated code refers to `::lvv`, so the crate using the derives must
//! depend on `lvv` under that name. Enable lvv's `derive` feature instead of
//! depending on this crate directly; it re-exports both macros next to the
//! traits they implement:
//!
//! ```toml
//! [dependencies]
//! lvv = { version = "0.5", features = ["derive"] }
//! serde = { version = "1", features = ["derive"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use lvv::points::{VectorDatabase, VectorDatabaseItem};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, VectorDatabaseItem)]
//! struct Project {
//!     #[lvv(description)]
//!     name: String,
//!     #[lvv(description)]
//!     summary: String,
//!     #[lvv(rename = "stack")]
//!     technologies: Vec<String>,
//!     #[lvv(skip)]
//!     private_notes: String,
//! }
//!
//! #[derive(VectorDatabase)]
//! struct Portfolio {
//!     projects: Vec<Project>,
//!     featured: Option<Project>,
//!     #[lvv(skip)]
//!     owner: String,
//! }
//!
//! # fn index(portfolio: &Portfolio) -> Result<(), lvv::points::PointError> {
//! for draft in portfolio.point_drafts()? {
//!     // Embed `draft.description`, store `draft.payload` next to the vector.
//!     println!("{}: {}", draft.category, draft.description);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! For a complete lvv pipeline built on these derives, from reading files to
//! writing Qdrant collections, see lvv's
//! [`examples/derive.rs`](https://github.com/egonik-unlp/lvv/blob/main/examples/derive.rs).

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
mod attributes;
mod types;
mod vector_database;
mod vector_database_item;

/// Derives `VectorDatabase` for a struct whose fields hold
/// [`VectorDatabaseItem`]s.
///
/// The generated `point_drafts` walks the fields in declaration order and
/// collects one `VectorPointDraft` per item. Each field must have one of these
/// shapes, where `T` implements `VectorDatabaseItem`:
///
/// | Field type                                    | Points                              |
/// |-----------------------------------------------|-------------------------------------|
/// | `T`                                           | one                                 |
/// | `Vec<T>`                                      | one per element                     |
/// | `BTreeMap<K, T>`, `HashMap<K, T>`             | one per value                       |
/// | `Option<T>`                                   | one, or none for `None`             |
/// | `Option<Vec<T>>`                              | one per element, or none for `None` |
/// | `Option<BTreeMap<K, T>>`, `Option<HashMap<K, T>>` | one per value, or none for `None` |
///
/// Map keys are not used. `BTreeMap` values come in key order, `HashMap`
/// values in arbitrary order.
///
/// Shapes are recognized by the type's last path segment, so
/// `std::vec::Vec<T>` works but a type alias for `Vec<T>` is treated as a
/// single item. Any other type is treated as a single item too and must
/// implement `VectorDatabaseItem` (`Box<T>` works); skip fields that aren't
/// items.
///
/// The container itself does not need to implement `Serialize` or
/// `Deserialize`.
///
/// # Field attributes
///
/// - `#[lvv(skip)]`: ignore the field. It is the only command accepted here;
///   any other `#[lvv(...)]` command is a compile error.
///
/// # Example
///
/// ```rust,ignore
/// use lvv::points::VectorDatabase;
///
/// #[derive(VectorDatabase)]
/// struct Portfolio {
///     owner: Contact,                        // one point
///     positions: Vec<Position>,              // one per position
///     current: Option<Position>,             // zero or one
///     publications: Option<Vec<Publication>>,
///     projects: BTreeMap<String, Project>,   // one per value
///     #[lvv(skip)]
///     updated_at: String,                    // not an item
/// }
///
/// let drafts = portfolio.point_drafts()?;
/// ```
#[proc_macro_derive(VectorDatabase, attributes(lvv))]
pub fn vector_database(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match vector_database::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives `VectorDatabaseItem` for a struct, so each value becomes one
/// vector point.
///
/// The generated implementation produces:
///
/// - **category**: the struct's name, such as `"Position"`.
/// - **description**: the text to embed. It joins the fields marked
///   `#[lvv(description)]` with newlines, in declaration order. Fields that
///   render to an empty string, such as a `None`, are left out.
/// - **payload**: the value serialized through its own `Serialize` impl,
///   without `#[lvv(skip)]` fields, with `#[lvv(rename)]` applied, and with a
///   `"category"` key added.
///
/// The struct must implement `serde::Serialize` and `serde::Deserialize`.
/// Named, tuple and generic structs are supported; the bounds a generic struct
/// needs are added to the impl.
///
/// # Field attributes
///
/// | Attribute                | Effect |
/// |--------------------------|--------|
/// | `#[lvv(description)]`    | Adds the field to the description. Its type must implement `IntoDescriptionValue`. |
/// | `#[lvv(skip)]`           | Leaves the field out of the payload and the description. Wins over `description`. |
/// | `#[lvv(rename = "key")]` | Stores the field under `key` in the payload. If given more than once, the last one wins. |
///
/// Commands can share one attribute: `#[lvv(description, rename = "role")]`.
///
/// `IntoDescriptionValue` is implemented for `String`, integer and float
/// primitives, `Option<T>` and `Vec<T>`. Implement it for your own types to use
/// them as description fields.
///
/// # Payload keys
///
/// `skip` and `rename` act on the key serde writes: the field's
/// `#[serde(rename = "...")]`, or its name under the container's
/// `#[serde(rename_all = "...")]`. Tuple struct fields use their index (`"0"`,
/// `"1"`, ...) as the key. Fields marked `#[serde(flatten)]` are not tracked,
/// so `skip` and `rename` have no effect on them.
///
/// # Compile errors
///
/// - The input is an enum or union.
/// - No field is marked `#[lvv(description)]`.
/// - A field would be stored under the reserved payload key `category`. Rename
///   or skip it.
/// - An `#[lvv(...)]` command other than `description`, `skip` or `rename`.
///
/// # Example
///
/// ```rust,ignore
/// use lvv::points::VectorDatabaseItem;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, VectorDatabaseItem)]
/// #[serde(rename_all = "camelCase")]
/// struct Position {
///     // Embedded, and stored under `role` instead of `jobTitle`.
///     #[lvv(description, rename = "role")]
///     job_title: String,
///     // Embedded when present.
///     #[lvv(description)]
///     organization: Option<String>,
///     // Stored as `startedOn`, following serde.
///     started_on: String,
///     // Neither embedded nor stored.
///     #[lvv(skip)]
///     internal_id: u64,
/// }
///
/// // Position { job_title: "Researcher", organization: Some("UNLP"), .. } becomes
/// //   category:    "Position"
/// //   description: "Researcher\nUNLP"
/// //   payload:     {"role": "Researcher", "organization": "UNLP",
/// //                 "startedOn": "2020-03-01", "category": "Position"}
/// ```
#[proc_macro_derive(VectorDatabaseItem, attributes(lvv))]
pub fn vector_database_item(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match vector_database_item::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

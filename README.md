# lvv-macros

Derive macros for [`lvv`](https://crates.io/crates/lvv) that turn Rust structs
into vector points.

- `#[derive(VectorDatabaseItem)]` makes a record produce one point: the text to
  embed, a category, and the payload stored next to the vector.
- `#[derive(VectorDatabase)]` makes a container struct produce the points of
  every item it holds.

## Usage

Use the macros through lvv's `derive` feature. The generated code refers to
`::lvv`, and the feature re-exports both macros next to the traits they
implement:

```toml
[dependencies]
lvv = { version = "0.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

```rust,ignore
use lvv::transform::transform::{VectorDatabase, VectorDatabaseItem};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, VectorDatabaseItem)]
#[serde(rename_all = "camelCase")]
struct Position {
    #[lvv(description, rename = "role")]
    job_title: String,
    #[lvv(description)]
    organization: Option<String>,
    started_on: String,
    #[lvv(skip)]
    internal_id: u64,
}

#[derive(VectorDatabase)]
struct Portfolio {
    positions: Vec<Position>,
    #[lvv(skip)]
    owner: String,
}

let drafts = portfolio.point_drafts()?;
// drafts[0].category    == "Position"
// drafts[0].description == "Researcher\nUNLP"
// drafts[0].payload     == {"role": "Researcher", "organization": "UNLP",
//                           "startedOn": "2020-03-01", "category": "Position"}
```

## Example

lvv's [`examples/derive.rs`](https://github.com/egonik-unlp/lvv/blob/main/examples/derive.rs)
is a complete lvv pipeline over structs that use these derives. It reads
records from JSON Lines, CSV and JSON files, turns them into points, embeds the
descriptions with Ollama using a cache, and runs a job queue that writes one
Qdrant collection per category. From a clone of
[lvv](https://github.com/egonik-unlp/lvv):

```sh
cargo run --example derive --features derive                  # load and print the points
cargo run --example derive --features derive -- --embed       # embed and build the jobs
cargo run --example derive --features derive -- --embed --qdrant http://localhost:6334
```

## Attributes

On `VectorDatabaseItem` fields:

| Attribute                | Effect |
|--------------------------|--------|
| `#[lvv(description)]`    | Adds the field to the embedded description. |
| `#[lvv(skip)]`           | Leaves the field out of the payload and the description. |
| `#[lvv(rename = "key")]` | Stores the field under `key` in the payload. |

On `VectorDatabase` fields, `#[lvv(skip)]` ignores the field. Every other field
must hold `VectorDatabaseItem`s as `T`, `Vec<T>`, `BTreeMap<K, T>` or
`HashMap<K, T>`, each optionally wrapped in `Option`.

See the [API documentation](https://docs.rs/lvv-macros) for the full rules.

## License

Licensed under the Apache License, Version 2.0.

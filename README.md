# Product data broker

## Introduction

There are immense amounts of data available online, each source accessed in its own way and operating on its own data format. The aim of the broker is to unify many such sources under one roof by allowing the user to register them and access the data through a single interface. The intent is to allow *queries* on the data, e.g. filtering or sorting by some field: the broker translates these queries into whatever formats the backing sources support and merge the results.

This also allows for a layer of validation: messy or untrusted data can be made to pass any number of transformations and checks, such as parsing strings into more machine-readable and invariant-enforced formats, or verifying known values. For data where one or more unique identifiers exist, such as a DOI or ISBN, missing fields can be completed by merging two partial entries corresponding to the same object but from two different sources.

Some entities allow submitting resources. The broker can also act as a publisher, distributing data to any of its contacts that would accept it.

## Structural overview

The centerpiece of the project is the broker itself. The broker allows registering *sources*: entities that can provide data, and *sinks*: entities that accept data. A sink could possibly be made to accept requests to delete data as well. A publicly available archive, an orgnization's annual reports or a subscribable stream might act as a source; a RESTful API or a database might act as both a source and a sink. The broker communicates with these entities through *connectors*. Connectors are designed to be modular and reusable, for example we can have one interface for a connector to use for any RESTful API, one for any locally-stored archive, one for an SQL-based relational database, etc.

The broker should support queries when requesting data. These queries should, where possible, be translated to the external entity's own format such that the work is delegated to use more efficient, domain-specific data structures and minimize the data transmitted. Where this is impossible, such as for a local storage, the broker should be prepared to perform these operations itself.

As another way of minimizing work and traffic, the broker should allow *constraints* to be put on connectors, indicating that all data they would provide/accept would/must have certain properties or uphold certain invariants.

Connectors often require *encoders* and *decoders* to map the data format used by the broker to the one used by the entity either internally or simply during transactions (and vice versa). This means we consider a RESTful API serving JSON and one serving XML different things, but allow them to both use the generic REST connector, simply plugging in different components. Support should also exist for API keys or other data that must be incldued with requests, but are not part of the queries.

## Example: book database

The broker is oblivious to the kind of data it is working with or any semantic meaning behind it. As a concrete proof of concept, an implementation of a book database is created. Queries would include, for example, filtering books by release year ("before 1900", "between 2010 and 2020"...), by author ("Jane Austen", "George Orwell"...), by ISBN (9781853260087 uniquely identifying *Moby-Dick, or, The whale* published by Wordsworth Classics on 2002), and so on.

Books are an interesting topic as there are especially huge amounts of data from countless sources, but with only some of it standardized (e.g. title, ISBN, number of pages are reasonably provided by any source, while each may define its own genres for categorizing books). They also allow practical demonstration of the ability to merge partial sources, since books can be globally uniquely identified by either ISBN or some other key such as the combination of title, publisher and release year. Book data is also publicly available online for free, allowing integrations with real-world sources like [Library Genesis](https://libgen.li/) or [Project Gutenberg](https://www.gutenberg.org/). For testing, a number of mock companies acting as sources or sinks have also been created. These are all found in the `mock` directory in the project root.

[Schema.org](https://schema.org/) is a collaborative project by founders Google, Microsoft, Yahoo and Yandex, as well as participation by the general web community. It is a service providing schemas for many types of data, here specifically [books](https://schema.org/Book). A library is created to dynamically generate types from these schemas at compile time, furthering modularity. Thus, with this book service implemented, it could easily be modified to serve as a database for [cars](https://schema.org/Car) or [infectious diseases](https://schema.org/InfectiousDisease).

## Documentation

See [installation](#Installation) for instructions on how to install required tools.

To build documentation, run `cargo doc`. This command will create files in the `target/doc` directory in the project root. Some useful flags:
- `--package` along with the package path to build documentation only for that package.
- `--no-deps` is useful to (drastically) speed up build times, but leaves broken or missing links when referring to symbols from external crates
- `--document-private-items` should be used to include documentation of internals, useful for developers.
- `--open` to open the index page directly in a default browser.

Suggested setup:
```sh
cargo doc --no-deps --open
```

For developers:
```sh
cargo doc --document-private-items --open
```

## Installation

All software components are written in Rust. Begin by installing `rustup`, the Rust update tool, [for your operating system](https://rust-lang.org/tools/install/). The official package manager `Cargo` comes installed by default. This project uses the nightly toolchain. Install it using `rustup`:

```sh
rustup toolchain install nightly
```

Run a package using `cargo run --release`. Use the `--package` flag and provide the package path to run the specified package (`broker` is the default package). The `--release` flag indicates release (optimized) mode, which means long build times. Omit it to build in debug mode.

See [The Cargo Book](https://doc.rust-lang.org/cargo/) for more commands.

## Developer information

The main branch is protected and direct pushes are disallowed. Any attempt to write to the main branch must be accompanied by a pull request to be approved by at least one other person. A linear history is to be maintained on the main branch, through either rebases or squashes when merging.

The project uses a strict CI run on each push. Any pull request failing CI checks will be denied. These checks include:
- Verifying that the code is formatted (as specified by `rustfmt`, configured in `rustfmt.toml).
- Linting (using both `rustc` and `cargo clippy`, configured in `Cargo.toml`), treating all warnings as errors. Lints include, but are not limited to, code style, naming conventions, presence of documentation and detection of suspicious patterns.
- Enforcing documentation (using `cargo doc`), including detection of broken links, missing "Errors" or "Panics" sections where applicable, and verifying that code snippets compile.
- Testing (using `cargo test`). Current tests include unit tests; automatic integration tests may be considered in the future.
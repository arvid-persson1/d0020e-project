//! Procedural macro support for the query system.
//!
//! This crate provides the `#[derive(Queryable)]` macro, which generates
//! type-safe [`Field`](broker::query::Field) accessors for each named field
//! in a struct. These accessors are used to construct queries in a fluent,
//! compile-time checked way.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

/// Derive macro enabling query construction on a struct.
///
/// For each named field in the struct, this macro generates an inherent
/// method returning a [`Field`](broker::query::Field) describing how to
/// access that field.
///
/// # Example
///
/// ```ignore
/// #[derive(Queryable)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// // Generated:
/// // impl User {
/// //     pub fn id() -> Field<User, u32> { ... }
/// //     pub fn name() -> Field<User, String> { ... }
/// // }
/// ```
///
/// # Panics
///
/// This macro panics at compile time if:
/// - The input type is not a struct
/// - The struct does not have named fields
///
/// These constraints are required to generate valid field accessors.
#[proc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let Data::Struct(data) = input.data else {
        panic!("Queryable can only be derived for struct types");
    };

    let field_methods = data.fields.iter().map(|field| {
        let ident = field
            .ident
            .as_ref()
            .expect("Queryable can only be derived for structs with named fields");
        let name = ident.to_string();

        let ty = &field.ty;

        quote! {
            /// Returns a queryable handle to this field.
            pub fn #ident() -> ::broker::query::Field<#struct_name, #ty> {
                ::broker::query::Field::new(
                    #name,
                    |s: &#struct_name| -> &#ty { &s.#ident }
                )
            }
        }
    });

    quote! {
        impl #struct_name {
            #(#field_methods)*
        }
    }
    .into()
}

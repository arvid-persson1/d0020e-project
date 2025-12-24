//! Procedural macro crate for use by [`broker::query`].

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Type, parse_macro_input};

#[proc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let Data::Struct(struct_) = input.data else {
        panic!("Queryable can only be derived for structure types.");
    };

    let field_methods = struct_.fields.iter().map(|field| {
        // PERF: It's not actually necessary to perform a proper check of each field, since the
        // presence of one named field means it's a "classic" `struct` and all fields will be
        // named.
        let Some(field_name) = field.ident.as_ref().map(ToString::to_string) else {
            panic!("Queryable can not be derived for tuple structs, as field names are needed.");
        };

        // HACK: This does not handle qualified paths such as `std::string::String`, nor does
        // it handle type aliases, as it works directly on the identifier as a string. Is it
        // possible to handle this, and should the latter be handled at all? Using a type alias
        // instead of `String` literally could be seen as an opt-out.
        let getter = if let Type::Path(path) = &field.ty
            && let Some(ident) = path.path.get_ident()
            && *ident == "String"
        {
            quote! {
                |data| data.#field_name.as_str()
            }
        } else {
            quote! {
                |data| &data.#field_name
            }
        };

        quote! {
            pub fn #field_name() -> broker::query::Field<#struct_name, _, #field_name> {
                broker::query::Field::new(#getter)
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

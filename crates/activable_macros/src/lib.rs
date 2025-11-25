//! Derive macro for `activable::Activable`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Implements `activable::Activable` and adds inherent helpers:
/// - `T::activate_system()`
/// - `T::deactivate_system()`
#[proc_macro_derive(Activable)]
pub fn derive_activable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;
    let (ig, tg, wc) = input.generics.split_for_impl();

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #ig ::activable::sealed::Sealed for #ty #tg #wc {}
        #[automatically_derived]
        impl #ig ::activable::Activable for #ty #tg #wc {}

        impl #ig #ty #tg #wc {
            /// @system Activates this type when run.
            pub fn activate_system() -> fn(::activable::Commands) {
                ::activable::activate_system::<Self>
            }
            /// @system Deactivates this type when run.
            pub fn deactivate_system() -> fn(::activable::Commands) {
                ::activable::deactivate_system::<Self>
            }
        }
    })
}

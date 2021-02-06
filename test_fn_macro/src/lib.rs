use proc_macro::{
    TokenStream,
};
use proc_macro2::{
    TokenStream as TokenStream2,
};
use quote::quote;

#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    #[cfg(feature = "tokio_executor")]
    let attr = quote!{#[tokio::test]};
    #[cfg(not(feature = "tokio_executor"))]
    let attr = quote!{#[::core::prelude::v1::test]};
    #[cfg(feature = "tokio_executor")]
    let qual = quote!{async};
    #[cfg(not(feature = "tokio_executor"))]
    let qual = quote!{};
    TokenStream::from(quote! {
        #attr
        #qual #input
    })
}

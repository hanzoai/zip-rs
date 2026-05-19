//! Proc macros for zip-rs.
//!
//! `#[handler]` turns an idiomatic Rust function into a HIP-0105-compatible
//! wasm export. The user writes:
//!
//! ```ignore
//! #[handler]
//! fn validate(req: ValidateReq) -> zip_rs::Result<ValidateResp> { ... }
//! ```
//!
//! and gets, in addition to their original function:
//!
//! ```ignore
//! #[no_mangle]
//! pub extern "C" fn validate(ptr: i32, len: i32) -> i64 { /* ... */ }
//! ```
//!
//! Allocator exports (`__base_alloc` / `__base_free`) live in the
//! `zip-rs` crate itself with `#[no_mangle]` — there's no per-crate
//! macro needed for them.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, FnArg, Ident, ItemFn, ReturnType};

/// Mark a function as a zip-rs wasm handler. See crate docs for the
/// expansion shape.
#[proc_macro_attribute]
pub fn handler(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    expand_handler(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_handler(mut input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    // Reject shapes we can't faithfully translate to a wasm export.
    if input.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            &input.sig,
            "zip-rs: handlers cannot be async (wasm host calls are synchronous)",
        ));
    }
    if !input.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.sig.generics,
            "zip-rs: handlers cannot be generic — instantiate a concrete type",
        ));
    }
    if input.sig.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            &input.sig,
            "zip-rs: handlers cannot be unsafe",
        ));
    }
    if input.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &input.sig.inputs,
            "zip-rs: handlers must take exactly one argument (the deserialized request)",
        ));
    }
    if matches!(input.sig.output, ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            &input.sig,
            "zip-rs: handlers must return zip_rs::Result<T> for a Serialize T",
        ));
    }

    let export_name = input.sig.ident.clone();
    let inner_name = Ident::new(
        &format!("__zip_rs_inner_{}", export_name),
        Span::call_site(),
    );

    let (arg_pat, arg_ty) = match input.sig.inputs.first().unwrap() {
        FnArg::Typed(pt) => (pt.pat.clone(), pt.ty.clone()),
        FnArg::Receiver(_) => {
            return Err(syn::Error::new_spanned(
                &input.sig.inputs,
                "zip-rs: handlers cannot take a `self` receiver",
            ));
        }
    };

    // Rewrite the inner function: rename and force private. There is
    // exactly one way to invoke a handler — via the wasm export.
    input.sig.ident = inner_name.clone();
    input.vis = syn::Visibility::Inherited;

    let expanded = quote! {
        #input

        #[no_mangle]
        pub extern "C" fn #export_name(__zip_rs_ptr: i32, __zip_rs_len: i32) -> i64 {
            // SAFETY: the wasm host writes `__zip_rs_len` bytes of JSON
            // at `__zip_rs_ptr` in our linear memory before invoking
            // this export. That is the HIP-0105 contract.
            let #arg_pat: #arg_ty = match unsafe {
                ::zip_rs::__private::read_payload::<#arg_ty>(__zip_rs_ptr, __zip_rs_len)
            } {
                ::core::result::Result::Ok(__zip_rs_v) => __zip_rs_v,
                ::core::result::Result::Err(__zip_rs_e) => {
                    let __zip_rs_bytes = ::zip_rs::__private::write_error(&__zip_rs_e);
                    let (__zip_rs_p, __zip_rs_l) = ::zip_rs::__private::leak_box(__zip_rs_bytes);
                    return ::zip_rs::__private::pack(__zip_rs_p, __zip_rs_l);
                }
            };

            match #inner_name(#arg_pat) {
                ::core::result::Result::Ok(__zip_rs_v) => {
                    let __zip_rs_bytes = ::zip_rs::__private::write_value(&__zip_rs_v);
                    let (__zip_rs_p, __zip_rs_l) = ::zip_rs::__private::leak_box(__zip_rs_bytes);
                    ::zip_rs::__private::pack(__zip_rs_p, __zip_rs_l)
                }
                ::core::result::Result::Err(__zip_rs_e) => {
                    let __zip_rs_err: ::zip_rs::Error = __zip_rs_e.into();
                    let __zip_rs_bytes = ::zip_rs::__private::write_error(&__zip_rs_err);
                    let (__zip_rs_p, __zip_rs_l) = ::zip_rs::__private::leak_box(__zip_rs_bytes);
                    ::zip_rs::__private::pack(__zip_rs_p, __zip_rs_l)
                }
            }
        }
    };

    Ok(expanded)
}

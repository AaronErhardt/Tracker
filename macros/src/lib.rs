//! Macros for the `tracker` crate.

#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    clippy::cargo,
    clippy::must_use_candidate,
    clippy::cargo
)]

use proc_macro::{self, Span, TokenStream};
use proc_macro2::{Span as Span2, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_macro_input, Attribute, Error, Field, Fields, GenericParam, Ident, ItemStruct, Type,
};

const NO_EQ: &str = "no_eq";
const DO_NOT_TRACK: &str = "do_not_track";

/// Implements tracker methods for structs.
#[proc_macro_attribute]
pub fn track(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return Error::new(
            attr.into_iter().next().unwrap().span().into(),
            "This macro doesn't handle attributes",
        )
        .into_compile_error()
        .into();
    }

    let mut data: ItemStruct = parse_macro_input!(item);
    let ident = data.ident.clone();
    let tracker_ty;
    let struct_vis = &data.vis;
    let where_clause = &data.generics.where_clause;

    // Remove default type parameters (like <Type=DefaultType>).
    let mut generics = data.generics.clone();
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(ty) = param {
            ty.eq_token = None;
            ty.default = None;
        }
    }

    let mut generics_iter = data.generics.params.iter();
    let mut generic_idents = TokenStream2::new();

    if let Some(first) = generics_iter.next() {
        impl_struct_generics(first, &mut generic_idents);
        for generic_param in generics_iter {
            generic_idents.extend(quote! {,});
            impl_struct_generics(generic_param, &mut generic_idents);
        }
    }

    let mut field_list = Vec::new();
    if let Fields::Named(named_fields) = &mut data.fields {
        for field in &mut named_fields.named {
            let (do_not_track, no_eq) = parse_field_attrs(&mut field.attrs);
            if !do_not_track {
                let ident = field.ident.clone().expect("Field has no identifier");
                let ty: Type = field.ty.clone();
                field_list.push((ident, ty, no_eq, field.vis.clone()));
            }
        }

        tracker_ty = tracker_type(field_list.len());
        let change_field = Field {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(Ident::new("tracker", Span::call_site().into())),
            colon_token: None,
            ty: Type::Verbatim(tracker_ty.clone()),
        };

        named_fields.named.push(change_field);
    } else {
        panic!("No named fields");
    }

    let mut output = data.to_token_stream();

    let mut methods = proc_macro2::TokenStream::new();
    for (num, (id, ty, no_eq, vis)) in field_list.iter().enumerate() {
        let id_span: Span2 = id.span().unwrap().into();

        let get_id = Ident::new(&format!("get_{}", id), id_span);
        let get_mut_id = Ident::new(&format!("get_mut_{}", id), id_span);
        let update_id = Ident::new(&format!("update_{}", id), id_span);
        let changed_id = Ident::new(&format!("changed_{}", id), id_span);
        let set_id = Ident::new(&format!("set_{}", id), id_span);

        let get_doc = format!("Get an immutable reference to the {id} field.");
        let get_mut_doc =
            format!("Get a mutable reference to the {id} field and mark the field as changed.");
        let update_doc =
            format!("Use a closure to update the {id} field and mark the field as changed.");
        let changed_doc =
            format!("Check if value of {id} field has changed.");
        let bit_mask_doc = format!("Get a bit mask to look for changes on the {id} field.");

        methods.extend(quote_spanned! { id_span =>
            #[allow(dead_code, non_snake_case)]
            #[must_use]
            #[doc = #get_doc]
            #vis fn #get_id(&self) -> &#ty {
                &self.#id
            }

            #[allow(dead_code, non_snake_case)]
            #[must_use]
            #[doc = #get_mut_doc]
            #vis fn #get_mut_id(&mut self) -> &mut #ty {
                self.tracker |= Self::#id();
                &mut self.#id
            }

            #[allow(dead_code, non_snake_case)]
            #[doc = #update_doc]
            #vis fn #update_id<F: FnOnce(&mut #ty)>(&mut self, f: F) {
                self.tracker |= Self::#id();
                f(&mut self.#id);
            }

            #[allow(dead_code, non_snake_case)]
            #[doc = #changed_doc]
            #vis fn #changed_id(&self) -> bool {
                self.changed(Self::#id())
            }

            #[allow(dead_code, non_snake_case)]
            #[must_use]
            #[doc = #bit_mask_doc]
            #vis fn #id() -> #tracker_ty {
                1 << #num
            }
        });

        if *no_eq {
            let set_doc = format!("Set the value of field {id} and mark the field as changed.");
            methods.extend(quote_spanned! { id_span =>
                #[allow(dead_code, non_snake_case)]
                #[doc = #set_doc]
                #vis fn #set_id(&mut self, value: #ty) {
                    self.tracker |= Self::#id();
                    self.#id = value;
                }
            });
        } else {
            let set_doc = format!("Set the value of field {id} and mark the field as changed if it's not equal to the previous value.");
            methods.extend(quote_spanned! { id_span =>
                #[allow(dead_code, non_snake_case)]
                #[doc = #set_doc]
                #vis fn #set_id(&mut self, value: #ty) {
                    if self.#id != value {
                        self.tracker |= Self::#id();
                    }
                    self.#id = value;
                }
            });
        }
    }

    output.extend(quote_spanned! { ident.span() =>
        impl #generics #ident < #generic_idents > #where_clause {
            #methods
            #[allow(dead_code)]
            #[must_use]
            /// Get a bit mask to look for changes on all fields.
            #struct_vis fn track_all() -> #tracker_ty {
                #tracker_ty::MAX
            }

            #[allow(dead_code)]
            /// Mark all fields of the struct as changed.
            #struct_vis fn mark_all_changed(&mut self) {
                self.tracker = #tracker_ty::MAX;
            }

            /// Check for changes made to this struct with a given bitmask.
            ///
            /// To receive the bitmask, simply call `Type::#field_name()`
            /// or `Type::#track_all()`.
            #[warn(dead_code)]
            #[must_use]
            #struct_vis fn changed(&self, mask: #tracker_ty) -> bool {
                self.tracker & mask != 0
            }

            /// Check for any changes made to this struct.
            #[allow(dead_code)]
            #[must_use]
            #struct_vis fn changed_any(&self) -> bool {
                self.tracker != 0
            }

            /// Resets the tracker value of this struct to mark all fields
            /// as unchanged again.
            #[warn(dead_code)]
            #struct_vis fn reset(&mut self) {
                self.tracker = 0;
            }
        }
    });

    output.into()
}

fn impl_struct_generics(param: &GenericParam, stream: &mut TokenStream2) {
    match param {
        GenericParam::Type(ty) => ty.ident.to_tokens(stream),
        GenericParam::Const(cnst) => cnst.to_tokens(stream),
        GenericParam::Lifetime(lifetime) => lifetime.to_tokens(stream),
    }
}

/// Look for no_eq and do_not_track attributes and remove
/// them from the tokens.
fn parse_field_attrs(attrs: &mut Vec<Attribute>) -> (bool, bool) {
    let mut do_not_track = false;
    let mut no_eq = false;
    let attrs_clone = attrs.clone();

    for (index, attr) in attrs_clone.iter().enumerate() {
        let segs = &attr.path().segments;
        match segs.len() {
            1 => {
                let first = &segs.first().unwrap().ident;
                if first == NO_EQ {
                    attrs.remove(index);
                    no_eq = true;
                } else if first == DO_NOT_TRACK {
                    attrs.remove(index);
                    do_not_track = true;
                }
            }
            2 => {
                let mut iter = segs.iter();
                let first = &iter.next().unwrap().ident;
                if first == "tracker" {
                    let second = &iter.next().unwrap().ident;
                    if second == NO_EQ {
                        attrs.remove(index);
                        no_eq = true;
                    } else if second == DO_NOT_TRACK {
                        attrs.remove(index);
                        do_not_track = true;
                    }
                }
            }
            _ => {}
        }
    }

    (do_not_track, no_eq)
}

fn tracker_type(len: usize) -> proc_macro2::TokenStream {
    match len {
        0..=8 => {
            quote! {u8}
        }
        9..=16 => {
            quote! {u16}
        }
        17..=32 => {
            quote! {u32}
        }
        33..=64 => {
            quote! {u64}
        }
        65..=128 => {
            quote! {u128}
        }
        _ => {
            panic!("You can only track up to 128 values")
        }
    }
}

use proc_macro::{self, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Field, Fields, Ident, ItemStruct, Type};

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
            panic!("Can only track up to 128 values")
        }
    }
}

#[proc_macro_attribute]
pub fn tracker(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut data: ItemStruct = parse_macro_input!(item);
    let ident = data.ident.clone();
    let tracker_ty;

    let mut field_list = Vec::new();
    if let Fields::Named(named_fields) = &mut data.fields {
        for field in &named_fields.named {
            let ident = field.ident.clone().expect("Field has no identifier");
            let ty: Type = field.ty.clone();
            field_list.push((ident, ty));
        }

        tracker_ty = tracker_type(field_list.len());
        let change_field = Field {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
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
    for (num, (id, ty)) in field_list.iter().enumerate() {
        let get_id = Ident::new(&format!("get_{}", id), Span::call_site().into());
        let update_id = Ident::new(&format!("update_{}", id), Span::call_site().into());
        let set_id = Ident::new(&format!("set_{}", id), Span::call_site().into());

        methods.extend(quote! {
            pub fn #get_id(&self) -> &#ty {
                &self.#id
            }

            pub fn #update_id<F: Fn(&mut #ty)>(&mut self, f: F)  {
                self.tracker |= Self::#id();
                f(&mut self.#id);
            }

            pub fn #set_id(&mut self, value: #ty) {
                self.tracker |= Self::#id();
                self.#id = value;
            }

            const fn #id() -> #tracker_ty {
                1 << #num
            }
        });
    }

    output.extend(quote! {
    impl #ident {
        #methods
        const fn all() -> #tracker_ty {
            #tracker_ty::MAX
        }

            fn changed(&self, mask: #tracker_ty) -> bool {
            self.tracker & mask != 0
        }
    }

    impl ::struct_tracker::Tracker for #ident {
        fn reset(&mut self) {
            self.tracker = 0;
        }
    }
    });

    output.into()
}

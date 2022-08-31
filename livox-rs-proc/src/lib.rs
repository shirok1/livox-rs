use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use quote::__private::TokenStream as Quote;
use syn::{braced, FieldsNamed, Ident, parse_macro_input, Token, DeriveInput};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

struct CmdAckGenerate {
    name: Ident,
    enum_structs: Punctuated<(Ident, FieldsNamed, FieldsNamed), Token![,]>,
}

impl Parse for CmdAckGenerate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![enum]>()?;
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);
        Ok(CmdAckGenerate {
            name,
            enum_structs: content.parse_terminated(|s| {
                let enum_ident: Ident = s.parse()?;
                // if s.peek(Token![pub]) {
                //     s.parse::<Token![pub]>()?;
                // }
                let cmd_struct: FieldsNamed = s.parse()?;
                let ack_struct: FieldsNamed = s.parse()?;
                Ok((enum_ident, cmd_struct, ack_struct))
            })?,
        })
    }
}


// fn generate_enum_writer(name: &Ident, enum_names:&[&Ident],enum_struct: &[&FieldsNamed])->Quote{
//
// }


fn generate_struct_writer(no_and_fields: (&&FieldsNamed, u8)) -> Quote {
    let (fields, no) = no_and_fields;
    let ident = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let ty = fields.named.iter().map(|f| &f.ty).collect::<Vec<_>>();
    quote! {
        bytes[0] = #no;
        let mut cur: usize = 1;
        #({
            let len = <#ty>::BYTE_LEN;

            #ident.write_bytes_default_le(&mut bytes[cur .. (cur + len)]);
            cur += len;
        })*
    }
}

fn generate_struct_reader(field_and_name: (&&FieldsNamed, &&Ident)) -> Quote {
    let (fields, name) = field_and_name;
    let ident = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let ty = fields.named.iter().map(|f| &f.ty).collect::<Vec<_>>();
    quote! {
        let mut cur: usize = 1;
        #(
            let len = <#ty>::BYTE_LEN;
            let #ident = <#ty>::read_bytes_default_le(&bytes[cur .. (cur + len)]);
            cur += len;
        )*
        Self::#name { #(#ident),* }
    }
}

fn generate_one_kind(name: &Ident, enum_name: &[&Ident], enum_struct: &[&FieldsNamed]) -> Quote {
    let numbers = 0..enum_name.len() as u8;
    let writers = enum_struct.iter().zip(numbers.clone()).map(generate_struct_writer);
    let readers = enum_struct.iter().zip(enum_name).map(generate_struct_reader);
    let field_names = enum_struct.iter()
        .map(|f| {
            let names = f.named.iter().map(|f| f.ident.as_ref().unwrap()).collect::<Vec<_>>();
            quote! {#(#names),*}
        });
    let field_sizes = enum_struct.iter()
        .map(|f| (f.named.iter().map(|f| &f.ty), f.named.is_empty()))
        .filter_map(|(f, empty)| (!empty).then_some(quote!((#(<#f>::BYTE_LEN)+*))))
        // .collect::<Vec<_>>();
        .fold(quote! {0}, |acc, expr| {
            // let current = quote!((#(<#ty>::BYTE_LEN)+*));
            // quote! { (#current > #acc as usize) * #current + (#current <= #acc as usize) * #acc }
            // quote! { [#expr, #acc][(#expr < #acc) as usize] }
            quote! { const_max(#expr, #acc) }
        });

    quote! {
        #[derive(PartialEq, Debug)]
        pub enum #name {
            #(#enum_name #enum_struct,)*
        }
        impl #name {
            pub const MAX_BYTE_LEN: usize = #field_sizes + 1;
            // pub const MAX_BYTE_LEN: usize = *[#(#field_sizes),*].iter().max().unwrap();
            pub fn write_bytes(&self, bytes: &mut [u8]) {
                match self {
                    #(Self::#enum_name {#field_names} => {
                        #writers
                    },)*
                }
            }
            pub fn read_bytes(bytes: &[u8]) -> Self {
                match bytes[0] {
                    #(#numbers => {#readers})*
                    _ => panic!("unknown enum id"),
                }
            }
        }
    }
}

#[proc_macro]
pub fn generate_full(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as CmdAckGenerate);
    let cmd_name = format_ident!("Cmd{}", &ast.name);
    let ack_name = format_ident!("Ack{}", &ast.name);

    let enum_name = ast.enum_structs.iter().map(|(i, _, _)| i).collect::<Vec<_>>();
    let cmd_enums = ast.enum_structs.iter().map(|(_, cmd, _)| cmd).collect::<Vec<_>>();
    let ack_enums = ast.enum_structs.iter().map(|(_, _, ack)| ack).collect::<Vec<_>>();

    let cmd_enum_def = generate_one_kind(&cmd_name, &enum_name, &cmd_enums);
    let ack_enum_def = generate_one_kind(&ack_name, &enum_name, &ack_enums);

    (quote! {
        #cmd_enum_def
        #ack_enum_def



        // impl ByteStruct for #cmd_name {
        //     fn write_bytes(&self, bytes: &mut [u8]) {
        //         let mut cur: usize = 0;
        //         #({
        //             let len = <#ty1>::BYTE_LEN;
        //             self.#ident1.#write_bytes_fn(&mut bytes[cur .. (cur + len)]);
        //             cur += len;
        //         })*
        //     }
        //     fn read_bytes(bytes: &[u8]) -> Self {
        //         let mut cur: usize = 0;
        //         #(
        //             let len = <#ty2>::BYTE_LEN;
        //             let #ident2 = <#ty3>::#read_bytes_fn(&bytes[cur .. (cur + len)]);
        //             cur += len;
        //         )*
        //         #name { #(#ident3),* }
        //     }
        // }
        //
        // impl ByteStructLen for #name {
        //     const BYTE_LEN: usize = #(<#ty0>::BYTE_LEN)+*;
        // }
    }).into()
}

#[proc_macro_derive(Request)]
pub fn derive_request_fn(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident.to_token_stream();
    // let respoonse_name = format_ident!("{}Response", &ast.ident);
    // let vis = ast.vis.to_token_stream();
    // ast.da
    (quote! {
        // impl Request for #name {
        //     type Response = #respoonse_name
        // }
        // impl Response for #respoonse_name {}
        // #vis struct #respoonse_name
        impl Request for #name { type Response = super::response::#name; }
    }).into()
}
#[proc_macro_derive(Response)]
pub fn derive_response_fn(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident.to_token_stream();

    // ast.da
    (quote! {
        // #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        // #[deku(endian = "little")]
        // #ast
        impl Response for #name {}
    }).into()
}
#[proc_macro_derive(Message)]
pub fn derive_message_fn(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident.to_token_stream();
    (quote! {
        impl Message for #name {}
    }).into()
}

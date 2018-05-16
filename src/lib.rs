//!

#![feature(proc_macro, proc_macro_lib)]
#![allow(unused_imports, unused_variables)]

extern crate proc_macro;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::ToTokens;
use std::collections::HashSet as Set;
use syn::fold::{self, Fold};
use syn::punctuated::Punctuated;
use syn::synom::Synom;
use syn::LitStr;
use syn::{
    Expr, FnArg, GenericArgument, Ident, ImplItem, ImplItemMethod, Item, ItemImpl, ItemStatic, Pat,
    PathArguments, Stmt, Type,
};

/// Main macro that implements automated clap generation.
///
/// Tag an `impl` block with this attribute of a type. Then
/// call `start()` on the type to handle match parsing.
#[proc_macro_attribute]
pub fn thunderclap(_args: TokenStream, input: TokenStream) -> TokenStream {
    let i: ItemImpl = match syn::parse(input.clone()) {
        Ok(input) => input,
        Err(e) => panic!("Error: '{}'", e),
    };

    let (name, app_token) = match *i.self_ty {
        Type::Path(ref p) => {
            let meh = p.path.segments[0].ident;
            (format!("{}", p.path.segments[0].ident), quote!( #meh ))
        }
        _ => (format!("Unknown App"), quote!()),
    };

    let about = match i.attrs.first() {
        Some(a) => String::from(
            format!("{}", a.tts)
                        /* Clean the tokens TODO: Make this not suck */
                        .replace("/", "")
                        .replace("\\", "")
                        .replace("\"", "")
                        .replace("=", "").trim(),
        ),
        _ => String::new(),
    };

    let mut matches: Vec<quote::Tokens> = Vec::new();
    let orignal = quote!(#i);
    let mut app = quote! {
        App::new(#name).about(#about).setting(AppSettings::SubcommandRequired)
    };

    for item in &i.items {
        match item {
            &ImplItem::Method(ref i) => {
                let name = LitStr::new(&i.sig.ident.to_string(), i.sig.ident.span);
                let func_id = &i.sig.ident;
                let about = match i.attrs.first() {
                    Some(a) => String::from(
                        format!("{}", a.tts)
                        /* Clean the tokens TODO: Make this not suck */
                        .replace("/", "")
                        .replace("\\", "")
                        .replace("\"", "")
                        .replace("=", "").trim(),
                    ),
                    _ => String::new(),
                };

                let mut arguments = quote!();

                let mut index: usize = 0;
                let args = i.sig
                    .decl
                    .inputs
                    .iter()
                    .fold(quote!{}, |acc, arg| match arg {
                        &FnArg::Captured(ref arg) => match &arg.pat {
                            &Pat::Ident(ref i) => {
                                let name = format!("{}", i.ident);
                                let optional = match arg.ty {
                                    Type::Path(ref p) => match p.path.segments.first() {
                                        Some(ps) => match &ps.value().ident.to_string().as_str() {
                                            &"Option" => true,
                                            _ => false,
                                        },
                                        _ => false,
                                    },
                                    _ => false,
                                };

                                let mmm = if let Some(typed) = match arg.ty {
                                    Type::Path(ref p) => match p.path.segments.first() {
                                        Some(ps) => match optional {
                                            false => Some(arg.ty.clone()),
                                            true => match ps.value().arguments {
                                                PathArguments::AngleBracketed(ref b) => {
                                                    match b.args.first() {
                                                        Some(pair) => match pair.value() {
                                                            GenericArgument::Type(Type::Path(
                                                                pp,
                                                            )) => Some(Type::from(pp.clone())),
                                                            _ => None,
                                                        },
                                                        _ => None,
                                                    }
                                                }
                                                _ => None,
                                            },
                                        },
                                        _ => None,
                                    },
                                    _ => None,
                                } {
                                    if optional {
                                        quote! {
                                            match m.value_of(#name) {
                                                Some(m) => Some(m.parse::<#typed>().unwrap()),
                                                None => None
                                            }
                                        }
                                    } else {
                                        quote! { m.value_of(#name).unwrap().parse::<#typed>().unwrap() }
                                    }
                                } else {
                                    if optional {
                                        quote! { m.value_of(#name) }
                                    } else {
                                        quote! { m.value_of(#name).unwrap() }
                                    }
                                };

                                index += 1;
                                if optional {
                                    arguments = quote! {
                                        #arguments
                                        #mmm
                                    };
                                    quote! { #acc.arg(Arg::with_name(#name)) }
                                } else {
                                    arguments = quote! {
                                        #arguments
                                        #mmm,
                                    };
                                    quote! { #acc.arg(Arg::with_name(#name).required(true)) }
                                }
                            }
                            _ => quote!{ #acc },
                        },
                        _ => quote!{ #acc },
                    });

                app = quote! {
                    #app.subcommand(
                        SubCommand::with_name(#name).about(#about)#args
                    )
                };

                matches.push(quote! { (#name, Some(m)) => #app_token :: #func_id ( #arguments ), });
            }
            _ => {}
        }
    }

    // let mut matchy = quote!{ match args.subcommand() { };
    let mut matchy = quote!{};

    for m in &matches {
        matchy = quote! {
            #matchy
            #m
        };
    }

    matchy = quote! {
        match args.subcommand() {
            #matchy
            _ => { /* We drop errors for now... */ },
        }
    };

    let tokens = quote! {
        #orignal

        /// This block was generated by thunder v0.0.0
        #[allow(unused)]
        impl #app_token {

            /// Starts the CLI parsing and calls whichever function handles the input
            fn start() {
                use clap::{App, SubCommand, Arg, AppSettings};

                let app = #app;
                let args = app.get_matches();
                #matchy
            }
        }
    };

    tokens.into()
}

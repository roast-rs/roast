extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate walkdir;
#[macro_use]
extern crate failure;
extern crate inflector;
extern crate itertools;

mod entity;

use entity::{DerivedEntity, DerivedFn, DerivedFnArg};
use inflector::Inflector;
use proc_macro::TokenStream;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use syn::{parse_file, DeriveInput, FnArg, ImplItem, Item, Pat, ReturnType, Type, Visibility};
use quote::ToTokens;
use walkdir::WalkDir;

#[proc_macro_derive(RoastExport)]
pub fn roast_export(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    let identifier_name = format!("{}", input.ident).to_pascal_case();

    let methods = methods_for_ident(&identifier_name);
    let entity = DerivedEntity::new(&identifier_name, methods);
    let token_stream = entity.export_jni_ffi_tokens();
    write_java_class(&entity);
    //panic!("{}", token_stream);
    token_stream.into()
}

/// Extracts a list of methods for a given identifier.
///
/// This function is hacky, because we don't have stable support
/// for custom attributes right now. We load all files from the
/// project and try to match up the struct name with its impl
/// methods. This is error prone and limited, but will work for
/// now. As soon as we get custom attributes we should switch over
/// to that since its much better suited for this task.
fn methods_for_ident(ident: &str) -> Vec<DerivedFn> {
    let rootdir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut methods = vec![];
    for entry in WalkDir::new(rootdir) {
        let e = entry.expect("could not decode entry");
        if e.file_name().to_str().unwrap().ends_with("rs") {
            let mut file = File::open(&e.path()).expect("Unable to open file");
            let mut src = String::new();
            file.read_to_string(&mut src).expect("Unable to read file");
            let syntax = parse_file(&src).expect("Unable to parse file");
            for item in syntax.items {
                if let Item::Impl(i) = item {
                    if let Type::Path(p) = *i.self_ty {
                        let mut found = false;
                        for segment in p.path.segments {
                            let segment_ident = format!("{}", segment.ident);
                            if ident == segment_ident {
                                found = true;
                            }
                        }
                        if !found {
                            continue;
                        }

                        for impl_item in i.items {
                            if let ImplItem::Method(m) = impl_item {
                                if let Visibility::Public(_) = m.vis {
                                    let mut args: Vec<
                                        DerivedFnArg,
                                    > = vec![];
                                    for arg in m.sig.decl.inputs.iter() {
                                        if let FnArg::Captured(a) = arg {
                                            let name = match &a.pat {
                                                Pat::Ident(p) => format!("{}", p.ident),
                                                _ => panic!("unsupported arg signature in name"),
                                            };
                                            let ty = match &a.ty {
                                                Type::Path(p) => tokens_to_string(*p.path.segments.first().unwrap().value()),
                                                _ => panic!("unsupported arg signature in type"),
                                            };
                                            args.push(DerivedFnArg::Captured { name, ty });
                                        }
                                        if let FnArg::SelfRef(s) = arg {
                                            args.push(DerivedFnArg::SelfBorrow {
                                                mutable: s.mutability.is_some(),
                                            })
                                        }
                                        if let FnArg::SelfValue(s) = arg {
                                            args.push(DerivedFnArg::SelfOwned {
                                                mutable: s.mutability.is_some(),
                                            })
                                        }
                                    }
                                    methods.push(DerivedFn::new(
                                        &format!("{}", &m.sig.ident),
                                        extract_return_type(&m.sig.decl.output),
                                        args,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    methods
}

fn extract_return_type(ty: &ReturnType) -> Option<String> {
    match ty {
        ReturnType::Default => None,
        ReturnType::Type(_, t) => match **t {
            Type::Path(ref p) => Some(tokens_to_string(*p.path.segments.first().unwrap().value())),
            _ => panic!("Unable to extract return type {:?}", ty),
        },
    }
}

fn write_java_class(entity: &DerivedEntity) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let java_dir = format!("{}/java", &out_dir);
    if !Path::new(&java_dir).exists() {
        fs::create_dir(&java_dir).unwrap();
    }

    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let exported = match entity.export_java_syntax(&package_name) {
        Ok(p) => p,
        Err(e) => panic!("{}", e),
    };
    let path = format!("{}/{}.java", java_dir, entity.name());
    fs::write(&path, exported.as_bytes()).unwrap();
}

/// Helper method which turns everything that can be converted into tokens into a String.
///
/// Note that it tries to be semi-intelling on removing whitespace so the output actually
/// looks okay.
fn tokens_to_string<I: ToTokens>(input: &I) -> String {
    let mut ts = proc_macro2::TokenStream::new();
    input.to_tokens(&mut ts);
    let converted = format!("{}", ts);
    let whitespace_removed = converted.replace(" ", "");
    whitespace_removed
}
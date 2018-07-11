use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_str, Expr, Ident};

#[derive(Debug, Fail)]
pub enum ConversionError {
    #[fail(display = "Unsupported Return Type {} on function {}", rt, func)]
    UnsupportedReturnType { func: String, rt: String },
}

/// Describes a function/method associated with the derived struct.
#[derive(Debug)]
pub struct DerivedFn {
    name: String,
    return_type: Option<String>,
    args: Vec<DerivedFnArg>,
}

#[derive(Debug)]
pub enum DerivedFnArg {
    /// &self and &mut self
    SelfBorrow {
        mutable: bool,
    },
    /// self and mut self
    SelfOwned {
        mutable: bool,
    },
    Captured {
        name: String,
        ty: String,
    },
}

impl DerivedFn {
    pub fn new(name: &str, return_type: Option<String>, args: Vec<DerivedFnArg>) -> Self {
        DerivedFn {
            name: name.into(),
            return_type,
            args,
        }
    }

    /// If the argument list contains a reference to self this method is
    /// non-static, otherwise it is.
    pub fn is_static(&self) -> bool {
        for a in &self.args {
            match a {
                DerivedFnArg::SelfBorrow { .. } => return false,
                DerivedFnArg::SelfOwned { .. } => return false,
                _ => (),
            }
        }
        return true;
    }
}

/// Describes the entity which is derived with methods and all.
#[derive(Debug)]
pub struct DerivedEntity {
    name: String,
    fns: Vec<DerivedFn>,
}

impl DerivedEntity {
    /// Creates a new `DerivedEntity`
    pub fn new(name: &str, fns: Vec<DerivedFn>) -> Self {
        DerivedEntity {
            name: name.into(),
            fns: fns,
        }
    }

    /// Returns the name of this derived entity.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Generates the JNI FFI wrapper functions for all the struct method
    /// implementations.
    pub fn export_jni_ffi_tokens(&self) -> TokenStream {
        let mut stream = quote!{};
        for func in &self.fns {
            let struct_name = Ident::new(&self.name, Span::call_site());
            let fn_name = Ident::new(&func.name, Span::call_site());
            let full_fn_name = Ident::new(
                &format!("Java_{}_{}", struct_name, fn_name),
                Span::call_site(),
            );

            let mut args = vec![];
            let mut inner_args = vec![];
            // add JNI env
            args.push(self.raw_arg_to_expr("_env", "roast::JNIEnv"));
            // add JCLass (static method?)
            if func.is_static() {
                args.push(self.raw_arg_to_expr("_class", "roast::JClass"));
            } else {
                args.push(self.raw_arg_to_expr("_obj", "roast::JObject"));
            }
            // add custom args
            for arg in &func.args {
                if let DerivedFnArg::Captured { name, ty } = arg {
                    args.push(self.raw_arg_to_expr(&name, rust_to_jni_type(&ty).unwrap()));
                    inner_args.push(Ident::new(&name, Span::call_site()));
                }
            }

            let retval = parse_str::<Expr>(&rust_to_jni_return_type(&func).unwrap()).unwrap();
            let expanded = quote!{
                #[no_mangle]
                pub extern "system" fn #full_fn_name(#(#args),*) -> #retval {
                    #struct_name::#fn_name(#(#inner_args),*)
                }
            };
            stream.extend(expanded.into_iter());
        }
        stream.into()
    }

    /// Converts an arg tuple of name and type into a expression tree that
    /// can be pushed into the quote macro.
    fn raw_arg_to_expr(&self, name: &str, ty: &str) -> Expr {
        parse_str::<Expr>(&format!("{}: {}", name, ty)).unwrap()
    }

    // Generates the equivalent full java class file for the derived entity.
    pub fn export_java_syntax(&self, lib_name: &str) -> Result<String, ConversionError> {
        let mut converted_methods = String::new();
        converted_methods.push_str(&format!(
            "\n\tstatic {{\n\t\tSystem.loadLibrary(\"{}\");\n\t}}\n",
            lib_name,
        ));

        for func in &self.fns {
            let return_type = rust_to_java_return_type(&func)?;
            let mut args = vec![];
            for arg in &func.args {
                if let DerivedFnArg::Captured { name, ty } = arg {
                    args.push(format!("{} {}", rust_to_java_type(&ty).unwrap(), name));
                }
            }

            let static_qualifier = if func.is_static() { " static" } else { "" };
            let result = format!(
                "\n\tpublic{} native {} {}({});\n",
                static_qualifier,
                return_type,
                func.name,
                args.iter().join(", ")
            );
            converted_methods.push_str(&result);
        }

        let result = format!("public class {} {{\n{}\n}}\n", self.name, converted_methods);

        Ok(result)
    }
}

/// Takes a derived function and returns its return type as a java string.
///
/// If the return type cannot be converted properly, a `ConversionError` is raised.
fn rust_to_java_return_type(func: &DerivedFn) -> Result<String, ConversionError> {
    let ret = &func.return_type;

    Ok(match ret {
        None => "void".into(),
        Some(t) => match rust_to_java_type(&t) {
            Some(v) => v,
            None => {
                return Err(ConversionError::UnsupportedReturnType {
                    rt: t.clone(),
                    func: func.name.clone(),
                })
            }
        }.into(),
    })
}

fn rust_to_jni_return_type(func: &DerivedFn) -> Result<String, ConversionError> {
    let ret = &func.return_type;

    Ok(match ret {
        None => "".into(),
        Some(t) => match rust_to_jni_type(&t) {
            Some(v) => v,
            None => {
                return Err(ConversionError::UnsupportedReturnType {
                    rt: t.clone(),
                    func: func.name.clone(),
                })
            }
        }.into(),
    })
}

/// Converts the string representation of a rust type into its java
/// equivalent.
///
/// Note that for now this method only supports primitive types since
/// more complex types are not implemented as of writing this.
///
/// If None is returned, it means that theo proper conversion could be
/// made.
fn rust_to_java_type(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "i8" => "byte",
        "u8" => "boolean",
        "i16" => "short",
        "u16" => "char",
        "i32" => "int",
        "u32" => "long",
        "i64" => "long",
        "f32" => "float",
        "f64" => "double",
        "bool" => "boolean",
        _ => return None,
    })
}

/// Converts the rust type into its JNI FFI equivalent type.
fn rust_to_jni_type(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "i8" => "roast::jbyte",
        "u8" => "roast::jboolean",
        "i16" => "roast::jshort",
        "u16" => "roast::jchar",
        "i32" => "roast::jint",
        "u32" => "roast::jlong",
        "i64" => "roast::jlong",
        "f32" => "roast::jfloat",
        "f64" => "roast::jdouble",
        "bool" => "roast::jboolean",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn rust_type_to_java_type() {
        assert_eq!(Some("byte"), rust_to_java_type("i8"));
        assert_eq!(Some("boolean"), rust_to_java_type("u8"));
        assert_eq!(Some("short"), rust_to_java_type("i16"));
        assert_eq!(Some("char"), rust_to_java_type("u16"));
        assert_eq!(Some("int"), rust_to_java_type("i32"));
        assert_eq!(Some("long"), rust_to_java_type("u32"));
        assert_eq!(Some("long"), rust_to_java_type("i64"));
        assert_eq!(Some("float"), rust_to_java_type("f32"));
        assert_eq!(Some("double"), rust_to_java_type("f64"));
        assert_eq!(Some("boolean"), rust_to_java_type("bool"));
    }

    #[test]
    fn rust_type_to_jni_type() {
        assert_eq!(Some("roast::jbyte"), rust_to_jni_type("i8"));
        assert_eq!(Some("roast::jboolean"), rust_to_jni_type("u8"));
        assert_eq!(Some("roast::jshort"), rust_to_jni_type("i16"));
        assert_eq!(Some("roast::jchar"), rust_to_jni_type("u16"));
        assert_eq!(Some("roast::jint"), rust_to_jni_type("i32"));
        assert_eq!(Some("roast::jlong"), rust_to_jni_type("u32"));
        assert_eq!(Some("roast::jlong"), rust_to_jni_type("i64"));
        assert_eq!(Some("roast::jfloat"), rust_to_jni_type("f32"));
        assert_eq!(Some("roast::jdouble"), rust_to_jni_type("f64"));
        assert_eq!(Some("roast::jboolean"), rust_to_jni_type("bool"));
    }

    #[test]
    fn java_convert_no_methods() {
        let derived = DerivedEntity::new("Entity", vec![]);
        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_static_no_arg_no_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("foobar", None, vec![]));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native void foobar();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_no_arg_no_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foobar",
            None,
            vec![DerivedFnArg::SelfBorrow { mutable: false }],
        ));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public native void foobar();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_static_no_arg_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("foobar", Some("i32".into()), vec![]));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native int foobar();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_static_arg_no_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foobar",
            None,
            vec![DerivedFnArg::Captured {
                name: "a".into(),
                ty: "i64".into(),
            }],
        ));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native void foobar(long a);

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_static_arg_and_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foobar",
            Some("bool".into()),
            vec![
                DerivedFnArg::Captured {
                    name: "a".into(),
                    ty: "i32".into(),
                },
                DerivedFnArg::Captured {
                    name: "b".into(),
                    ty: "i16".into(),
                },
            ],
        ));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native boolean foobar(int a, short b);

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_static_two_methods() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foo",
            Some("bool".into()),
            vec![
                DerivedFnArg::Captured {
                    name: "a".into(),
                    ty: "i32".into(),
                },
                DerivedFnArg::Captured {
                    name: "b".into(),
                    ty: "i16".into(),
                },
            ],
        ));
        fns.push(DerivedFn::new("bar", Some("i32".into()), vec![]));

        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native boolean foo(int a, short b);

	public static native int bar();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn java_convert_mixed_static_nonstatic_two_methods() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foo",
            Some("bool".into()),
            vec![
                DerivedFnArg::Captured {
                    name: "a".into(),
                    ty: "i32".into(),
                },
                DerivedFnArg::Captured {
                    name: "b".into(),
                    ty: "i16".into(),
                },
                DerivedFnArg::SelfOwned { mutable: true },
            ],
        ));
        fns.push(DerivedFn::new("bar", Some("i32".into()), vec![]));

        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public native boolean foo(int a, short b);

	public static native int bar();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }
}

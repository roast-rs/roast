use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
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

impl DerivedFnArg {
    fn name(&self) -> Option<String> {
        match self {
            DerivedFnArg::Captured { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    fn java_name(&self) -> Option<String> {
        match self {
            DerivedFnArg::Captured { name, .. } => Some(name.to_camel_case()),
            _ => None,
        }
    }
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

    /// Returns the rust style function name turned into java style.
    pub fn java_name(&self) -> String {
        self.name.to_camel_case()
    }

    /// Takes the return type but simply removes all invalid chars so it can
    /// be used in rust code as part of the function signatures.
    pub fn sanitized_return_type(&self) -> Option<String> {
        self.return_type
            .as_ref()
            .map(|t| t.replace("<", "").replace(">", "").replace(" ", ""))
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
            let jni_name = Ident::new(
                &format!("Java_{}_{}", struct_name, &func.java_name()),
                Span::call_site(),
            );

            let raw_ret_type =
                rust_to_jni_return_type(&func).expect("Could not convert JNI return type");

            let mut args = vec![];
            let mut inner_args = vec![];

            // add custom args
            for arg in &func.args {
                if let DerivedFnArg::Captured { name: _name, ty } = arg {
                    args.push(self.raw_arg_to_expr(
                        &arg.name().expect("Could not read java name"),
                        rust_to_jni_type(&ty).expect("Could not convert rust to jni type"),
                    ));

                    let convert_fn = format!(
                        "roast::convert::convert_arg_{}(&env, {})",
                        rust_to_jni_type(&ty)
                            .expect("Could not convert rust to jni type")
                            .replace("roast::", "")
                            .to_lowercase(),
                        &arg.name().expect("Could not read java name")
                    );
                    inner_args
                        .push(parse_str::<Expr>(&convert_fn).expect("Could not parse expression"));
                }
            }

            // add JNI env
            if raw_ret_type.is_some() || !inner_args.is_empty() {
                // for now we only need the env if we parse return values
                args.insert(0, self.raw_arg_to_expr("env", "roast::JNIEnv"));
            } else {
                args.insert(0, self.raw_arg_to_expr("_env", "roast::JNIEnv"));
            }
            // add JCLass (static method?)
            if func.is_static() {
                args.insert(1, self.raw_arg_to_expr("_class", "roast::JClass"));
            } else {
                args.insert(1, self.raw_arg_to_expr("_obj", "roast::JObject"));
            }

            // todo: switch some
            let expanded = if raw_ret_type.is_none() {
                // no return argument, skip the ret conversion
                quote!{
                    #[no_mangle]
                    pub extern "system" fn #jni_name(#(#args),*) {
                       #struct_name::#fn_name(#(#inner_args),*)
                    }
                }
            } else {
                let retval = parse_str::<Expr>(&raw_ret_type.unwrap()).unwrap();
                let convert_fn = format!(
                    "roast::convert::convert_retval_{}",
                    func.sanitized_return_type()
                        .as_ref()
                        .unwrap()
                        .to_lowercase()
                );
                let convert_ret_fn_name = parse_str::<Expr>(&convert_fn).unwrap();
                // we got a return value, so add a conversion wrapper
                quote!{
                    #[no_mangle]
                    pub extern "system" fn #jni_name(#(#args),*) -> #retval {
                       #convert_ret_fn_name(&env, #struct_name::#fn_name(#(#inner_args),*))
                    }
                }
            };
            stream.extend(expanded.into_iter());
        }
        stream
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
                if let DerivedFnArg::Captured { name: _name, ty } = arg {
                    args.push(format!(
                        "{} {}",
                        rust_to_java_type(&ty).unwrap(),
                        arg.java_name().unwrap()
                    ));
                }
            }

            let static_qualifier = if func.is_static() { " static" } else { "" };
            let result = format!(
                "\n\tpublic{} native {} {}({});\n",
                static_qualifier,
                return_type,
                func.java_name(),
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

fn rust_to_jni_return_type(func: &DerivedFn) -> Result<Option<String>, ConversionError> {
    let ret = &func.return_type;

    Ok(match ret {
        None => None,
        Some(t) => match rust_to_jni_type(&t) {
            Some(v) if v == "roast::JString" => Some(v.to_lowercase()),
            Some(v) => Some(v.into()),
            None => {
                return Err(ConversionError::UnsupportedReturnType {
                    rt: t.clone(),
                    func: func.name.clone(),
                })
            }
        },
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
        "i64" => "long",
        "f32" => "float",
        "f64" => "double",
        "bool" => "boolean",
        "String" => "String",
        "Vec<u8>" => "byte[]",
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
        "i64" => "roast::jlong",
        "f32" => "roast::jfloat",
        "f64" => "roast::jdouble",
        "bool" => "roast::jboolean",
        "String" => "roast::JString",
        "Vec<u8>" => "roast::jbyteArray",
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
        assert_eq!(Some("long"), rust_to_java_type("i64"));
        assert_eq!(Some("float"), rust_to_java_type("f32"));
        assert_eq!(Some("double"), rust_to_java_type("f64"));
        assert_eq!(Some("boolean"), rust_to_java_type("bool"));
        assert_eq!(Some("String"), rust_to_java_type("String"));
        assert_eq!(Some("byte[]"), rust_to_java_type("Vec<u8>"));
    }

    #[test]
    fn rust_type_to_jni_type() {
        assert_eq!(Some("roast::jbyte"), rust_to_jni_type("i8"));
        assert_eq!(Some("roast::jboolean"), rust_to_jni_type("u8"));
        assert_eq!(Some("roast::jshort"), rust_to_jni_type("i16"));
        assert_eq!(Some("roast::jchar"), rust_to_jni_type("u16"));
        assert_eq!(Some("roast::jint"), rust_to_jni_type("i32"));
        assert_eq!(Some("roast::jlong"), rust_to_jni_type("i64"));
        assert_eq!(Some("roast::jfloat"), rust_to_jni_type("f32"));
        assert_eq!(Some("roast::jdouble"), rust_to_jni_type("f64"));
        assert_eq!(Some("roast::jboolean"), rust_to_jni_type("bool"));
        assert_eq!(Some("roast::JString"), rust_to_jni_type("String"));
        assert_eq!(Some("roast::jbyteArray"), rust_to_jni_type("Vec<u8>"));
    }

    #[test]
    fn func_name_to_java_style() {
        assert_eq!(
            String::from("func"),
            DerivedFn::new("func", None, vec![]).java_name()
        );
        assert_eq!(
            String::from("myFuncName"),
            DerivedFn::new("my_func_name", None, vec![]).java_name()
        );
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
    fn ffi_convert_no_methods() {
        let derived = DerivedEntity::new("Entity", vec![]);
        let tokens = derived.export_jni_ffi_tokens();
        let exported = format!("{}", tokens);
        assert!(exported.is_empty());
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
    fn ffi_convert_static_no_arg_no_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("foobar", None, vec![]));
        let derived = DerivedEntity::new("Entity", fns);
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected = "# [ no_mangle ] pub extern \"system\" fn \
                        Java_Entity_foobar ( _env : roast :: JNIEnv , _class : roast :: JClass ) \
                        { Entity :: foobar ( ) }";
        assert_eq!(expected, exported);
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
    fn ffi_convert_no_arg_no_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "foobar",
            None,
            vec![DerivedFnArg::SelfBorrow { mutable: false }],
        ));
        let derived = DerivedEntity::new("Entity", fns);
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected = "# [ no_mangle ] pub extern \"system\" fn \
                        Java_Entity_foobar ( _env : roast :: JNIEnv , _obj : roast :: JObject ) \
                        { Entity :: foobar ( ) }";
        assert_eq!(expected, exported);
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
    fn ffi_convert_static_no_arg_ret() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("foobar", Some("i32".into()), vec![]));
        let derived = DerivedEntity::new("Entity", fns);
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn \
             Java_Entity_foobar ( env : roast :: JNIEnv , _class : roast :: JClass ) -> \
             roast :: jint { roast :: convert :: convert_retval_i32 ( & env , Entity :: foobar ( ) ) }";
        assert_eq!(expected, exported);
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
    fn ffi_convert_static_arg_no_ret() {
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
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn Java_Entity_foobar \
             ( env : roast :: JNIEnv , _class : roast :: JClass , a : roast :: jlong ) \
             { Entity :: foobar ( roast :: convert :: convert_arg_jlong ( & env , a ) ) }";
        assert_eq!(expected, exported);
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
    fn ffi_convert_static_arg_and_ret() {
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
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected = "# [ no_mangle ] pub extern \"system\" fn Java_Entity_foobar \
                        ( env : roast :: JNIEnv , _class : roast :: JClass , \
                        a : roast :: jint , b : roast :: jshort ) -> roast :: jboolean \
                        { roast :: convert :: convert_retval_bool ( & env , Entity :: foobar \
                        ( roast :: convert :: convert_arg_jint ( & env , a ) , \
                        roast :: convert :: convert_arg_jshort ( & env , b ) ) ) }";
        assert_eq!(expected, exported);
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
    fn ffi_convert_static_two_methods() {
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
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn Java_Entity_foo \
             ( env : roast :: JNIEnv , _class : roast :: JClass , a : roast :: jint , \
             b : roast :: jshort ) -> roast :: jboolean { roast :: convert :: convert_retval_bool \
             ( & env , Entity :: foo ( roast :: convert :: convert_arg_jint ( & env , a ) , \
             roast :: convert :: convert_arg_jshort ( & env , b ) ) ) } \
             # [ no_mangle ] pub extern \"system\" fn Java_Entity_bar ( env : roast :: JNIEnv , \
             _class : roast :: JClass ) -> roast :: jint { roast :: convert :: convert_retval_i32 \
             ( & env , Entity :: bar ( ) ) }";
        assert_eq!(expected, exported);
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

    #[test]
    fn ffi_convert_mixed_static_nonstatic_two_methods() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "get_foo_bar",
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
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn Java_Entity_getFooBar \
             ( env : roast :: JNIEnv , _obj : roast :: JObject , a : roast :: jint , b : \
             roast :: jshort ) -> roast :: jboolean { roast :: convert :: convert_retval_bool \
             ( & env , Entity :: get_foo_bar ( roast :: convert :: convert_arg_jint ( & env , a ) \
             , roast :: convert :: convert_arg_jshort ( & env , b ) ) ) } \
             # [ no_mangle ] pub extern \"system\" fn Java_Entity_bar ( env : roast :: JNIEnv , \
             _class : roast :: JClass ) -> roast :: jint { roast :: convert :: convert_retval_i32 \
             ( & env , Entity :: bar ( ) ) }";
        assert_eq!(expected, exported);
    }

    #[test]
    fn ffi_convert_string_return_value() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("myfunc", Some("String".into()), vec![]));
        let derived = DerivedEntity::new("Entity", fns);
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn Java_Entity_myfunc \
             ( env : roast :: JNIEnv , _class : roast :: JClass ) -> roast :: jstring \
             { roast :: convert :: convert_retval_string ( & env , Entity :: myfunc ( ) ) }";
        assert_eq!(expected, exported);
    }

    #[test]
    fn java_convert_string_return_value() {
        let mut fns = vec![];
        fns.push(DerivedFn::new("myfunc", Some("String".into()), vec![]));
        let derived = DerivedEntity::new("Entity", fns);

        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native String myfunc();

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }

    #[test]
    fn ffi_convert_string_arg_value() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "my_func",
            None,
            vec![DerivedFnArg::Captured {
                name: "my_var".into(),
                ty: "String".into(),
            }],
        ));
        let derived = DerivedEntity::new("Entity", fns);
        let exported = format!("{}", derived.export_jni_ffi_tokens());
        let expected =
            "# [ no_mangle ] pub extern \"system\" fn Java_Entity_myFunc \
             ( env : roast :: JNIEnv , _class : roast :: JClass , my_var : roast :: JString ) \
             { Entity :: my_func ( roast :: convert :: convert_arg_jstring ( & env , my_var ) ) }";
        assert_eq!(expected, exported);
    }

    #[test]
    fn java_convert_string_arg_value() {
        let mut fns = vec![];
        fns.push(DerivedFn::new(
            "my_func",
            None,
            vec![DerivedFnArg::Captured {
                name: "my_var".into(),
                ty: "String".into(),
            }],
        ));
        let derived = DerivedEntity::new("Entity", fns);
        let expected = r#"public class Entity {

	static {
		System.loadLibrary("mylib");
	}

	public static native void myFunc(String myVar);

}
"#;
        assert_eq!(expected, derived.export_java_syntax("mylib").unwrap());
    }
}

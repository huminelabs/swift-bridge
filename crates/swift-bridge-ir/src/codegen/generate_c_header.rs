//! Tests can be found in src/codegen/codegen_tests.rs and its submodules.

use crate::bridged_type::shared_struct::StructField;
use crate::bridged_type::{BridgedType, StdLibType, StructFields};
use crate::codegen::CodegenConfig;
use crate::parse::{SharedTypeDeclaration, TypeDeclaration, TypeDeclarations};
use crate::parsed_extern_fn::ParsedExternFn;
use crate::{SwiftBridgeModule, SWIFT_BRIDGE_PREFIX};
use std::collections::{BTreeSet, HashSet};
use syn::ReturnType;

const NOTICE: &'static str = "// File automatically generated by swift-bridge.";

struct Bookkeeping {
    includes: BTreeSet<&'static str>,
    slice_types: HashSet<String>,
}

impl SwiftBridgeModule {
    /// Generate the contents of a C header file based on the contents of this module.
    pub(crate) fn generate_c_header(&self, config: &CodegenConfig) -> String {
        format!(
            r#"{notice}
{header}"#,
            notice = NOTICE,
            header = self.generate_c_header_inner(config)
        )
    }

    pub(crate) fn generate_c_header_inner(&self, config: &CodegenConfig) -> String {
        let mut header = "".to_string();

        if !self.module_will_be_compiled(config) {
            return header;
        }

        let mut bookkeeping = Bookkeeping {
            includes: BTreeSet::new(),
            // TODO: Delete this.
            //  Don't think we're using it.
            slice_types: HashSet::new(),
        };

        for ty in self.types.types() {
            match ty {
                TypeDeclaration::Shared(ty) => match ty {
                    SharedTypeDeclaration::Struct(ty_struct) => {
                        if ty_struct.already_declared {
                            continue;
                        }

                        let name = ty_struct.swift_name_string();
                        let ffi_name = ty_struct.ffi_name_string();
                        let option_ffi_name = ty_struct.ffi_option_name_string();

                        let mut fields = vec![];

                        // Used for `Option<T>` ...
                        // typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; ... }
                        bookkeeping.includes.insert("stdbool.h");

                        // Empty structs get represented as
                        //  `__swift_bridge__$MyStruct { uint8_t _private }`
                        if ty_struct.fields.is_empty() {
                            bookkeeping.includes.insert("stdint.h");

                            fields.push("uint8_t _private".to_string())
                        } else {
                            match &ty_struct.fields {
                                StructFields::Named(f) => {
                                    for field in f.iter() {
                                        let ty = BridgedType::new_with_type(&field.ty, &self.types)
                                            .unwrap();
                                        if let Some(include) = ty.to_c_include() {
                                            bookkeeping.includes.insert(include);
                                        }

                                        let name = field.swift_name_string();

                                        fields.push(format!("{} {}", ty.to_c(), name));
                                    }
                                }
                                StructFields::Unnamed(types) => {
                                    for (idx, field) in types.iter().enumerate() {
                                        let ty = BridgedType::new_with_type(&field.ty, &self.types)
                                            .unwrap();
                                        if let Some(include) = ty.to_c_include() {
                                            bookkeeping.includes.insert(include);
                                        }

                                        let name = format!("_{}", idx);

                                        fields.push(format!("{} {}", ty.to_c(), name));
                                    }
                                }
                                StructFields::Unit => {
                                    // SAFETY: This can't be reached since we check if the struct
                                    //  has no fields above.
                                    unreachable!()
                                }
                            }
                        }

                        let maybe_fields = if fields.len() > 0 {
                            let mut maybe_fields = " ".to_string();

                            maybe_fields += &fields.join("; ");

                            maybe_fields += "; ";
                            maybe_fields
                        } else {
                            "".to_string()
                        };

                        let ty_decl = format!(
                            r#"typedef struct {prefix}${name} {{{maybe_fields}}} {prefix}${name};
typedef struct {option_ffi_name} {{ bool is_some; {ffi_name} val; }} {option_ffi_name};"#,
                            prefix = SWIFT_BRIDGE_PREFIX,
                            ffi_name = ffi_name,
                            option_ffi_name = option_ffi_name,
                            name = name,
                            maybe_fields = maybe_fields
                        );

                        header += &ty_decl;
                        header += "\n";
                    }
                    SharedTypeDeclaration::Enum(ty_enum) => {
                        if ty_enum.already_declared {
                            continue;
                        }

                        let ffi_name = ty_enum.ffi_name_string();
                        let ffi_tag_name = ty_enum.ffi_tag_name_string();
                        let option_ffi_name = ty_enum.ffi_option_name_string();

                        // Used for `Option<T>` ...
                        // typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; ...
                        bookkeeping.includes.insert("stdbool.h");

                        let mut variants = "".to_string();

                        for variant in ty_enum.variants.iter() {
                            let v = format!("{}${}, ", ffi_name, variant.name);
                            variants += &v;
                        }

                        let maybe_vec_support = if ty_enum.has_one_or_more_variants_with_data() {
                            "".to_string()
                        } else {
                            vec_transparent_enum_c_support(&ty_enum.swift_name_string())
                        };

                        let enum_decl = format!(
                            r#"typedef enum {ffi_tag_name} {{ {variants}}} {ffi_tag_name};
typedef struct {ffi_name} {{ {ffi_tag_name} tag; }} {ffi_name};
typedef struct {option_ffi_name} {{ bool is_some; {ffi_name} val; }} {option_ffi_name};{maybe_vec_support}"#,
                            ffi_name = ffi_name,
                            ffi_tag_name = ffi_tag_name,
                            option_ffi_name = option_ffi_name,
                            variants = variants
                        );

                        header += &enum_decl;
                        header += "\n";
                    }
                },
                TypeDeclaration::Opaque(ty) => {
                    if ty.host_lang.is_swift() {
                        continue;
                    }

                    if ty.attributes.already_declared {
                        continue;
                    }

                    if ty.attributes.declare_generic {
                        continue;
                    }
                    if ty.attributes.hashable {
                        let ty_name = ty.ty_name_ident();
                        let hash_ty =
                            format!("uint64_t __swift_bridge__${}$_hash(void* self);", ty_name);
                        header += &hash_ty;
                    }
                    if ty.attributes.equatable {
                        let ty_name = ty.ty_name_ident();
                        let equal_ty = format!(
                            "bool __swift_bridge__${}$_partial_eq(void* lhs, void* rhs);",
                            ty_name
                        );
                        bookkeeping.includes.insert("stdint.h");
                        bookkeeping.includes.insert("stdbool.h");
                        header += &equal_ty;
                        header += "\n";
                    }
                    let ty_name = ty.to_string();

                    if let Some(copy) = ty.attributes.copy {
                        bookkeeping.includes.insert("stdint.h");
                        bookkeeping.includes.insert("stdbool.h");
                        let c_ty_name = ty.ffi_copy_repr_string();
                        let c_option_ty_name = ty.ffi_option_copy_repr_string();

                        let ty_decl = format!(
                            "typedef struct {copy_ffi_repr} {{ uint8_t bytes[{size}]; }} {copy_ffi_repr};",
                            copy_ffi_repr = c_ty_name,
                            size = copy.size_bytes
                        );
                        let option_ty_decl = format!(
                            "typedef struct {option_copy_ffi_repr} {{ bool is_some; {copy_ffi_repr} val; }} {option_copy_ffi_repr};",
                            copy_ffi_repr = c_ty_name,
                            option_copy_ffi_repr = c_option_ty_name,
                        );

                        header += &ty_decl;
                        header += "\n";
                        header += &option_ty_decl;
                        header += "\n";
                    } else {
                        let ty_decl =
                            format!("typedef struct {ty_name} {ty_name};", ty_name = ty_name);
                        header += &ty_decl;
                        header += "\n";

                        let generics = ty.generics.dollar_prefixed_generics_string();
                        let drop_ty = format!(
                            r#"void __swift_bridge__${ty_name}{generics}$_free(void* self);"#,
                            ty_name = ty_name,
                            generics = generics
                        );

                        header += &drop_ty;
                        header += "\n";
                    }

                    // TODO: Support Vec<OpaqueCopyType>. Add codegen tests and then
                    //  make them pass.
                    // TODO: Support Vec<GenericOpaqueRustType
                    if ty.attributes.copy.is_none() && ty.generics.len() == 0 {
                        let vec_functions = vec_opaque_rust_type_c_support(&ty_name);

                        header += &vec_functions;
                        header += "\n";
                    }
                }
            }
        }

        for func in self.functions.iter() {
            if func.host_lang.is_swift() {
                for (idx, boxed_fn) in func.args_filtered_to_boxed_fns(&self.types) {
                    if boxed_fn.params.is_empty() && boxed_fn.ret.is_null() {
                        continue;
                    }

                    let fns = func.boxed_fn_to_c_header_fns(idx, &boxed_fn);
                    header += &format!("{fns}");
                    header += "\n";
                }
                continue;
            }

            header += &declare_func(&func, &mut bookkeeping, &self.types);
        }

        for slice_ty in bookkeeping.slice_types.iter() {
            header = format!(
                r#"typedef struct FfiSlice_{slice_ty} {{ {slice_ty}* start; uintptr_t len; }} FfiSlice_{slice_ty};
{header}"#,
                slice_ty = slice_ty,
                header = header
            )
        }

        let mut includes = bookkeeping.includes.iter().collect::<Vec<_>>();
        includes.sort();
        for include in includes {
            header = format!(
                r#"#include <{}>
{}"#,
                include, header
            );
        }

        header
    }
}

fn vec_opaque_rust_type_c_support(ty_name: &str) -> String {
    format!(
        r#"
void* __swift_bridge__$Vec_{ty_name}$new(void);
void __swift_bridge__$Vec_{ty_name}$drop(void* vec_ptr);
void __swift_bridge__$Vec_{ty_name}$push(void* vec_ptr, void* item_ptr);
void* __swift_bridge__$Vec_{ty_name}$pop(void* vec_ptr);
void* __swift_bridge__$Vec_{ty_name}$get(void* vec_ptr, uintptr_t index);
void* __swift_bridge__$Vec_{ty_name}$get_mut(void* vec_ptr, uintptr_t index);
uintptr_t __swift_bridge__$Vec_{ty_name}$len(void* vec_ptr);
void* __swift_bridge__$Vec_{ty_name}$as_ptr(void* vec_ptr);
"#,
        ty_name = ty_name
    )
}

fn vec_transparent_enum_c_support(enum_name: &str) -> String {
    format!(
        r#"
void* __swift_bridge__$Vec_{enum_name}$new(void);
void __swift_bridge__$Vec_{enum_name}$drop(void* vec_ptr);
void __swift_bridge__$Vec_{enum_name}$push(void* vec_ptr, __swift_bridge__${enum_name} item);
__swift_bridge__$Option${enum_name} __swift_bridge__$Vec_{enum_name}$pop(void* vec_ptr);
__swift_bridge__$Option${enum_name} __swift_bridge__$Vec_{enum_name}$get(void* vec_ptr, uintptr_t index);
__swift_bridge__$Option${enum_name} __swift_bridge__$Vec_{enum_name}$get_mut(void* vec_ptr, uintptr_t index);
uintptr_t __swift_bridge__$Vec_{enum_name}$len(void* vec_ptr);
void* __swift_bridge__$Vec_{enum_name}$as_ptr(void* vec_ptr);
"#,
        enum_name = enum_name
    )
}

fn declare_func(
    func: &ParsedExternFn,
    bookkeeping: &mut Bookkeeping,
    types: &TypeDeclarations,
) -> String {
    let ret = func.to_c_header_return(types);
    let name = func.link_name();
    let params = func.to_c_header_params(types);

    if let ReturnType::Type(_, ty) = &func.func.sig.output {
        if let Some(ty) = BridgedType::new_with_type(&ty, types) {
            if let BridgedType::StdLib(StdLibType::RefSlice(ref_slice)) = ty {
                bookkeeping.slice_types.insert(ref_slice.ty.to_c());
            }
        }
    }

    if let Some(includes) = func.c_includes(types) {
        for include in includes {
            bookkeeping.includes.insert(include);
        }
    }

    let declaration = if func.sig.asyncness.is_some() {
        let maybe_ret = BridgedType::new_with_return_type(&func.sig.output, types).unwrap();
        let maybe_ret = if maybe_ret.is_null() {
            "".to_string()
        } else {
            format!(", {} ret", maybe_ret.to_c())
        };

        let maybe_params = if func.sig.inputs.is_empty() {
            "".to_string()
        } else {
            format!(", {}", params)
        };

        format!(
            "void {name}(void* callback_wrapper, void {name}$async(void* callback_wrapper{maybe_ret}){maybe_params});\n",
            name = name,
            maybe_ret = maybe_ret
        )
    } else {
        format!(
            "{ret} {name}({params});\n",
            ret = ret,
            name = name,
            params = params
        )
    };

    declaration
}

#[cfg(test)]
mod tests {
    //! More tests can be found in src/codegen/codegen_tests.rs and its submodules.

    use proc_macro2::TokenStream;
    use quote::quote;

    use crate::parse::SwiftBridgeModuleAndErrors;
    use crate::test_utils::{
        assert_trimmed_generated_contains_trimmed_expected,
        assert_trimmed_generated_equals_trimmed_expected,
    };
    use crate::SwiftBridgeModule;

    use super::*;

    /// Verify that we generate an empty header file for an empty module.
    #[test]
    fn generates_empty_header_for_empty_section() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" { }
            }
        };
        let module = parse_ok(tokens);

        let header = module.generate_c_header(&CodegenConfig::no_features_enabled());
        assert_eq!(header.trim(), NOTICE)
    }

    /// Verify that we do not generate any headers for extern "Swift" blocks since Rust does not
    /// need any C headers.
    #[test]
    fn ignores_extern_swift() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Swift" {
                    type Foo;
                    fn bar ();
                }
            }
        };
        let module = parse_ok(tokens);

        let header = module.generate_c_header(&CodegenConfig::no_features_enabled());
        assert_eq!(header.trim(), NOTICE)
    }

    /// Verify that we generate a type definition for a freestanding function that has no args.
    #[test]
    fn freestanding_function_no_args() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn foo();
                }
            }
        };
        let expected = r#"
void __swift_bridge__$foo(void);
        "#;

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we generate a type definition for a freestanding function that has one arg.
    #[test]
    fn freestanding_function_one_args() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn foo(arg1: u8);
                }
            }
        };
        let expected = r#"
#include <stdint.h>
void __swift_bridge__$foo(uint8_t arg1);
        "#;

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we generate a type definition for a freestanding function that returns a value.
    #[test]
    fn freestanding_function_with_return() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn foo() -> u8;
                }
            }
        };
        let expected = r#"
#include <stdint.h>
uint8_t __swift_bridge__$foo(void);
        "#;

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we include the Vec<T> functions in the generated C header for a Rust type.
    #[test]
    fn type_definition_includes_vec_functions() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                }
            }
        };
        let expected = format!(
            r#"
typedef struct SomeType SomeType;
void __swift_bridge__$SomeType$_free(void* self);
{}
"#,
            vec_opaque_rust_type_c_support("SomeType")
        );

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we generate a type definition for a method with no arguments.
    #[test]
    fn method_no_args() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn a(self);
                    fn b(&self);
                    fn c(&mut self);
                    fn d(self: SomeType);
                    fn e(self: &SomeType);
                    fn f(self: &mut SomeType);
                }
            }
        };
        let expected = format!(
            r#"
void __swift_bridge__$SomeType$a(void* self);
void __swift_bridge__$SomeType$b(void* self);
void __swift_bridge__$SomeType$c(void* self);
void __swift_bridge__$SomeType$d(void* self);
void __swift_bridge__$SomeType$e(void* self);
void __swift_bridge__$SomeType$f(void* self);
        "#,
        );

        let module = parse_ok(tokens);
        assert_trimmed_generated_contains_trimmed_expected(
            &module.generate_c_header_inner(&CodegenConfig::no_features_enabled()),
            &expected,
        );
    }

    /// Verify that we generate a type definition for a method with no arguments.
    #[test]
    fn method_one_arg() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn foo(&self, val: u8);
                }
            }
        };
        let expected = format!(
            r#"
#include <stdint.h>
typedef struct SomeType SomeType;
void __swift_bridge__$SomeType$_free(void* self);
{}
void __swift_bridge__$SomeType$foo(void* self, uint8_t val);
        "#,
            vec_opaque_rust_type_c_support("SomeType")
        );

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we generate a type definition for a method with an opaque argument.
    #[test]
    fn method_one_opaque_arg() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn foo(&self, val: SomeType);
                }
            }
        };
        let expected = format!(
            r#"
typedef struct SomeType SomeType;
void __swift_bridge__$SomeType$_free(void* self);
{}
void __swift_bridge__$SomeType$foo(void* self, void* val);
        "#,
            vec_opaque_rust_type_c_support("SomeType")
        );

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we generate a type definition for a method that has a return type.
    #[test]
    fn method_with_return() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn foo(&self) -> u8;
                }
            }
        };
        let expected = format!(
            r#"
#include <stdint.h>
typedef struct SomeType SomeType;
void __swift_bridge__$SomeType$_free(void* self);
{}
uint8_t __swift_bridge__$SomeType$foo(void* self);
        "#,
            vec_opaque_rust_type_c_support("SomeType")
        );

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    /// Verify that we define a FfiSlice_T struct if we return a slice of type T.
    /// We make sure to only define one instance of FfiSlice_T even if there are multiple functions
    /// that need it.
    #[test]
    fn slice_return() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn foo() -> &'static [u8];
                    fn bar() -> &'static [u8];
                }
            }
        };
        let expected = r#"
#include <stdint.h>
typedef struct FfiSlice_uint8_t { uint8_t* start; uintptr_t len; } FfiSlice_uint8_t;
struct __private__FfiSlice __swift_bridge__$foo(void);
struct __private__FfiSlice __swift_bridge__$bar(void);
        "#;

        let module = parse_ok(tokens);
        assert_eq!(
            module
                .generate_c_header_inner(&CodegenConfig::no_features_enabled())
                .trim(),
            expected.trim()
        );
    }

    fn parse_ok(tokens: TokenStream) -> SwiftBridgeModule {
        let module_and_errors: SwiftBridgeModuleAndErrors = syn::parse2(tokens).unwrap();
        module_and_errors.module
    }

    /// Verify that we generate a proper header for a Rust function that returns an owned Swift
    /// type.
    #[test]
    fn extern_rust_fn_returns_extern_swift_owned_opaque_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function() -> Foo;
                }

                extern "Swift" {
                    type Foo;
                }
            }
        };
        let expected = r#"
void* __swift_bridge__$some_function(void);
        "#;

        let module = parse_ok(tokens);
        assert_trimmed_generated_equals_trimmed_expected(
            &module.generate_c_header_inner(&CodegenConfig::no_features_enabled()),
            &expected,
        );
    }
}

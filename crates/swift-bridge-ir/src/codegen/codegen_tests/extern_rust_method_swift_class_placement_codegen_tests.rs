use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that extern "Rust" methods get added to the proper Swift class.
///
/// self -> SomeType
/// &self -> SomeTypeRef
/// &mut self -> SomeTypeRefMut
mod extern_rust_method_swift_class_placement {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    fn a(self);
                    fn b(self: SomeType);

                    fn c(&self);
                    fn d(self: &SomeType);

                    fn e(&mut self);
                    fn f(self: &mut SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::SkipTest
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
internal class SomeType: SomeTypeRefMut {
    var isOwned: Bool = true

    internal override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$SomeType$_free(ptr)
        }
    }
}
extension SomeType {
    internal func a() {
        __swift_bridge__$SomeType$a({isOwned = false; return ptr;}())
    }

    internal func b() {
        __swift_bridge__$SomeType$b({isOwned = false; return ptr;}())
    }
}
internal class SomeTypeRefMut: SomeTypeRef {
    internal override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
extension SomeTypeRefMut {
    internal func e() {
        __swift_bridge__$SomeType$e(ptr)
    }

    internal func f() {
        __swift_bridge__$SomeType$f(ptr)
    }
}
internal class SomeTypeRef {
    var ptr: UnsafeMutableRawPointer

    internal init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension SomeTypeRef {
    internal func c() {
        __swift_bridge__$SomeType$c(ptr)
    }

    internal func d() {
        __swift_bridge__$SomeType$d(ptr)
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn extern_rust_fn_return_option_string() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

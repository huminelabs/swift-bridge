internal class RustString: RustStringRefMut {
    var isOwned: Bool = true

    internal override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RustString$_free(ptr)
        }
    }
}
extension RustString {
    internal convenience init() {
        self.init(ptr: __swift_bridge__$RustString$new())
    }

    internal convenience init<GenericToRustStr: ToRustStr>(_ str: GenericToRustStr) {
        self.init(ptr: str.toRustStr({ strAsRustStr in
            __swift_bridge__$RustString$new_with_str(strAsRustStr)
        }))
    }
}
internal class RustStringRefMut: RustStringRef {
    internal override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
internal class RustStringRef {
    var ptr: UnsafeMutableRawPointer

    internal init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RustStringRef {
    internal func len() -> UInt {
        __swift_bridge__$RustString$len(ptr)
    }

    internal func as_str() -> RustStr {
        __swift_bridge__$RustString$as_str(ptr)
    }

    internal func trim() -> RustStr {
        __swift_bridge__$RustString$trim(ptr)
    }
}
extension RustString: Vectorizable {
    internal static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RustString$new()
    }

    internal static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RustString$drop(vecPtr)
    }

    internal static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RustString) {
        __swift_bridge__$Vec_RustString$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    internal static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RustString$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RustString(ptr: pointer!) as! Self)
        }
    }

    internal static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustStringRef> {
        let pointer = __swift_bridge__$Vec_RustString$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustStringRef(ptr: pointer!)
        }
    }

    internal static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustStringRefMut> {
        let pointer = __swift_bridge__$Vec_RustString$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustStringRefMut(ptr: pointer!)
        }
    }

    internal static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RustStringRef> {
        UnsafePointer<RustStringRef>(OpaquePointer(__swift_bridge__$Vec_RustString$as_ptr(vecPtr)))
    }

    internal static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RustString$len(vecPtr)
    }
}
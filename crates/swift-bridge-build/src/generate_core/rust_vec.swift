internal class RustVec<T: Vectorizable> {
    var ptr: UnsafeMutableRawPointer
    var isOwned: Bool = true

    internal init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    internal init() {
        ptr = T.vecOfSelfNew()
        isOwned = true
    }

    internal func push (value: T) {
        T.vecOfSelfPush(vecPtr: ptr, value: value)
    }

    internal func pop () -> Optional<T> {
        T.vecOfSelfPop(vecPtr: ptr)
    }

    internal func get(index: UInt) -> Optional<T.SelfRef> {
         T.vecOfSelfGet(vecPtr: ptr, index: index)
    }

    internal func as_ptr() -> UnsafePointer<T.SelfRef> {
        UnsafePointer<T.SelfRef>(OpaquePointer(T.vecOfSelfAsPtr(vecPtr: ptr)))
    }

    /// Rust returns a UInt, but we cast to an Int because many Swift APIs such as
    /// `ForEach(0..rustVec.len())` expect Int.
    internal func len() -> Int {
        Int(T.vecOfSelfLen(vecPtr: ptr))
    }

    deinit {
        if isOwned {
            T.vecOfSelfFree(vecPtr: ptr)
        }
    }
}

extension RustVec: Sequence {
    internal func makeIterator() -> RustVecIterator<T> {
        return RustVecIterator(self)
    }
}

internal struct RustVecIterator<T: Vectorizable>: IteratorProtocol {
    var rustVec: RustVec<T>
    var index: UInt = 0

    init (_ rustVec: RustVec<T>) {
        self.rustVec = rustVec
    }

    internal mutating func next() -> T.SelfRef? {
        let val = rustVec.get(index: index)
        index += 1
        return val
    }
}

extension RustVec: Collection {
    internal typealias Index = Int

    internal func index(after i: Int) -> Int {
        i + 1
    }

    internal subscript(position: Int) -> T.SelfRef {
        self.get(index: UInt(position))!
    }

    internal var startIndex: Int {
        0
    }

    internal var endIndex: Int {
        self.len()
    }
}

extension RustVec: RandomAccessCollection {}

extension UnsafeBufferPointer {
    func toFfiSlice () -> __private__FfiSlice {
        __private__FfiSlice(start: UnsafeMutablePointer(mutating: self.baseAddress), len: UInt(self.count))
    }
}

internal protocol Vectorizable {
    associatedtype SelfRef
    associatedtype SelfRefMut

    static func vecOfSelfNew() -> UnsafeMutableRawPointer;

    static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer)

    static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self)

    static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self>

    static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SelfRef>

    static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SelfRefMut>

    static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<SelfRef>

    static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt
}

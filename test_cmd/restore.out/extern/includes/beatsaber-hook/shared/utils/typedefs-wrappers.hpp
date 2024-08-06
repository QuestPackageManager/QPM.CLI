#pragma once
#include <concepts>
#include <functional>
#include <memory>
#include <mutex>
#include <set>
#include <shared_mutex>
#include <type_traits>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include "base-wrapper-type.hpp"
#include "il2cpp-functions.hpp"
#include "il2cpp-type-check.hpp"
#include "il2cpp-utils-exceptions.hpp"

#if __has_feature(cxx_exceptions)
struct CreatedTooEarlyException : il2cpp_utils::exceptions::StackTraceException {
    CreatedTooEarlyException() : il2cpp_utils::exceptions::StackTraceException("A SafePtr<T> instance was created too early or a necessary GC function was not found!") {}
};
struct NullHandleException : il2cpp_utils::exceptions::StackTraceException {
    NullHandleException() : il2cpp_utils::exceptions::StackTraceException("A SafePtr<T> instance is holding a null handle!") {}
};
struct TypeCastException : il2cpp_utils::exceptions::StackTraceException {
    TypeCastException() : il2cpp_utils::exceptions::StackTraceException("The type could not be cast safely! Check your SafePtr/CountPointer cast calls!") {}
};
#define __SAFE_PTR_NULL_HANDLE_CHECK(handle, ...) \
    if (handle) return __VA_ARGS__;               \
    throw NullHandleException()

#else
#include "utils.h"
#define __SAFE_PTR_NULL_HANDLE_CHECK(handle, ...) \
    if (handle) return __VA_ARGS__;               \
    CRASH_UNLESS(false)
#endif

/// @brief A thread-safe, static type that holds a mapping from addresses to reference counts.
struct Counter {
    /// @brief Adds to the reference count of an address. If the address does not exist, initializes a new entry for it to 1.
    /// @param addr The address to add.
    static void add(void* addr) {
        std::unique_lock lock(mutex);
        auto itr = addrRefCount.find(addr);
        if (itr != addrRefCount.end()) {
            ++itr->second;
        } else {
            addrRefCount.emplace(addr, 1);
        }
    }
    /// @brief Decreases the reference count of an address. If the address has 1 or fewer references, erases it.
    /// @param addr The address to decrease.
    static void remove(void* addr) {
        std::unique_lock lock(mutex);
        auto itr = addrRefCount.find(addr);
        if (itr != addrRefCount.end() && itr->second > 1) {
            --itr->second;
        } else if (itr != addrRefCount.end()) {
            addrRefCount.erase(itr);
        }
    }
    /// @brief Gets the reference count of an address, or 0 if no such address exists.
    /// @param addr The address to get the count of.
    /// @return The reference count of the provided address.
    static size_t get(void* addr) {
        std::shared_lock lock(mutex);
        auto itr = addrRefCount.find(addr);
        if (itr != addrRefCount.end()) {
            return itr->second;
        } else {
            return 0;
        }
    }

   private:
    static std::unordered_map<void*, size_t> addrRefCount;
    static std::shared_mutex mutex;
};

/// @brief Represents a smart pointer that has a reference count, which does NOT destroy the held instance on refcount reaching 0.
/// @tparam T The type to wrap as a pointer.
template <class T>
struct CountPointer {
    /// @brief Default constructor for Count Pointer, defaults to a nullptr, with 0 references.
    explicit CountPointer() : ptr(nullptr) {}
    /// @brief Construct a count pointer from the provided pointer, adding to the reference count (if non-null) for the provided pointer.
    /// @param p The pointer to provide. May be null, which does nothing.
    explicit CountPointer(T* p) : ptr(p) {
        if (p) {
            Counter::add(p);
        }
    }
    /// @brief Copy constructor, copies and adds to the reference count for the held non-null pointer.
    CountPointer(const CountPointer<T>& other) : ptr(other.ptr) {
        if (ptr) {
            Counter::add(ptr);
        }
    }
    /// @brief Move constructor is default, moves the pointer and keeps the reference count the same.
    CountPointer(CountPointer&& other) = default;
    /// @brief Destructor, decreases the ref count for the held non-null pointer.
    ~CountPointer() {
        if (ptr) {
            Counter::remove(ptr);
        }
    }
    /// @brief Gets the reference count held by this pointer.
    /// @return The reference count for this pointer, or 0 if the held pointer is null.
    size_t count() const {
        if (ptr) {
            return Counter::get(ptr);
        }
        return 0;
    }
    /// @brief Emplaces a new pointer into the shared pointer, decreasing the existing ref count as necessary.
    /// @param val The new pointer to replace the currently held one with.
    inline void emplace(T* val) {
        if (val != ptr) {
            if (ptr) {
                Counter::remove(ptr);
            }
            ptr = val;
            if (ptr) {
                Counter::add(ptr);
            }
        }
    }
    /// Assignment operator.
    CountPointer& operator=(T* val) {
        emplace(val);
        return *this;
    }
    /// Dereference operator.
    T& operator*() noexcept {
        if (ptr) {
            return *ptr;
        }
        SAFE_ABORT();
        return *ptr;
    }
    const T& operator*() const noexcept {
        if (ptr) {
            return *ptr;
        }
        SAFE_ABORT();
        return *ptr;
    }
    T* operator->() noexcept {
        if (ptr) {
            return ptr;
        }
        SAFE_ABORT();
        return nullptr;
    }
    T* const operator->() const noexcept {
        if (ptr) {
            return ptr;
        }
        SAFE_ABORT();
        return nullptr;
    }
    constexpr operator bool() const noexcept {
        return ptr != nullptr;
    }
    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// This function may throw TypeCastException or crash.
    /// See try_cast for a version that does not.
    /// @tparam U The type to cast to.
    /// @return A new CountPointer of the cast value.
    template <class U>
    [[nodiscard]] inline CountPointer<U> cast() const {
        // TODO: We currently assume that the first sizeof(void*) bytes of ptr is the klass field.
        // This should hold true for everything except value types.
        auto* k1 = CRASH_UNLESS(classof(U*));
        auto* k2 = *CRASH_UNLESS(reinterpret_cast<Il2CppClass**>(ptr));
        CRASH_UNLESS(k2);
        il2cpp_functions::Init();
        if (il2cpp_functions::class_is_assignable_from(k1, k2)) {
            return CountPointer<U>(reinterpret_cast<U*>(ptr));
        }
#if __has_feature(cxx_exceptions)
        throw TypeCastException();
#else
        SAFE_ABORT();
        return CountPointer<U>();
#endif
    }
    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// @tparam U The type to cast to.
    /// @return A new CountPointer of the cast value, if successful.
    template <class U>
    [[nodiscard]] inline std::optional<CountPointer<U>> try_cast() const noexcept {
        auto* k1 = classof(U*);
        if (!ptr || !k1) {
            return std::nullopt;
        }
        auto* k2 = *reinterpret_cast<Il2CppClass**>(ptr);
        if (!k2) {
            return std::nullopt;
        }
        il2cpp_functions::Init();
        if (il2cpp_functions::class_is_assignable_from(k1, k2)) {
            return CountPointer<U>(reinterpret_cast<U*>(ptr));
        }
        return std::nullopt;
    }

    /// @brief Get the raw pointer. Should ALMOST NEVER BE USED, UNLESS SCOPE GUARANTEES IT DIES BEFORE THIS INSTANCE DOES!
    /// @return The raw pointer saved by this instance.
    constexpr T* const __internal_get() const noexcept {
        return ptr;
    }

   private:
    T* ptr;
};

// TODO: Make an overall Ptr interface type, virtual destructor and *, -> operators
// TODO: Remove all conversion operators? (Basically force people to guarantee lifetime of held instance?)

// Fd UnityEngine.Object
#ifdef HAS_CODEGEN
namespace UnityEngine {
class Object;
}
#endif

template <typename T>
struct SafePtrUnity;

/// @brief Represents a C++ type that wraps a C# pointer that will be valid for the entire lifetime of this instance.
/// This instance must be created at a time such that il2cpp_functions::Init is valid, or else it will throw a CreatedTooEarlyException
/// @tparam T The type of the instance to wrap.
/// @tparam AllowUnity Whether to permit convertible Unity Object types to be wrapped.
template <class T, bool AllowUnity = false>
struct SafePtr {
#ifdef HAS_CODEGEN
    static_assert(!std::is_assignable_v<UnityEngine::Object, T> || AllowUnity, "Don't use Unity types with SafePtr. Ignore this warning by specifying SafePtr<T, true>");
#endif
    /// @brief Default constructor. Should be paired with emplace or = to ensure validity.
    SafePtr() {}
    /// @brief Construct a SafePtr<T> with the provided instance pointer (which may be nullptr).
    /// If you wish to wrap a non-existent pointer (ex, use as a default constructor) see the 0 arg constructor instead.
    SafePtr(T* wrappableInstance) : internalHandle(SafePointerWrapper::New(wrappableInstance)) {}
    /// @brief Construct a SafePtr<T> with the provided wrapper
    SafePtr(T& wrappableInstance)
        requires(il2cpp_utils::has_il2cpp_conversion<T>)
        : internalHandle(SafePointerWrapper::New(wrappableInstance.convert())) {}
    /// @brief Construct a SafePtr<T> with the provided wrapper
    SafePtr(T&& wrappableInstance)
        requires(il2cpp_utils::has_il2cpp_conversion<T>)
        : internalHandle(SafePointerWrapper::New(wrappableInstance.convert())) {}
    /// @brief Construct a SafePtr<T> with the provided reference
    SafePtr(T& wrappableInstance)
        requires(!il2cpp_utils::has_il2cpp_conversion<T>)
        : internalHandle(SafePointerWrapper::New(std::addressof(wrappableInstance))) {}
    /// @brief Move constructor is default, moves the internal handle and keeps reference count the same.
    SafePtr(SafePtr&& other) = default;
    /// @brief Copy constructor copies the HANDLE, that is, the held pointer remains the same.
    /// Note that this means if you modify one SafePtr's held instance, all others that point to the same location will also reflect this change.
    /// In order to avoid a (small) performance overhead, consider using a reference type instead of a value type, or the move constructor instead.
    SafePtr(const SafePtr& other) : internalHandle(other.internalHandle) {}
    /// @brief Destructor. Destroys the internal wrapper type, if necessary.
    /// Aborts if a wrapper type exists and must be freed, yet GC_free does not exist.
    ~SafePtr() {
        if (!internalHandle) {
            // Destructor without an internal handle is trivial
            return;
        }
        // If our internal handle has 1 instance, we need to clean up the instance it points to.
        // Otherwise, some other SafePtr is currently holding a reference to this instance, so keep it around.
        if (internalHandle.count() <= 1) {
            il2cpp_functions::Init();
            #ifdef UNITY_2021
            il2cpp_functions::gc_free_fixed(internalHandle.__internal_get());
            #else
            if (!il2cpp_functions::hasGCFuncs) {
                SAFE_ABORT_MSG("Cannot use SafePtr without GC functions!");
            }
            il2cpp_functions::GC_free(internalHandle.__internal_get());
            #endif
        }
    }

    /// @brief Emplace a new value into this SafePtr, freeing an existing one, if it exists.
    /// @param other The instance to emplace.
    inline void emplace(T& other) {
        this->~SafePtr();
        internalHandle = SafePointerWrapper::New(std::addressof(other));
    }

    /// @brief Emplace a new value into this SafePtr, freeing an existing one, if it exists.
    /// @param other The instance to emplace.
    inline void emplace(T* other) {
        this->~SafePtr();
        internalHandle = SafePointerWrapper::New(other);
    }

    /// @brief Emplace a new pointer into this SafePtr, managing the existing one, if it exists.
    /// @param other The CountPointer to copy during the emplace.
    inline void emplace(CountPointer<T>& other) {
        // Clear existing instance as necessary
        this->~SafePtr();
        // Copy other into handle
        internalHandle = other;
    }

    /// @brief Move an existing CountPointer<T> into this SafePtr, deleting the existing one, if necessary.
    /// @param other The CountPointer to move during this call.
    inline void move(CountPointer<T>& other) {
        // Clear existing instance as necessary
        this->~SafePtr();
        // Move into handle
        internalHandle = std::move(other);
    }

    inline SafePtr<T, AllowUnity>& operator=(T* other) {
        emplace(other);
        return *this;
    }

    inline SafePtr<T, AllowUnity>& operator=(T& other) {
        emplace(other);
        return *this;
    }
    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// This function may throw TypeCastException or NullHandleException or otherwise abort.
    /// See try_cast for a version that does not.
    /// @tparam U The type to cast to.
    /// @tparam AllowUnityPrime Whether the casted SafePtr should allow unity conversions.
    /// @return A new SafePtr of the cast value.
    template <class U, bool AllowUnityPrime = AllowUnity>
    [[nodiscard]] inline SafePtr<U, AllowUnityPrime> cast() const {
        // TODO: We currently assume that the first sizeof(void*) bytes of ptr is the klass field.
        // This should hold true for everything except value types.
        if (!internalHandle) {
#if __has_feature(cxx_exceptions)
            throw NullHandleException();
#else
            SAFE_ABORT();
            return SafePtr<U, AllowUnityPrime>();
#endif
        }
        auto* k1 = CRASH_UNLESS(classof(U*));
        auto* k2 = *CRASH_UNLESS(reinterpret_cast<Il2CppClass**>(internalHandle->instancePointer));
        il2cpp_functions::Init();
        if (il2cpp_functions::class_is_assignable_from(k1, k2)) {
            return SafePtr<U, AllowUnityPrime>(reinterpret_cast<U*>(internalHandle->instancePointer));
        }
#if __has_feature(cxx_exceptions)
        throw TypeCastException();
#else
        SAFE_ABORT();
        return SafePtr<U, AllowUnityPrime>();
#endif
    }
    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// @tparam U The type to cast to.
    /// @tparam AllowUnityPrime Whether the casted SafePtr should allow unity conversions.
    /// @return A new SafePtr of the cast value, if successful.
    template <class U, bool AllowUnityPrime = AllowUnity>
    [[nodiscard]] inline std::optional<SafePtr<U, AllowUnityPrime>> try_cast() const noexcept {
        auto* k1 = classof(U*);
        if (!internalHandle || !internalHandle->instancePointer || k1) {
            return std::nullopt;
        }
        auto* k2 = *reinterpret_cast<Il2CppClass**>(internalHandle->instancePointer);
        if (!k2) {
            return std::nullopt;
        }
        il2cpp_functions::Init();
        if (il2cpp_functions::class_is_assignable_from(k1, k2)) {
            return SafePtr<U, AllowUnityPrime>(reinterpret_cast<U*>(internalHandle->instancePointer));
        }
        return std::nullopt;
    }

    /// @brief Returns false if this is a defaultly constructed SafePtr, true otherwise.
    /// Note that this means that it will return true if it holds a nullptr value explicitly!
    /// This means that you should check yourself before calling anything using the held T*.
    inline bool isHandleValid() const noexcept {
        return static_cast<bool>(internalHandle);
    }

    T* ptr() {
        __SAFE_PTR_NULL_HANDLE_CHECK(internalHandle, internalHandle->instancePointer);
    }

    T const* ptr() const {
        __SAFE_PTR_NULL_HANDLE_CHECK(internalHandle, internalHandle->instancePointer);
    }

    /// @brief Returns false if this is a defaultly constructed SafePtr,
    /// or if the held pointer evaluates to false.
    operator bool() const noexcept {
        return isHandleValid() && ptr();
    }

    /// @brief Dereferences the instance pointer to a reference type of the held instance.
    /// Throws a NullHandleException if there is no internal handle.
    [[nodiscard]] T& operator*() {
        return *ptr();
    }

    [[nodiscard]] const T& operator*() const {
        return *ptr();
    }

    [[nodiscard]] T* const operator->() const {
        return const_cast<T*>(ptr());
    }

    /// @brief Explicitly cast this instance to a T*.
    /// Note, however, that the lifetime of this returned T* is not longer than the lifetime of this instance.
    /// Consider passing a SafePtr reference or copy instead.
    [[nodiscard]] explicit operator T* const() const {
        return const_cast<T*>(ptr());
    }

   private:
    friend struct SafePtrUnity<T>;

    struct SafePointerWrapper {
        static SafePointerWrapper* New(T* instance) {
            il2cpp_functions::Init();
            // It should be safe to assume that gc_alloc_fixed returns a non-null pointer. If it does return null, we have a pretty big issue.
            static constexpr auto sz = sizeof(SafePointerWrapper);

            #ifdef UNITY_2021
            auto* wrapper = reinterpret_cast<SafePointerWrapper*>(il2cpp_functions::gc_alloc_fixed(sz));

            #else

            if (!il2cpp_functions::hasGCFuncs) {
                #if __has_feature(cxx_exceptions)
                throw CreatedTooEarlyException();
                #else
                SAFE_ABORT_MSG("Cannot use a SafePtr this early/without GC functions!");
                #endif
            }
            auto* wrapper = reinterpret_cast<SafePointerWrapper*>(il2cpp_functions::GarbageCollector_AllocateFixed(sz, nullptr));
            #endif

            CRASH_UNLESS(wrapper);
            wrapper->instancePointer = instance;
            return wrapper;
        }
        // Must be explicitly GC freed and allocated
        SafePointerWrapper() = delete;
        ~SafePointerWrapper() = delete;
        T* instancePointer;
    };
    CountPointer<SafePointerWrapper> internalHandle;
};

#if __has_feature(cxx_exceptions)
#define __SAFE_PTR_UNITY_NULL_HANDLE_CHECK(...) \
    if (isAlive()) return __VA_ARGS__;          \
    throw NullHandleException()

#else
#include "utils.h"
#define __SAFE_PTR_UNITY_NULL_HANDLE_CHECK(...) \
    if (isAlive()) return __VA_ARGS__;          \
    CRASH_UNLESS(false)
#endif

template <typename T>
struct SafePtrUnity : public SafePtr<T, true> {
   private:
    using Parent = SafePtr<T, true>;

   public:
    SafePtrUnity() = default;

    SafePtrUnity(T* wrappableInstance) : Parent(wrappableInstance) {}
    SafePtrUnity(T& wrappableInstance) : Parent(wrappableInstance) {}
    SafePtrUnity(Parent&& p) : Parent(p) {}
    SafePtrUnity(Parent const& p) : Parent(p) {}

    SafePtrUnity(SafePtrUnity&& p) : Parent(p) {}
    SafePtrUnity(SafePtrUnity const& p) : Parent(p) {}

    T* ptr() {
        __SAFE_PTR_UNITY_NULL_HANDLE_CHECK(Parent::internalHandle->instancePointer);
    }

    T const* ptr() const {
        __SAFE_PTR_UNITY_NULL_HANDLE_CHECK(Parent::internalHandle->instancePointer);
    }

    inline SafePtrUnity<T>& operator=(T* other) {
        Parent::emplace(other);
        return *this;
    }

    inline SafePtrUnity<T>& operator=(T& other) {
        Parent::emplace(other);
        return *this;
    }

    /// @brief Explicitly cast this instance to a T*.
    /// Note, however, that the lifetime of this returned T* is not longer than the lifetime of this instance.
    /// Consider passing a SafePtrUnity reference or copy instead.
    explicit operator T* const() const {
        return const_cast<T*>(ptr());
    }

    T* const operator->() {
        return const_cast<T*>(ptr());
    }

    T* const operator->() const {
        return ptr();
    }

    T& operator*() {
        return *ptr();
    }

    T const& operator*() const {
        return *ptr();
    }

    operator bool() const {
        return isAlive();
    }

    template <typename U = T>
        requires(std::is_assignable_v<T, U> || std::is_same_v<T, U>) bool
    operator==(SafePtrUnity<U> const& other) const {
        if (!other.isAlive() || !isAlive()) {
            return other.isAlive() == isAlive();
        }

        return static_cast<T*>(other.internalHandle) == static_cast<T*>(Parent::ptr());
    }

    template <typename U = T>
    bool operator==(U const* other) const {
        if (!other || !isAlive()) {
            return static_cast<bool>(other) == isAlive();
        }

        return static_cast<T*>(other) == static_cast<T*>(Parent::ptr());
    }

    inline bool isAlive() const {
#ifdef HAS_CODEGEN
        return static_cast<bool>(Parent::internalHandle) && (Parent::ptr()) && Parent::ptr()->m_CachedPtr;
#else
        // offset yay
        // the offset as specified in the codegen header of [m_CachedPtr] is 0x10
        // which is also the first field of the instance UnityEngine.Object
        return static_cast<bool>(Parent::internalHandle) && (Parent::ptr()) && *reinterpret_cast<void* const*>(reinterpret_cast<uint8_t const*>(Parent::ptr()) + 0x10);
#endif
    }
};

/// @brief Represents a pointer that may be GC'd, but will notify you when it has.
/// Currently unimplemented, requires a hook into all GC frees/collections
template <class T>
struct WeakPtr {};

template <template <typename> typename Container, typename Item>
concept is_valid_container = requires(Container<Item> coll, Item item) {
                                 coll.erase(item);
                                 coll.emplace(item);
                                 coll.clear();
                                 coll.begin();
                                 coll.end();
                                 coll.size();
                             };

template <class T>
struct AbstractFunction;

template <typename R, typename T, typename... TArgs>
struct AbstractFunction<R(T*, TArgs...)> {
    virtual T* instance() const = 0;
    virtual void* ptr() const = 0;

    virtual R operator()(TArgs... args) const noexcept = 0;
    virtual ~AbstractFunction() = default;
};

template <class T>
struct FunctionWrapper;

template <typename R, typename... TArgs>
struct FunctionWrapper<R (*)(TArgs...)> : AbstractFunction<R(void*, TArgs...)> {
    void* instance() const override {
        return nullptr;
    }
    void* ptr() const override {
        return reinterpret_cast<void*>(held);
    }
    R (*held)(TArgs...);
    template <class F>
    FunctionWrapper(F&& f) : held(f) {}
    R operator()(TArgs... args) const noexcept override {
        if constexpr (std::is_same_v<R, void>) {
            held(args...);
        } else {
            return held(args...);
        }
    }
};

template <typename R, typename T, typename... TArgs>
struct FunctionWrapper<R (T::*)(TArgs...)> : AbstractFunction<R(void*, TArgs...)> {
    void* instance() const override {
        return _instance;
    }
    void* ptr() const override {
        using fptr = R (T::*)(TArgs...);
        union dat {
            fptr wrapper;
            void* data;
        };
        dat d{ .wrapper = held };
        return d.data;
    }
    R (T::*held)(TArgs...);
    T* _instance;
    template <class F>
    FunctionWrapper(F&& f, T* inst) : held(f), _instance(inst) {}
    R operator()(TArgs... args) const noexcept override {
        if constexpr (std::is_same_v<R, void>) {
            (reinterpret_cast<T*>(_instance)->*held)(args...);
        } else {
            return (reinterpret_cast<T*>(_instance)->*held)(args...);
        }
    }
};

template <typename R, typename... TArgs>
struct FunctionWrapper<std::function<R(TArgs...)>> : AbstractFunction<R(void*, TArgs...)> {
    [[nodiscard]] void* instance() const override {
        return nullptr;
    }
    [[nodiscard]] void* ptr() const override {
        return handle;
    }
    std::function<R(TArgs...)> const held;
    void* handle;

    FunctionWrapper(std::function<R(TArgs...)> const& f) : held(f), handle(const_cast<void*>(reinterpret_cast<const void*>(&f))) {}
    R operator()(TArgs... args) const noexcept override {
        if constexpr (std::is_same_v<R, void>) {
            held(args...);
        } else {
            return held(args...);
        }
    }
};

namespace std {
template <typename R, typename T, typename... TArgs>
struct hash<AbstractFunction<R(T*, TArgs...)>> {
    std::size_t operator()(const AbstractFunction<R(T*, TArgs...)>& obj) const noexcept {
        auto seed = std::hash<void*>{}(obj.instance());
        return seed ^ std::hash<void*>{}(reinterpret_cast<void*>(obj.ptr())) + 0x9e3779b9 + (seed << 6) + (seed >> 2);
        return seed;
    }
};
}  // namespace std

template <typename R, typename T, typename... TArgs>
bool operator==(const AbstractFunction<R(T*, TArgs...)>& a, const AbstractFunction<R(T*, TArgs...)>& b) {
    return a.instance() == b.instance() && a.ptr() == b.ptr();
}

template <typename R, typename T, typename... TArgs>
bool operator<(const AbstractFunction<R(T*, TArgs...)>& a, const AbstractFunction<R(T*, TArgs...)>& b) {
    return a.ptr() < b.ptr();
}

template <class T>
struct ThinVirtualLayer;

template <typename R, typename T, typename... TArgs>
struct std::hash<ThinVirtualLayer<R(T*, TArgs...)>>;

template <typename R, typename T, typename... TArgs>
struct ThinVirtualLayer<R(T*, TArgs...)> {
    friend struct std::hash<ThinVirtualLayer<R(T*, TArgs...)>>;

   private:
    std::shared_ptr<AbstractFunction<R(T*, TArgs...)>> func;

   public:
    ThinVirtualLayer(R (*ptr)(TArgs...)) : func(new FunctionWrapper<R (*)(TArgs...)>(ptr)) {}
    template <class F, typename Q>
    ThinVirtualLayer(F&& f, Q* inst) : func(new FunctionWrapper<R (Q::*)(TArgs...)>(f, inst)) {}
    template <class F>
    ThinVirtualLayer(F&& f) : func(new FunctionWrapper<std::function<R(TArgs...)>>(f)) {}

    R operator()(TArgs... args) const noexcept {
        (*func)(args...);
    }
    void* instance() const {
        return func->instance();
    }
    void* ptr() const {
        return func->ptr();
    }

    bool operator==(const ThinVirtualLayer<R(T*, TArgs...)> other) const {
        return *func == (*other.func);
    }
    bool operator<(const ThinVirtualLayer<R(T*, TArgs...)> other) const {
        return *func < *other.func;
    }
};

namespace std {
template <typename R, typename T, typename... TArgs>
struct hash<ThinVirtualLayer<R(T*, TArgs...)>> {
    std::size_t operator()(const ThinVirtualLayer<R(T*, TArgs...)>& obj) const noexcept {
        return std::hash<AbstractFunction<R(T*, TArgs...)>>{}(*obj.func);
    }
};
}  // namespace std

// TODO: Make a version of this for C# delegates?
// TODO: Also require the function type to be invokable and all that
template <template <typename> typename Container, typename... TArgs>
    requires(is_valid_container<Container, ThinVirtualLayer<void(void*, TArgs...)>>)
class BasicEventCallback {
   private:
    using functionType = ThinVirtualLayer<void(void*, TArgs...)>;
    Container<functionType> callbacks;

   public:
    void invoke(TArgs... args) const {
#ifndef NO_EVENT_CALLBACK_INVOKE_SAFETY
        // copy the callbacks so an unsubscribe during invoke of the container doesn't cause UB
        auto cbs = callbacks;
        for (auto& callback : cbs) {
            callback(args...);
        }
#else
        // no safety requested, just run it from the callbacks as
        for (auto& callback : callbacks) {
            callback(args...);
        }
#endif
    }

    BasicEventCallback& operator+=(ThinVirtualLayer<void(void*, TArgs...)> callback) {
        callbacks.emplace(std::move(callback));
        return *this;
    }

    BasicEventCallback& operator-=(void (*callback)(TArgs...)) {
        removeCallback(callback);
        return *this;
    }

    BasicEventCallback& operator-=(ThinVirtualLayer<void(void*, TArgs...)> callback) {
        callbacks.erase(callback);
        return *this;
    }

    template <typename T>
    BasicEventCallback& operator-=(void (T::*callback)(TArgs...)) {
        removeCallback(callback);
        return *this;
    }

    void addCallback(void (*callback)(TArgs...)) {
        callbacks.emplace(callback);
    }
    // The instance provide here should have lifetime > calls to invoke.
    // If the provided instance dies before this instance, or before invoke is called, invoke will crash.
    template <typename T>
    void addCallback(void (T::*callback)(TArgs...), T* inst) {
        callbacks.emplace(callback, inst);
    }

    void removeCallback(void (*callback)(TArgs...)) {
        callbacks.erase(callback);
    }

    template <typename T>
    void removeCallback(void (T::*callback)(TArgs...)) {
        // Removal of member functions is expensive because we need to remove all member functions regardless of instance
        for (auto itr = callbacks.begin(); itr != callbacks.end();) {
            union dat {
                decltype(callback) wrapper;
                void* data;
            };
            dat d{ .wrapper = callback };
            if (itr->ptr() == d.data) {
                itr = callbacks.erase(itr);
            } else {
                ++itr;
            }
        }
    }
    auto size() const {
        return callbacks.size();
    }
    void clear() {
        callbacks.clear();
    }
};
#undef __SAFE_PTR_NULL_HANDLE_CHECK

template <typename Item>
using default_ordered_set = std::set<Item>;

template <typename Item>
using default_unordered_set = std::unordered_set<Item>;

// Good default for most
template <typename... TArgs>
using EventCallback = BasicEventCallback<default_ordered_set, TArgs...>;

template <typename... TArgs>
using UnorderedEventCallback = BasicEventCallback<default_unordered_set, TArgs...>;

#pragma once
#include "il2cpp-type-check.hpp"
#include "type-concepts.hpp"

/// @brief Represents a byref parameter.
/// This is REQUIRED for codegen invokes, as RunMethodThrow can't tell the difference between a reference parameter and a byref on constexpr time.
/// This wraps a reference and the sizeof it must be the size of a pointer.
template<class T>
requires (!std::is_reference_v<T>)
struct ByRef {
    constexpr ByRef(T& val) noexcept : heldRef(val) {}
    constexpr ByRef(void* val) noexcept : heldRef(*reinterpret_cast<T*>(val)) {}

    T& heldRef;
    constexpr T* operator->() noexcept {
        return &heldRef;
    }
    constexpr T& operator*() noexcept {
        return heldRef;
    }
    constexpr T const& operator*() const noexcept {
        return heldRef;
    }
    constexpr const T* operator->() const noexcept {
        return &heldRef;
    }
    ByRef<T>& operator =(T const&& other) {
        heldRef = other;
        return *this;
    }

    constexpr void* convert() const noexcept {
        return (void*)&heldRef;
    }
    static_assert(sizeof(T*) == sizeof(void*));
};
MARK_GEN_REF_T(ByRef);

namespace il2cpp_utils {
    template<typename T>
    concept has_no_arg_class = requires {
        { ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<T>::get() } -> il2cpp_utils::convertible_to<Il2CppClass*>;
    };
}

// Type specializations for byref specifics
// We do not need il2cpp_no_arg_class specialization for ByRef, since it will never get to that point.

template<typename T>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_type<ByRef<T>> {
    static inline const Il2CppType* get() {
        // if T has an arg class, look it up by the this_arg
        if constexpr (has_no_arg_class<T>) {
            static auto* typ = &::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<T>::get()->this_arg;
            return typ;
        } else {
            // if not try using the type getter directly for T
            return ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_type<T>::get();
        }
    }
};

template<typename T>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_arg_type<ByRef<T>> {
    static inline const Il2CppType* get([[maybe_unused]] ByRef<T> arg) {
        // ByRef types should NOT perform argument based lookups, since they could be holding a null reference.
        // ex: out
        // Instead, we should deduce the type from our held type alone.
        return ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_type<ByRef<T>>::get();
    }
};

// Creates a ByRef type to wrap a reference
#define byref(...) (ByRef(__VA_ARGS__))

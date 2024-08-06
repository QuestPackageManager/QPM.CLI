#pragma once

#include "type-concepts.hpp"

#include "il2cpp-tabledefs.h"
#include "il2cpp-type-check.hpp"

#include "size-concepts.hpp"

namespace il2cpp_utils {
#pragma region boxing
template <typename T>
BS_HOOKS_HIDDEN Il2CppObject* Box(T);

template <typename T>
BS_HOOKS_HIDDEN Il2CppObject* Box(T*);

template <>
BS_HOOKS_HIDDEN constexpr Il2CppObject* Box<Il2CppObject*>(Il2CppObject* t) {
    return t;
}

template <typename T>
    requires(!std::is_pointer_v<T> && !std::is_base_of_v<Il2CppObject, T>)
BS_HOOKS_HIDDEN Il2CppObject* Box(T* t) {
    return il2cpp_functions::value_box(classof(T), t);
}

template <::il2cpp_utils::has_il2cpp_conversion T>
    requires(!std::is_base_of_v<Il2CppObject, T>)
BS_HOOKS_HIDDEN Il2CppObject* Box(T t) {
    return il2cpp_functions::value_box(classof(T), t.convert());
}

template <::il2cpp_utils::has_il2cpp_conversion T>
    requires(!std::is_base_of_v<Il2CppObject, T>)
BS_HOOKS_HIDDEN Il2CppObject* Box(T* t) {
    return il2cpp_functions::value_box(classof(T), t->convert());
}
#pragma endregion  // boxing

#pragma region unboxing
template <typename T>
BS_HOOKS_HIDDEN T Unbox(Il2CppObject* t) {
    return *static_cast<T*>(il2cpp_functions::object_unbox(t));
}

template <::il2cpp_utils::il2cpp_reference_type_wrapper T>
BS_HOOKS_HIDDEN T Unbox(Il2CppObject* t) {
    return T(t);
}

template <::il2cpp_utils::il2cpp_reference_type_pointer T>
BS_HOOKS_HIDDEN T Unbox(Il2CppObject* t) {
    return reinterpret_cast<T>(t);
}
#pragma endregion  // unboxing
}  // namespace il2cpp_utils

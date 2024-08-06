#pragma once

#include "type-concepts.hpp"

namespace il2cpp_utils {
namespace {
    /// @brief any builtins will just have the same size as in cpp
    template<typename T>
    struct il2cpp_size {
        static constexpr auto value = sizeof(T);
    };

    /// @brief the expectation is that val types do match their size
    template<::il2cpp_utils::il2cpp_value_type T>
    struct il2cpp_size<T> {
        static constexpr auto value = T::VALUE_TYPE_SIZE;
    };

    /// @brief the expectation is that ref types are not going to match size
    template<::il2cpp_utils::il2cpp_reference_type T>
    struct il2cpp_size<T> {
        static constexpr auto value = sizeof(void*);
    };

    /// @brief shorthand to get the size value
    template<typename T>
    static constexpr auto il2cpp_size_v = il2cpp_size<T>::value;

    /// @brief extracting the instance size of an il2cpp type wrapper
    template<typename T>
    struct il2cpp_instance_size;

    /// @brief ref types should declare a static member __IL2CPP_REFERENCE_TYPE_SIZE that contains their instance size
    template<::il2cpp_utils::il2cpp_reference_type_wrapper T>
    struct il2cpp_instance_size<T> {
        static constexpr auto value = T::__IL2CPP_REFERENCE_TYPE_SIZE;
    };

    /// @brief ref types should declare a static member __IL2CPP_REFERENCE_TYPE_SIZE that contains their instance size
    template<::il2cpp_utils::il2cpp_reference_type_pointer T>
    struct il2cpp_instance_size<T> {
        static constexpr auto value = std::remove_pointer_t<T>::__IL2CPP_REFERENCE_TYPE_SIZE;
    };

    /// @brief VT wrappers always define their instance size in the lowermost type
    template<::il2cpp_utils::il2cpp_value_type T>
    struct il2cpp_instance_size<T> {
        static constexpr auto value = T::VALUE_TYPE_SIZE;
    };

    /// @brief type trait that checks whether the il2cpp size matches the cpp size
    template<typename T>
    struct is_il2cpp_size_safe {
        static constexpr bool value = il2cpp_size_v<T> == sizeof(T);
    };

    /// @brief shorthand to get the size check value
    template<typename T>
    static constexpr bool is_il2cpp_size_safe_v = is_il2cpp_size_safe<T>::value;
    }  // namespace il2cpp_utils
}

#define il2cpp_sizeof(...) (::il2cpp_utils::il2cpp_size<__VA_ARGS__>::value)
#define il2cpp_safe(...) (::il2cpp_utils::is_il2cpp_size_safe_v<__VA_ARGS__>)
#define il2cpp_instance_sizeof(...) (::il2cpp_utils::il2cpp_instance_size<__VA_ARGS__>::value)

#pragma once
#include <type_traits>
#include <concepts>
#include <cstdint>
#include "il2cpp-type-check.hpp"
#include "type-concepts.hpp"

namespace bs_hook {
    /// @brief Represents the most basic wrapper type.
    /// All other wrapper types should inherit from this or otherwise satisfy the constraint above.
    struct Il2CppWrapperType {
        constexpr explicit Il2CppWrapperType(void* i) noexcept : instance(i) {}
        constexpr Il2CppWrapperType(Il2CppWrapperType const& other) = default;
        constexpr Il2CppWrapperType(Il2CppWrapperType && other) = default;
        constexpr Il2CppWrapperType& operator=(Il2CppWrapperType const& other) = default;
        constexpr Il2CppWrapperType& operator=(Il2CppWrapperType && other) = default;

        constexpr void* convert() const noexcept {
            return const_cast<void*>(instance);
        }

        Il2CppObject* operator ->() const noexcept { return const_cast<Il2CppObject*>(static_cast<Il2CppObject const*>(instance)); }
        operator Il2CppObject*() const noexcept { return const_cast<Il2CppObject*>(static_cast<Il2CppObject const*>(instance)); }

        protected:
        void* instance;
    };
    static_assert(il2cpp_utils::has_il2cpp_conversion<Il2CppWrapperType>);
}

NEED_NO_BOX(bs_hook::Il2CppWrapperType);
DEFINE_IL2CPP_DEFAULT_TYPE(bs_hook::Il2CppWrapperType, object);

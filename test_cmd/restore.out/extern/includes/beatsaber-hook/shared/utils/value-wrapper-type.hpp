#pragma once

#include <cstddef>
#include <array>
#include "type-concepts.hpp"
#include "il2cpp-type-check.hpp"
#include "il2cpp-functions.hpp"

namespace bs_hook {
    template<std::size_t sz>
    struct ValueTypeWrapper {
        static constexpr auto VALUE_TYPE_SIZE = sz;

        constexpr explicit ValueTypeWrapper(std::array<std::byte, VALUE_TYPE_SIZE> i) noexcept : instance(std::move(i)) {}
        void* convert() const noexcept { return const_cast<void*>(static_cast<const void*>(instance.data())); }

        constexpr ValueTypeWrapper() = default;
        ~ValueTypeWrapper() = default;

        constexpr ValueTypeWrapper(ValueTypeWrapper&&) = default;
        constexpr ValueTypeWrapper& operator=(ValueTypeWrapper&& o) {
            instance = std::move(o.instance);
            return *this;
        }

        constexpr ValueTypeWrapper(ValueTypeWrapper const&) = default;
        constexpr ValueTypeWrapper& operator=(ValueTypeWrapper const& o) {
            instance = o.instance;
            return *this;
        }

        std::array<std::byte, sz> instance;
    };

    /// @brief struct to pass a pointer to a value type into a method
    struct VTPtr {
        template<std::size_t sz>
        VTPtr(ValueTypeWrapper<sz>& v) : instance(&v) {};

        explicit VTPtr(void* i) : instance(i) {};
        void* convert() const { return const_cast<void*>(instance); }

        void* instance;
    };

}

template<>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<::bs_hook::VTPtr> {
    static inline Il2CppClass* get() {
        auto enumClass = il2cpp_utils::GetClassFromName("System", "ValueType");
        static auto ptrKlass = il2cpp_functions::il2cpp_Class_GetPtrClass(enumClass);
        return ptrKlass;
    }
};

template<std::size_t sz>
struct ::il2cpp_utils::ValueTypeTrait<::bs_hook::ValueTypeWrapper<sz>> {
    constexpr static bool value = true;
};

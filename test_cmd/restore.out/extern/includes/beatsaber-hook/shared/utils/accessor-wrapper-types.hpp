#pragma once
#include <cstddef>
#include <cstdint>
#include <string_view>
#include <string>
#include <array>
#include <algorithm>

#include "il2cpp-utils-properties.hpp"
#include "il2cpp-utils-fields.hpp"
#include "typedefs-object.hpp"

namespace bs_hook {
    namespace internal {
        template<std::size_t sz>
        struct NTTPString {
            constexpr NTTPString(char const(&n)[sz]) : data{} {
                std::copy_n(n, sz, data.begin());
            }
            std::array<char, sz> data;
        };

        /* Anything that's not a wrapper type */
        template<typename T, bool is_const>
        struct MakeWrapperPtr {
            using type = std::conditional_t<is_const, std::add_pointer_t<std::add_const_t<T>>, std::add_pointer_t<T>>;
        };

        /* Wrapper Types */
        template<typename T, bool is_const>
        requires(il2cpp_utils::has_il2cpp_conversion<T>)
        struct MakeWrapperPtr<T, is_const> {
            using type = T;
        };

        /* Anything that's not a wrapper type */
        template<typename T, bool is_const> 
        struct MakeWrapperRef {
            using type = std::conditional_t<is_const, T const&, T&>;
        };

        /* Wrapper Types */
        template<typename T, bool is_const> 
        requires(il2cpp_utils::has_il2cpp_conversion<T>)
        struct MakeWrapperRef<T, is_const> {
            using type = T;
        };
    }

    struct PropertyException : public il2cpp_utils::exceptions::StackTraceException {
        using StackTraceException::StackTraceException;
    };



    struct FieldException : public il2cpp_utils::exceptions::StackTraceException {
        using StackTraceException::StackTraceException;
    };
    // TODO: Note that these types are not safe to be passed into RunMethod by themselves (or generic functions)
    // This is because they are wrapper over T, as opposed to being analogous to T.
    // We cannot EASILY solve this using wrapper types, because we could be holding a value type as T.
    // This makes things tricky and we should come back to this when we are confident we can handle this case correctly.

    template<internal::NTTPString name, class T, bool get, bool set>
    /// @brief Represents a InstanceProperty on a wrapper type. Forwards to calling the get/set methods where applicable.
    struct InstanceProperty;
    
    template<internal::NTTPString name, class T>
    struct InstanceProperty<name, T, true, false> {
        explicit constexpr InstanceProperty(void* inst) noexcept : instance(inst) {}
        operator T() const {
            auto res = il2cpp_utils::GetPropertyValue<T, false>(reinterpret_cast<Il2CppObject*>(const_cast<void*>(instance)), name.data.data());
            if (!res) throw PropertyException(std::string("Failed to get instance property: ") + name.data.data());
            return *res;
        }

        inline auto operator ->() const {
            return this->operator T();
        } 

        inline auto operator *() const {
            return this->operator T();
        }

        inline auto v() const {
            return this->operator T();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator T()(std::forward<Targs...>(args...)); }

        private:
        void* instance;
    };

    template<internal::NTTPString name, class T>
    struct InstanceProperty<name, T, false, true> {
        explicit constexpr InstanceProperty(void* inst) noexcept : instance(inst) {}
        template<class U>
#ifdef HAS_CODEGEN
        requires(std::is_convertible_v<U, T>)
#endif
        InstanceProperty& operator=(U&& t) {
            auto val = static_cast<Il2CppObject*>(instance);
            auto res = il2cpp_utils::SetPropertyValue<false>(val, name.data.data(), std::forward<decltype(t)>(t));
            if (!res) throw PropertyException(std::string("Failed to set instance property: ") + name.data.data());
            return *this;
        }

        private:
        void* instance;
    };

#define BINARY_OPERATOR_OP_EQ_PROP(op)      \
template<typename U>                        \
auto& operator op##=(U&& rhs) {             \
    auto temp = this->operator T();         \
    return this->operator=(temp op##= rhs); \
}

    template<internal::NTTPString name, class T>
    struct InstanceProperty<name, T, true, true> {
        explicit constexpr InstanceProperty(void* inst) noexcept : instance(inst) {}
        operator T() const {
            auto res = il2cpp_utils::GetPropertyValue<T, false>(reinterpret_cast<Il2CppObject*>(const_cast<void*>(instance)), name.data.data());
            if (!res) throw PropertyException(std::string("Failed to get instance property: ") + name.data.data());
            return *res;
        }
        template<class U>
        InstanceProperty& operator=(U&& t) {
            auto val = static_cast<Il2CppObject*>(instance);
            auto res = il2cpp_utils::SetPropertyValue<false>(val, name.data.data(), std::forward<decltype(t)>(t));
            if (!res) throw PropertyException(std::string("Failed to set instance property: ") + name.data.data());
            return *this;
        }

        inline auto operator ->() const {
            return this->operator T();
        }

        inline auto operator *() const {
            return this->operator T();
        }

        inline auto v() const {
            return this->operator T();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator T()(std::forward<Targs...>(args...)); }

        auto& operator ++() { return this->operator=(++this->operator T()); }
        auto& operator ++(int) { return this->operator=(this->operator T()++); }
        auto& operator --() { return this->operator=(--this->operator T()); }
        auto& operator --(int) { return this->operator=(this->operator T()--); }

        /* These operators forward to the ones on the underlying type */
        BINARY_OPERATOR_OP_EQ_PROP(+);
        BINARY_OPERATOR_OP_EQ_PROP(-);
        BINARY_OPERATOR_OP_EQ_PROP(*);
        BINARY_OPERATOR_OP_EQ_PROP(/);
        BINARY_OPERATOR_OP_EQ_PROP(%);
        BINARY_OPERATOR_OP_EQ_PROP(&);
        BINARY_OPERATOR_OP_EQ_PROP(|);
        BINARY_OPERATOR_OP_EQ_PROP(^);
        BINARY_OPERATOR_OP_EQ_PROP(<<);
        BINARY_OPERATOR_OP_EQ_PROP(>>);

        private:
        void* instance;
    };


    template<class T, internal::NTTPString name, bool get, bool set, auto klass_resolver>
    struct StaticProperty;

    template<class T, internal::NTTPString name, auto klass_resolver>
    struct StaticProperty<T, name, true, false, klass_resolver> {
        operator T() const {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static property with name: ") + name.data.data() + " is null!");
            auto res = il2cpp_utils::GetPropertyValue<T, false>(klass, name.data.data());
            if (!res) throw PropertyException(std::string("Failed to get static property: ") + name.data.data());
            return *res;
        }

        inline auto operator ->() const {
            return this->operator T();
        }

        inline auto operator *() const {
            return this->operator T();
        }

        inline auto v() const {
            return this->operator T();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator T()(std::forward<Targs...>(args...)); }
    };

    template<class T, internal::NTTPString name, auto klass_resolver>
    struct StaticProperty<T, name, false, true, klass_resolver> {
        template<class U>
#ifdef HAS_CODEGEN
        requires(std::is_convertible_v<U, T>)
#endif
        StaticProperty& operator=(U&& value) {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static property with name: ") + name.data.data() + " is null!");
            auto res = il2cpp_utils::SetPropertyValue<false>(klass, name.data.data(), std::forward<decltype(value)>(value));
            if (!res) throw PropertyException(std::string("Failed to set static property: ") + name.data.data());
            return *this;
        }
    };

    template<class T, internal::NTTPString name, auto klass_resolver>
    struct StaticProperty<T, name, true, true, klass_resolver> {
        operator T() const {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static property with name: ") + name.data.data() + " is null!");
            auto res = il2cpp_utils::GetPropertyValue<T, false>(klass, name.data.data());
            if (!res) throw PropertyException(std::string("Failed to get static property: ") + name.data.data());
            return *res;
        }

        template<class U>
        StaticProperty& operator=(U&& value) {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static property with name: ") + name.data.data() + " is null!");
            auto res = il2cpp_utils::SetPropertyValue<false>(klass, name.data.data(), std::forward<decltype(value)>(value));
            if (!res) throw PropertyException(std::string("Failed to set static property: ") + name.data.data());
            return *this;
        }

        inline auto operator ->() const {
            return this->operator T();
        }

        inline auto operator *() const {
            return this->operator T();
        }

        inline auto v() const {
            return this->operator T();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator T()(std::forward<Targs...>(args...)); }

        auto& operator ++() { return this->operator=(++this->operator T()); }
        auto& operator ++(int) { return this->operator=(this->operator T()++); }
        auto& operator --() { return this->operator=(--this->operator T()); }
        auto& operator --(int) { return this->operator=(this->operator T()--); }

        BINARY_OPERATOR_OP_EQ_PROP(+);
        BINARY_OPERATOR_OP_EQ_PROP(-);
        BINARY_OPERATOR_OP_EQ_PROP(*);
        BINARY_OPERATOR_OP_EQ_PROP(/);
        BINARY_OPERATOR_OP_EQ_PROP(%);
        BINARY_OPERATOR_OP_EQ_PROP(&);
        BINARY_OPERATOR_OP_EQ_PROP(|);
        BINARY_OPERATOR_OP_EQ_PROP(^);
        BINARY_OPERATOR_OP_EQ_PROP(<<);
        BINARY_OPERATOR_OP_EQ_PROP(>>);
    };

#undef BINARY_OPERATOR_OP_EQ_PROP

    template<class T, std::size_t offset, bool is_const = true>
    requires(!std::is_reference_v<T>)
    struct InstanceField {
        protected:
        using Ref = typename internal::MakeWrapperRef<T, is_const>::type;
        using Ptr = typename internal::MakeWrapperPtr<T, is_const>::type;
        void* getAtOffset() const {
            return reinterpret_cast<uint8_t*>(const_cast<void*>(instance)) + offset;
        }
        public:
        explicit constexpr InstanceField(void* inst) noexcept : instance(inst) {}
        operator Ref () const {
            if (instance == nullptr) throw NullException("Instance field access failed at offset: " + std::to_string(offset) + " because instance was null!");
            // No wbarrier required for unilateral gets
            if constexpr (il2cpp_utils::has_il2cpp_conversion<T>) {
                // Handle wrapper types differently
                return T(*reinterpret_cast<void**>(getAtOffset()));
            }
            return *reinterpret_cast<T*>(getAtOffset());
        }

        Ptr operator ->() const {
            if (instance == nullptr) throw NullException("Instance field access failed at offset: " + std::to_string(offset) + " because instance was null!");
            if constexpr (il2cpp_utils::has_il2cpp_conversion<T>) {
                return this->operator T();
            }
            return const_cast<Ptr>(reinterpret_cast<T*>(getAtOffset()));
        } 

        inline auto operator *() const {
            return this->operator Ref();
        }

        inline auto v() const {
            return this->operator Ref();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator Ref()(std::forward<Targs...>(args...)); }

        private:
        void* instance;
    };

#define BINARY_OPERATOR_OP_EQ_FIELD(op)     \
template<typename U>                        \
auto& operator op##=(U&& rhs) {             \
    this->operator Ref() op##= rhs;         \
    return *this;                           \
}

    template<class T, std::size_t offset>
    struct AssignableInstanceField : public InstanceField<T, offset, false> {
        using InstanceField<T, offset, false>::InstanceField;
        template<class U>
#ifdef HAS_CODEGEN
        requires(std::is_convertible_v<U, T>)
#endif
        AssignableInstanceField& operator=(U&& t) {
            if (instance == nullptr) throw NullException("Instance field assignment failed at offset: " + std::to_string(offset) + " because instance was null!");

            if constexpr (il2cpp_utils::has_il2cpp_conversion<T>) {
                // We only do this if we are a wrapper type!
                il2cpp_functions::Init();
                // instance is actually unused for wbarrier, wbarrier call performs the assignment
                il2cpp_functions::gc_wbarrier_set_field(instance, reinterpret_cast<void**>(InstanceField<T, offset, false>::getAtOffset()), t.convert());
            } else {
                // No wbarrier for types that are not wrapper types
                // TODO: Value types ALSO need a wbarrier, but for the whole size of themselves.
                // We need to xref trace to find the correct wbarrier set in this case, or call the set_field directly...
                // Which is a bit of a pain.
                *reinterpret_cast<T*>(InstanceField<T, offset, false>::getAtOffset()) = t;
            }
            return *this;
        }

        auto& operator ++() { return this->operator=(++this->operator Ref()); }
        auto& operator ++(int) { return this->operator=(this->operator Ref()++); }
        auto& operator --() { return this->operator=(--this->operator Ref()); }
        auto& operator --(int) { return this->operator=(this->operator Ref()--); }

        BINARY_OPERATOR_OP_EQ_FIELD(+);
        BINARY_OPERATOR_OP_EQ_FIELD(-);
        BINARY_OPERATOR_OP_EQ_FIELD(*);
        BINARY_OPERATOR_OP_EQ_FIELD(/);
        BINARY_OPERATOR_OP_EQ_FIELD(%);
        BINARY_OPERATOR_OP_EQ_FIELD(&);
        BINARY_OPERATOR_OP_EQ_FIELD(|);
        BINARY_OPERATOR_OP_EQ_FIELD(^);
        BINARY_OPERATOR_OP_EQ_FIELD(<<);
        BINARY_OPERATOR_OP_EQ_FIELD(>>);

        private:
        void* instance;
        using Ref = typename InstanceField<T, offset, false>::Ref;
    };

#undef BINARY_OPERATOR_OP_EQ_FIELD


#define BINARY_OPERATOR_OP_EQ_STATIC_FIELD(op)  \
template<typename U>                            \
auto& operator op##=(const U& rhs) {            \
    auto temp = this->operator T();             \
    return this->operator =(temp op##= rhs);    \
}

    // Static fields all have proper wbarriers through using set field API calls
    template<class T, internal::NTTPString name, auto klass_resolver, bool is_const = true>
    struct StaticField {
        operator T() const {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static field with name: ") + name.data.data() + " is null!");
            auto val = il2cpp_utils::GetFieldValue<T>(klass, name.data.data());
            if (!val) throw FieldException(std::string("Could not get static field with name: ") + name.data.data());
            return *val;
        }

        inline auto operator ->() const {
            return this->operator T();
        }

        inline auto operator *() const {
            return this->operator T();
        }

        inline auto v() const {
            return this->operator T();
        }

        template<typename... Targs>
        decltype(auto) operator ()(Targs&&... args) { return this->operator T()(std::forward<Targs...>(args...)); }
    };

    template<class T, internal::NTTPString name, auto klass_resolver>
    struct AssignableStaticField : public StaticField<T, name, klass_resolver, false> {
        template<class U>
#ifdef HAS_CODEGEN
        requires(std::is_convertible_v<U, T>)
#endif
        AssignableStaticField& operator=(U&& value) {
            auto klass = klass_resolver();
            if (!klass) throw NullException(std::string("Class for static field with name: ") + name.data.data() + " is null!");
            auto val = il2cpp_utils::SetFieldValue(klass, name.data.data(), std::forward<decltype(value)>(value));
            if (!val) throw FieldException(std::string("Could not set static field with name: ") + name.data.data());
            return *this;
        }

        auto& operator ++() { return this->operator=(++this->operator T()); }
        auto& operator ++(int) { return this->operator=(this->operator T()++); }
        auto& operator --() { return this->operator=(--this->operator T()); }
        auto& operator --(int) { return this->operator=(this->operator T()--); }

        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(+);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(-);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(*);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(/);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(%);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(&);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(|);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(^);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(<<);
        BINARY_OPERATOR_OP_EQ_STATIC_FIELD(>>);
    };

#undef BINARY_OPERATOR_OP_EQ_STATIC_FIELD

}
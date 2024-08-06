#pragma once

#pragma pack(push)

#include "logging.hpp"
#include "il2cpp-type-check.hpp"
#include "il2cpp-functions.hpp"
#include "il2cpp-utils-exceptions.hpp"
#include "base-wrapper-type.hpp"
#include "type-concepts.hpp"
#include <optional>
#include <vector>

namespace il2cpp_utils {
    template<class TOut>
    ::std::optional<TOut> FromIl2CppObject(Il2CppObject* obj) {
        il2cpp_functions::Init();

        // using Dt = ::std::decay_t<TOut>;
        void* val = obj;
        // nullptr (which runtime_invoke returns for "void" return type!) is different from nullopt (a runtime_invoke error!)
        if (obj && il2cpp_functions::class_is_valuetype(il2cpp_functions::object_get_class(obj))) {
            static auto& logger = getLogger();
            // So, because il2cpp finds it necessary to box returned value types (and also not deallocate them), we need to free them ourselves.
            // What we need to do is first extract the value, which we can do by casting and dereferencing
            // Then we need to PROPERLY free the allocating object at obj
            // Then we can return our result.
            val = RET_NULLOPT_UNLESS(logger, il2cpp_functions::object_unbox(obj));
            if constexpr (::std::is_pointer_v<TOut>) {
                // No cleanup necessary for pointer value types
                return static_cast<TOut>(val);
            } else {
                // Cleanup required here.
                auto ret = *static_cast<TOut*>(val);
                il2cpp_functions::GC_free(obj);
                return ret;
            }
        }
        if constexpr (::std::is_pointer_v<TOut>) {
            return static_cast<TOut>(val);
        } else if constexpr (il2cpp_reference_type_wrapper<TOut>) {
            return TOut(static_cast<void*>(val));
        }
        else {
            return *static_cast<TOut*>(val);
        }
    }

    template<class T>
    bool FromIl2CppObject(Il2CppObject* obj, T& out) {
        using Dt = ::std::decay_t<T>;
        if (auto ret = FromIl2CppObject<Dt>(obj)) {
            if constexpr (::std::is_pointer_v<Dt>) {
                // if they asked for the output in a pointer, we shouldn't change the pointer itself
                *out = *(*ret);
            } else {
                out = *ret;
            }
            return true;
        }
        return false;
    }

    std::string GenericClassStandardName(Il2CppGenericClass* genClass);
    // Some parts provided by zoller27osu
    // Logs information about the given Il2CppClass* as log(DEBUG)
    void LogClass(LoggerContextObject& logger, Il2CppClass* klass, bool logParents = false) noexcept;

    // Logs all classes (from every namespace) that start with the given prefix
    // WARNING: THIS FUNCTION IS VERY SLOW. ONLY USE THIS FUNCTION ONCE AND WITH A FAIRLY SPECIFIC PREFIX!
    void LogClasses(LoggerContextObject& logger, ::std::string_view classPrefix, bool logParents = false) noexcept;

    // Gets the System.Type Il2CppObject* (actually an Il2CppReflectionType*) for an Il2CppClass*
    Il2CppReflectionType* GetSystemType(const Il2CppClass* klass);
    Il2CppReflectionType* GetSystemType(const Il2CppType* typ);

    // Gets the System.Type Il2CppObject* (actually an Il2CppReflectionType*) for the class with the given namespace and name
    Il2CppReflectionType* GetSystemType(::std::string_view nameSpace, ::std::string_view className);

    // Gets the standard class name of an Il2CppClass*
    ::std::string ClassStandardName(const Il2CppClass* klass, bool generics = true);

    // Gets a C# name of a type
    const char* TypeGetSimpleName(const Il2CppType* type);

    // "Calling" this gives a compile-time warning (if warnings from this header are enabled)
    template<class T>
    [[deprecated]]void a_lack_of_no_arg_class_for([[maybe_unused]]::std::string_view s) {};

    ///
    /// \return The Il2CppClass* for arg. If arg is Il2CppClass*, returns itself
    ///
    template<typename T>
    Il2CppClass* ExtractClass(T&& arg) {
        using Dt = ::std::decay_t<T>;
        using arg_class = il2cpp_type_check::il2cpp_arg_class<Dt>;
        static auto& logger = getLogger();
        Il2CppClass* klass = arg_class::get(arg);
        if (!klass) {
            logger.error("Failed to determine class! Tips: instead of nullptr, pass the Il2CppType* or Il2CppClass* of the argument instead!");
        }
        return klass;
    }

    template<class T, bool ResultRequired = false>
    Il2CppClass* NoArgClass() {
        // TODO: change ifndef HAS_CODEGEN to 'if compile warnings are not errors'?
        static auto& logger = getLogger();
        #ifndef HAS_CODEGEN
        using arg_class = il2cpp_type_check::il2cpp_no_arg_class<T>;
        if constexpr (!has_get<arg_class>) {
            if constexpr (ResultRequired) {
                static_assert(false_t<arg_class>, "il2cpp-type-check.hpp could not deduce what C# type your type represents");
            } else {
                a_lack_of_no_arg_class_for<T>("please tell il2cpp-type-check.hpp what C# type your type represents");
                THROW_OR_RET_NULL(logger, false);
            }
        } else
        #endif
        if constexpr (ResultRequired) {
            return THROW_OR_RET_NULL(logger, il2cpp_type_check::il2cpp_no_arg_class<T>::get());
        } else {
            return il2cpp_type_check::il2cpp_no_arg_class<T>::get();
        }
    }

    template<typename T>
    const Il2CppType* ExtractType(T&& arg) {
        static auto& logger = getLogger();
        const Il2CppType* typ = il2cpp_type_check::il2cpp_arg_type<T>::get(arg);
        if (!typ)
            logger.error("ExtractType: failed to determine type! Tips: instead of nullptr, pass the Il2CppType* or Il2CppClass* of the argument instead!");
        return typ;
    }

    // Like ExtractType, but only returns an Il2CppType* if it can be extracted without an instance of T.
    template<class T>
    const Il2CppType* ExtractIndependentType() {
        static auto& logger = getLogger();
        static auto* typ = RET_0_UNLESS(logger, il2cpp_type_check::il2cpp_no_arg_type<T>::get());
        return typ;
    }

    inline auto ExtractTypes() {
        return ::std::array<const Il2CppType*, 0>();
    }

    template <typename... TArgs>
    auto ExtractTypes(TArgs&&... args) {
        constexpr std::size_t array_count = sizeof...(TArgs);

        return std::array<const Il2CppType*, array_count>(ExtractType(args)...);
    }

    // Adds the given TypeDefinitionIndex to the class hash table of a given image
    // Mainly used in LogClasses
    void AddTypeToNametoClassHashTable(const Il2CppImage* img, TypeDefinitionIndex index);

    // Adds the given nested types of the namespaze, parentName, and klass to the hastable
    // Mainly used in AddTypeToNametoClassHashTable
    void AddNestedTypesToNametoClassHashTable(Il2CppNameToTypeHandleHashTable* hashTable, const char *namespaze, const ::std::string& parentName, Il2CppClass *klass);

    // Adds the given nested types of typeDefinition to the class hash table of a given image
    // Mainly used in AddTypeToNametoClassHashTable
    void AddNestedTypesToNametoClassHashTable(const Il2CppImage* img, const Il2CppTypeDefinition* typeDefinition);

    /// @brief This method allows you to check if the parameter is a child or instance of the parent class. E.g (B extends A)
    /// @tparam ParentT The parent class (left hand assignment)
    /// @param subOrInstanceKlass the instance class (right hand assignment)
    /// ```
    /// A a;
    /// if (a is B b) {
    ///
    /// }
    /// ```
    /// @return Returns true if subOrInstanceKlass is a child or instance of ParentT. For more information, check https://docs.microsoft.com/en-us/dotnet/api/system.type.isassignablefrom?view=net-5.0
    template<typename ParentT>
    bool AssignableFrom(Il2CppClass* subOrInstanceKlass) {
        il2cpp_functions::Init();
        RET_DEFAULT_UNLESS(getLogger(), subOrInstanceKlass);
        static auto* parentK = RET_DEFAULT_UNLESS(getLogger(), classof(ParentT));
        return il2cpp_functions::class_is_assignable_from(parentK, subOrInstanceKlass);
    }

    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// This function may crash. See try_cast for a version that does not.
    /// @tparam T The type to cast from.
    /// @tparam U The type to cast to.
    /// @return A U* of the cast value.
    template<class U, class T>
    [[nodiscard]] U* cast(T* inst) {
        // TODO: Assumes T* is (at least) an Il2CppClass**, this means it assumes klass as first field.
        static auto* k1 = classof(U*);
        if (!k1) {
            throw il2cpp_utils::exceptions::NullException("cannot cast null target klass!");
        }

        if (!inst) {
            throw il2cpp_utils::exceptions::NullException("cannot cast null instance!");
        }
        auto* k2 = *reinterpret_cast<Il2CppClass**>(inst);
        if (!k2) {
            throw il2cpp_utils::exceptions::NullException("cannot cast null klass!");
        }
        if (!il2cpp_functions::class_is_assignable_from(k1, k2)) {
            throw ::il2cpp_utils::exceptions::BadCastException(k2, k1, reinterpret_cast<Il2CppObject*>(inst));
        }

        return reinterpret_cast<U*>(inst);
    }
    /// @brief Performs an il2cpp type checked cast from T to U, reference version. See `cast` for more documentation.
    /// @tparam T The type to cast from.
    /// @tparam U The type to cast to.
    /// @return A U& of the cast value.
    template<typename U, typename T>
    [[nodiscard]] U& cast_ref(T& inst) {
        return *cast(std::addressof(inst));
    }
    /// @brief Performs an il2cpp type checked cast from T to U.
    /// This should only be done if both T and U are reference types
    /// Currently assumes the `klass` field is the first pointer in T.
    /// @tparam T The type to cast from.
    /// @tparam U The type to cast to.
    /// @return A U* of the cast value, if successful.
    template<typename U, typename T>
    [[nodiscard]] std::optional<U*> try_cast(T* inst) noexcept {
        static auto* k1 = classof(U*);
        if (!k1 || !inst) {
            return std::nullopt;
        }
        // TODO: Assumes T* is (at least) an Il2CppClass**, this means it assumes klass as first field.
        auto* k2 = *reinterpret_cast<Il2CppClass**>(inst);
        if (!k2) {
            return std::nullopt;
        }
        if (il2cpp_functions::class_is_assignable_from(k1, k2)) {
            return reinterpret_cast<U*>(inst);
        }
        return std::nullopt;
    }
}

#pragma pack(pop)

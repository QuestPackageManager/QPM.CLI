#include "il2cpp-functions.hpp"
#include "type-concepts.hpp"

#ifndef IL2CPP_TYPE_CHECK_H
#define IL2CPP_TYPE_CHECK_H

#pragma pack(push)

#if __has_include(<concepts>)
#include <concepts>
#include <type_traits>
template<typename T>
constexpr bool has_get = requires {
    T::get();
};
#ifndef BS_HOOK_NO_CONCEPTS
#define BS_HOOK_USE_CONCEPTS
#endif
#elif __has_include(<experimental/type_traits>)
#include <experimental/type_traits>
template<typename T>
using get_t = decltype( &T::get );

template<typename T>
constexpr bool has_get = std::experimental::is_detected_v<get_t, T>;
#else
#error No libraries for the implementation of has_get available!
#endif

#include "utils.h"
#include <string_view>

#ifndef classof
// Returns the Il2CppClass* of the provided type T.
// Should be a pointer type if it is a reference type, otherwise it should be a value type.
#define classof(...) (::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<__VA_ARGS__>::get())
#endif

#ifndef csTypeOf
// Returns the Il2CppReflectionType* of the provided type T.
// Should be a pointer type if it is a reference type, otherwise it should be a value type.
#if !defined(NO_CODEGEN_MACROS) && __has_include("System/Type.hpp")
namespace System {
    class Type;
}
#endif

/// @brief wrapper for the return of csTypeOf so it can be more easily used with codegen
struct Il2CppCsTypeWrapper {
    constexpr inline Il2CppCsTypeWrapper(void* t) noexcept : t(t) {}
    constexpr inline void* convert() const noexcept { return const_cast<void*>(t); }

    constexpr inline Il2CppCsTypeWrapper(Il2CppReflectionType* t) noexcept : t(static_cast<void*>(t)) {}
    constexpr inline operator Il2CppReflectionType*() const noexcept { return static_cast<Il2CppReflectionType*>(const_cast<void*>(t)); }

    #if !defined(NO_CODEGEN_MACROS) && __has_include("System/Type.hpp")
    constexpr inline Il2CppCsTypeWrapper(System::Type* t) noexcept : t(static_cast<void*>(t)) {}
    constexpr inline operator System::Type*() const noexcept { return static_cast<System::Type*>(const_cast<void*>(t)); }
    #endif

    void* t;
};

#define csTypeOf(...) (Il2CppCsTypeWrapper(::il2cpp_utils::GetSystemType(classof(__VA_ARGS__))))
#endif

namespace il2cpp_utils {


    // instant return
    inline ::std::vector<Il2CppClass*>& ClassesFrom(::std::vector<Il2CppClass*>& classes) {
        return classes;
    }
    inline ::std::vector<Il2CppClass*> const& ClassesFrom(::std::vector<Il2CppClass*> const& classes) {
        return classes;
    }
    ::std::vector<Il2CppClass*> ClassesFrom(::std::span<::std::string_view> strings);


    ::std::vector<const Il2CppType*> TypesFrom(::std::span<const Il2CppClass*> classes);
    ::std::vector<const Il2CppType*> TypesFrom(::std::span<::std::string_view> strings);

    // instant return
    inline std::vector<const Il2CppType*>& TypesFrom(std::vector<const Il2CppType*>& types) {
        return types;
    }
    inline std::vector<const Il2CppType*> const& TypesFrom(std::vector<const Il2CppType*> const& types) {
        return types;
    }

    // Returns the il2cpp_utils logger context singleton.
    LoggerContextObject& getLogger();

    // Returns the first matching class from the given namespace and typeName by searching through all assemblies that are loaded.
    Il2CppClass* GetClassFromName(std::string_view name_space, std::string_view type_name);

    // Function made by zoller27osu, modified by Sc2ad
    // PLEASE don't use, there are easier ways to get generics (see CreateParam, CreateFieldValue)
    Il2CppClass* MakeGeneric(const Il2CppClass* klass, std::span<const Il2CppClass* const> args);
    Il2CppClass* MakeGeneric(const Il2CppClass* klass, const Il2CppType** types, uint32_t numTypes);

    // Framework provided by DaNike
    namespace il2cpp_type_check {
    namespace {
        template<typename T>
        /// @brief Describes whether the type T needs to be boxed or not (for instance method invokes).
        /// This is true for all non-pointer types by default.
        /// @tparam T The type to be boxed or not.
        struct BS_HOOKS_HIDDEN need_box {
            constexpr static bool value = true;
        };

        template<typename T>
        /// @brief Describes whether the type T needs to be boxed or not (for instance method invokes).
        /// This is false for T*s by default.
        /// @tparam T The type to be boxed or not.
        struct BS_HOOKS_HIDDEN need_box<T*> {
            constexpr static bool value = false;
        };

        #define NEED_NO_BOX(type) \
        template<> \
        struct ::il2cpp_utils::il2cpp_type_check::need_box<type> { \
            constexpr static bool value = false; \
        }

        // You should extend this class for any nested/inner type of a generic type, and also put a
        // `using declaring_type = Foo<T>*;` (minus the * if it is a value type)
        // and a `static constexpr std::string_view NESTED_NAME = "Bar";`
        class NestedType {};
        // To fix "no member named 'get' in il2cpp_type_check::il2cpp_arg_class<Blah>", just define il2cpp_no_arg_class<Blah>!
        // The macros like DEFINE_IL2CPP_ARG_TYPE make this easy! The only reason to define an il2cpp_arg_class struct is if,
        // given a value of Blah, the class returned should depend on the value, and not in the same way as Blah's base class.
        #ifndef BS_HOOK_USE_CONCEPTS
        template<typename T, class Enable = void>
        #else
        template<typename T>
        #endif
        struct il2cpp_no_arg_class { };

        template<typename T>
#ifndef BS_HOOK_USE_CONCEPTS
        struct il2cpp_no_arg_class<T*, typename std::enable_if_t<has_get<il2cpp_no_arg_class<T>>>> {
        #else
        requires has_get<il2cpp_no_arg_class<T>>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<T*> {
        #endif
            static inline Il2CppClass* get() {
                static Il2CppClass* ptrKlass = nullptr;
                if (ptrKlass) return ptrKlass;
                
                il2cpp_functions::Init();
                static auto& logger = getLogger();
                auto* klass = RET_0_UNLESS(logger, il2cpp_no_arg_class<T>::get());
                RET_0_UNLESS(logger, il2cpp_functions::class_is_valuetype(klass));
                ptrKlass = il2cpp_functions::Class_GetPtrClass(klass);

                return ptrKlass;
            }
        };

        static inline Il2CppClass* GetGenericTemplateClass(Il2CppGenericClass* generic_class) {
            #ifdef UNITY_2021
            return CRASH_UNLESS(il2cpp_functions::type_get_class_or_element_class(generic_class->type));
            #else
            return CRASH_UNLESS(il2cpp_functions::MetadataCache_GetTypeInfoFromTypeDefinitionIndex(generic_class->typeDefinitionIndex));
            #endif
        }

        template<typename T>
        #ifndef BS_HOOK_USE_CONCEPTS
        struct il2cpp_no_arg_class<T, typename std::enable_if_t<std::is_base_of_v<NestedType, T> && T::__IL2CPP_IS_VALUE_TYPE>> {
        #else
            requires(std::is_base_of_v<NestedType, T> && T::__IL2CPP_IS_VALUE_TYPE)
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<T> {
        #endif
            // TODO: make this work on any class with a `using declaring_type`, then remove NestedType
            static inline Il2CppClass* get() {
                il2cpp_functions::Init();
                Il2CppClass* declaring = il2cpp_no_arg_class<typename T::declaring_type>::get();
                Il2CppClass* classWithNested = declaring;
                if (declaring->generic_class) {
                    // Class::GetNestedTypes refuses to work on generic instances, so get the generic template instead
                    classWithNested = GetGenericTemplateClass(declaring->generic_class);
                }
                #if __has_feature(cxx_rtti)
                std::string typeName = type_name<T>();
                auto idx = typeName.find_last_of(':');
                if (idx >= 0) typeName = typeName.substr(idx+1);
                #else
                std::string typeName(T::NESTED_NAME);
                #endif

                // log(INFO, "type_name: %s", typeName.c_str());
                void* myIter = nullptr;
                Il2CppClass* found = nullptr;
                while (Il2CppClass* nested = il2cpp_functions::class_get_nested_types(classWithNested, &myIter)) {
                    // log(INFO, "nested->name: %s", nested->name);
                    if (typeName == nested->name) {
                        found = nested;
                        break;
                    }
                }
                CRASH_UNLESS(found);
                if (declaring->generic_class) {
                    const Il2CppGenericInst* genInst = declaring->generic_class->context.class_inst;
                    found = CRASH_UNLESS(il2cpp_utils::MakeGeneric(found, genInst->type_argv, genInst->type_argc));
                }

                return found;
            }
        };

        // For non-value types, forward accordingly. Should only apply to T*s that have these properties.
        template<typename T>
        #ifndef BS_HOOK_USE_CONCEPTS
        struct il2cpp_no_arg_class<T, typename std::enable_if_t<std::is_base_of_v<NestedType, T> && !T::__IL2CPP_IS_VALUE_TYPE>> {
        #else
        requires (std::is_base_of_v<NestedType, T> && !T::__IL2CPP_IS_VALUE_TYPE)
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<T*> {
        #endif
            // TODO: make this work on any class with a `using declaring_type`, then remove NestedType
            static inline Il2CppClass* get() {
                il2cpp_functions::Init();
                Il2CppClass* declaring = il2cpp_no_arg_class<typename T::declaring_type>::get();
                Il2CppClass* classWithNested = declaring;
                if (declaring->generic_class) {
                    // Class::GetNestedTypes refuses to work on generic instances, so get the generic template instead
                    classWithNested = GetGenericTemplateClass(declaring->generic_class);
                }
                #if __has_feature(cxx_rtti)
                std::string typeName = type_name<T>();
                auto idx = typeName.find_last_of(':');
                if (idx >= 0) typeName = typeName.substr(idx+1);
                #else
                std::string typeName(T::NESTED_NAME);
                #endif

                // log(INFO, "type_name: %s", typeName.c_str());
                void* myIter = nullptr;
                Il2CppClass* found = nullptr;
                while (Il2CppClass* nested = il2cpp_functions::class_get_nested_types(classWithNested, &myIter)) {
                    // log(INFO, "nested->name: %s", nested->name);
                    if (typeName == nested->name) {
                        found = nested;
                        break;
                    }
                }
                CRASH_UNLESS(found);
                if (declaring->generic_class) {
                    const Il2CppGenericInst* genInst = declaring->generic_class->context.class_inst;
                    found = CRASH_UNLESS(il2cpp_utils::MakeGeneric(found, genInst->type_argv, genInst->type_argc));
                }

                return found;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_class {
            static inline Il2CppClass* get([[maybe_unused]] T arg) {
                if constexpr (has_get<il2cpp_no_arg_class<T>>) {
                    return il2cpp_no_arg_class<T>::get();
                }
                return nullptr;
            }
        };

#define DEFINE_IL2CPP_DEFAULT_TYPE(type, fieldName) \
        template<> \
        struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<type> { \
            static inline Il2CppClass* get() { \
                il2cpp_functions::Init(); \
                return il2cpp_functions::defaults->fieldName##_class; \
            } \
        }

#define DEFINE_IL2CPP_ARG_TYPE(type, nameSpace, className)                                \
            template <>                                                                           \
            struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<type> { \
                static inline Il2CppClass* get() {                                                \
                    static auto klass = il2cpp_utils::GetClassFromName(nameSpace, className);     \
                    return klass;                                                                 \
                }                                                                                 \
            }

        DEFINE_IL2CPP_DEFAULT_TYPE(int8_t, sbyte);
        DEFINE_IL2CPP_DEFAULT_TYPE(uint8_t, byte);
        DEFINE_IL2CPP_DEFAULT_TYPE(int16_t, int16);  // "short"
        DEFINE_IL2CPP_DEFAULT_TYPE(uint16_t, uint16);  // "ushort"
        DEFINE_IL2CPP_DEFAULT_TYPE(int32_t, int32);  // "int"
        DEFINE_IL2CPP_DEFAULT_TYPE(uint32_t, uint32);  // "uint"
        DEFINE_IL2CPP_DEFAULT_TYPE(int64_t, int64);  // "long"
        DEFINE_IL2CPP_DEFAULT_TYPE(uint64_t, uint64);  // "ulong"

        DEFINE_IL2CPP_DEFAULT_TYPE(float, single);
        DEFINE_IL2CPP_DEFAULT_TYPE(double, double);

        DEFINE_IL2CPP_DEFAULT_TYPE(bool, boolean);
        DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppChar, char);

        DEFINE_IL2CPP_DEFAULT_TYPE(void, void);
        DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppObject*, object);
        NEED_NO_BOX(Il2CppObject);
        DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppString*, string);
        NEED_NO_BOX(Il2CppString);
        DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppArray*, array);
        NEED_NO_BOX(Il2CppArray);
        DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionType*, systemtype);
        NEED_NO_BOX(Il2CppReflectionType);

        NEED_NO_BOX(Il2CppClass);

        template <>
        struct BS_HOOKS_HIDDEN il2cpp_arg_class<Il2CppClass*> {
            static inline Il2CppClass* get(Il2CppClass* arg) {
                return arg;
            }
        };

        template <>
        struct BS_HOOKS_HIDDEN il2cpp_arg_class<Il2CppType*> {
            static inline Il2CppClass* get(Il2CppType* arg) {
                static auto& logger = getLogger();
                RET_0_UNLESS(logger, arg);
                il2cpp_functions::Init();
                return il2cpp_functions::class_from_il2cpp_type(arg);
            }
        };

        // TODO: is_il2cpp_object type trait?

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_class<T*> {
            static inline Il2CppClass* get(T* arg) {
                using element_arg_class = il2cpp_no_arg_class<T>;
                if constexpr (has_get<element_arg_class>) {
                    Il2CppClass* elementClass = element_arg_class::get();
                    il2cpp_functions::Init();
                    return il2cpp_functions::Class_GetPtrClass(elementClass);
                }
                if (arg) {
                    il2cpp_functions::Init();
                    auto* klass = il2cpp_functions::object_get_class(reinterpret_cast<Il2CppObject*>(arg));
                    if (klass && klass->klass == klass) return klass;
                }
                using ptr_arg_class = il2cpp_no_arg_class<T*>;
                if constexpr (has_get<ptr_arg_class>) {
                    return ptr_arg_class::get();
                }
                return nullptr;
            }
        };

        template<template<typename... ST> class S>
        struct il2cpp_gen_struct_no_arg_class;

        template<template<typename... ST> class S>
        struct il2cpp_gen_class_no_arg_class;

        template<template<typename... ST> class S>
        struct need_box_gen {
            constexpr static bool value = true;
        };

        template<typename... TArgs, template<typename... ST> class S>
        #ifndef BS_HOOK_USE_CONCEPTS
        struct il2cpp_no_arg_class<S<TArgs...>, typename std::enable_if_t<has_get<il2cpp_gen_struct_no_arg_class<S>>>> {
        #else
        requires has_get<il2cpp_gen_struct_no_arg_class<S>>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<S<TArgs...>> {
        #endif
            static inline Il2CppClass* get() {
                static Il2CppClass* gen_klass = nullptr;
                if (gen_klass) return gen_klass;

                auto* klass = il2cpp_gen_struct_no_arg_class<S>::get();
                std::array<const Il2CppClass*, sizeof...(TArgs)> const types{ il2cpp_no_arg_class<TArgs>::get()... };
                gen_klass = il2cpp_utils::MakeGeneric(klass, types);

                return gen_klass;
            }
        };

        template <typename... TArgs, template <typename... ST> class S>
        struct BS_HOOKS_HIDDEN need_box<S<TArgs...>> {
            constexpr static bool value = need_box_gen<S>::value;
        };

        template <typename... TArgs, template <typename... ST> class S>
        struct BS_HOOKS_HIDDEN need_box<S<TArgs...>*> {
            constexpr static bool value = false;
        };

        template<typename... TArgs, template<typename... ST> class S>
        requires (S<TArgs...>::__IL2CPP_IS_VALUE_TYPE && has_get<il2cpp_no_arg_class<typename S<TArgs...>::declaring_type>>)
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<S<TArgs...>> {
            static inline Il2CppClass* get() {
                static Il2CppClass* found = nullptr;

                if (found) {
                    return found;
                }

                // Resolve our declaring type
                Il2CppClass* declaring = il2cpp_no_arg_class<typename S<TArgs...>::declaring_type>::get();
                Il2CppClass* classWithNested = declaring;
                if (declaring->generic_class) {
                    // Class::GetNestedTypes refuses to work on generic instances, so get the generic template instead
                    classWithNested = GetGenericTemplateClass(declaring->generic_class);
                }
                std::string typeName(S<TArgs...>::NESTED_NAME);

                void* myIter = nullptr;
                while (Il2CppClass* nested = il2cpp_functions::class_get_nested_types(classWithNested, &myIter)) {
                    if (typeName == nested->name) {
                        found = nested;
                        break;
                    }
                }
                CRASH_UNLESS(found);
                if (declaring->generic_class) {
                    const Il2CppGenericInst* genInst = declaring->generic_class->context.class_inst;
                    found = CRASH_UNLESS(il2cpp_utils::MakeGeneric(found, genInst->type_argv, genInst->type_argc));
                }
                return found;
            }
        };

        template<typename... TArgs, template<typename... ST> class S>
        requires (!S<TArgs...>::__IL2CPP_IS_VALUE_TYPE && has_get<il2cpp_no_arg_class<typename S<TArgs...>::declaring_type>>)
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<S<TArgs...>*> {
            static inline Il2CppClass* get() {
                static Il2CppClass* found = nullptr;

                if (found) {
                    return found;
                }

                // Resolve our declaring type
                Il2CppClass* declaring = il2cpp_no_arg_class<typename S<TArgs...>::declaring_type>::get();
                Il2CppClass* classWithNested = declaring;
                if (declaring->generic_class) {
                    // Class::GetNestedTypes refuses to work on generic instances, so get the generic template instead
                    classWithNested = GetGenericTemplateClass(declaring->generic_class);
                }
                std::string typeName(S<TArgs...>::NESTED_NAME);

                void* myIter = nullptr;
                while (Il2CppClass* nested = il2cpp_functions::class_get_nested_types(classWithNested, &myIter)) {
                    if (typeName == nested->name) {
                        found = nested;
                        break;
                    }
                }
                CRASH_UNLESS(found);
                if (declaring->generic_class) {
                    const Il2CppGenericInst* genInst = declaring->generic_class->context.class_inst;
                    found = CRASH_UNLESS(il2cpp_utils::MakeGeneric(found, genInst->type_argv, genInst->type_argc));
                }
                return found;
            }
        };

        template<typename... TArgs, template<typename... ST> class S>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_class<S<TArgs...>*> {
            static inline Il2CppClass* get() {
                static Il2CppClass* genInst = nullptr;
                if (genInst) return genInst;

                Il2CppClass* genTemplate;
                bool isStruct = false;
                if constexpr (has_get<il2cpp_gen_class_no_arg_class<S>>) {
                    genTemplate = il2cpp_gen_class_no_arg_class<S>::get();
                } else if constexpr (has_get<il2cpp_gen_struct_no_arg_class<S>>) {
                    genTemplate = il2cpp_gen_struct_no_arg_class<S>::get();
                    isStruct = true;
                } else {
                    static_assert(false_t<S<TArgs...>>);
                }
                auto genericArgs = std::array<const Il2CppClass*, sizeof...(TArgs)>({ il2cpp_no_arg_class<TArgs>::get()... });
                genInst = il2cpp_utils::MakeGeneric(genTemplate, std::span(genericArgs));
                if (isStruct) {
                    il2cpp_functions::Init();
                    genInst = il2cpp_functions::Class_GetPtrClass(genInst);
                }
                return genInst;
            }
        };

        #define DEFINE_IL2CPP_ARG_TYPE_GENERIC_STRUCT(templateType, nameSpace, className) \
        template<> \
        struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::il2cpp_gen_struct_no_arg_class<templateType> { \
            static inline Il2CppClass* get() { \
                static auto klass = il2cpp_utils::GetClassFromName(nameSpace, className); \
                return klass; \
            } \
        }

        #define DEFINE_IL2CPP_ARG_TYPE_GENERIC_CLASS(templateType, nameSpace, className) \
        template<> \
        struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::il2cpp_gen_class_no_arg_class<templateType> { \
            static inline Il2CppClass* get() { \
                static auto klass = ::il2cpp_utils::GetClassFromName(nameSpace, className); \
                return klass; \
            } \
        }; \
        template<> \
        struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::need_box_gen<templateType> { \
            constexpr static bool value = false; \
        }

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_type {
            static inline const Il2CppType* get() {
                auto klass = il2cpp_no_arg_class<T>::get();
                if (klass) {
                    return &klass->byval_arg;
                }
                return nullptr;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_type<T*> {
            static inline const Il2CppType* get() {
                auto klass = il2cpp_no_arg_class<T*>::get();
                if (klass) {
                    return &klass->byval_arg;
                }
                return nullptr;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_type<T&&> {
            static inline const Il2CppType* get() {
                auto klass = il2cpp_no_arg_class<T>::get();
                if (klass) {
                    return &klass->byval_arg;
                }
                return nullptr;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_type<T&> {
            static inline const Il2CppType* get() {
                auto klass = il2cpp_no_arg_class<T>::get();
                if (klass) {
                    return &klass->this_arg;
                }
                return nullptr;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_no_arg_type<T const&> {
            static inline const Il2CppType* get() {
                return il2cpp_no_arg_type<T>::get();
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_type {};

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_type<T&> {
            static inline const Il2CppType* get(T& arg) {
                // Pure reference type is not the same as ByRef<T>. Thus, use the byval version.
                // Therefore, the only way to get the byref type match for any expression is to use a ByRef.
                Il2CppClass* klass = il2cpp_arg_class<T>::get(arg);
                return &klass->byval_arg;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_type<T*> {
            static inline const Il2CppType* get(T* arg) {
                // A pointer could be passed in explicitly. In such a case, get the class of the pointer and return it non-byref.
                Il2CppClass* klass = il2cpp_arg_class<T*>::get(arg);
                return &klass->byval_arg;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_type<const T&> {
            static inline const Il2CppType* get(const T& arg) {
                // A method cannot store a result back to a const ref. It is not a C# ref.
                const Il2CppClass* klass = il2cpp_arg_class<T>::get(arg);
                return &klass->byval_arg;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_type<T&&> {
            static inline const Il2CppType* get(T&& arg) {
                Il2CppClass* klass = il2cpp_arg_class<T>::get(arg);
                return &klass->byval_arg;
            }
        };

        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_ptr {
            static inline void* get(T const& arg) {
                return reinterpret_cast<void*>(&arg);
            }
        };
        template <typename T>
        struct BS_HOOKS_HIDDEN il2cpp_arg_ptr<T*> {
            static inline void* get(T* arg) {
                return reinterpret_cast<void*>(arg);
            }
        };

        /// @brief Represents a specialization type that should be used for exposing metadata from particular values.
        /// @tparam val The value to specialize this particular metadata getter on.
        template <auto val>
        struct BS_HOOKS_HIDDEN MetadataGetter;

        template <class T>
        struct BS_HOOKS_HIDDEN MethodDecomposer;

        template <typename R, typename... TArgs>
        struct BS_HOOKS_HIDDEN MethodDecomposer<R (*)(TArgs...)> {
            using mPtr = R (*)(TArgs...);
        };

        template <typename R, typename T, typename... TArgs>
        struct BS_HOOKS_HIDDEN MethodDecomposer<R (T::*)(TArgs...)> {
            using mPtr = R (*)(T*, TArgs...);
        };

        template<auto val>
        concept is_valid_il2cpp_method = requires (decltype(val) v) {
            typename MethodDecomposer<decltype(val)>::mPtr;
            { il2cpp_utils::il2cpp_type_check::MetadataGetter<val>::get() } -> std::same_as<const MethodInfo *>;
        };

        template <auto val>
            requires(is_valid_il2cpp_method<val>)
        struct BS_HOOKS_HIDDEN FPtrWrapper {
            static auto get() {
                return reinterpret_cast<typename MethodDecomposer<decltype(val)>::mPtr>(il2cpp_utils::il2cpp_type_check::MetadataGetter<val>::get()->methodPointer);
            }
        };
    }
    }
}


#pragma pack(pop)

#endif /* #ifndef IL2CPP_TYPE_CHECK_H */

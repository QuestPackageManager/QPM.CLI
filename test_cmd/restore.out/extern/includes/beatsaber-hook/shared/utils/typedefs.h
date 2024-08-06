#ifndef TYPEDEFS_H
#define TYPEDEFS_H

#pragma pack(push)

#include <stdio.h>
#include <stdlib.h>
#include <type_traits>
#include <initializer_list>

#include <cassert>
// For including il2cpp properly
#ifdef _MSC_VER
#undef _MSC_VER
#endif
#ifndef __GNUC__
#define __GNUC__
#endif

#define NET_4_0 true
#include "il2cpp-config.h"
#include "il2cpp-api-types.h"
#include "il2cpp-class-internals.h"
#include "il2cpp-tabledefs.h"

#ifdef __cplusplus

template<class T>
struct Array;

extern "C" {
#endif

#ifdef __cplusplus
}  /* extern "C" */
#endif /* __cplusplus */

#include "manual-il2cpp-typedefs.h"

#include "il2cpp-functions.hpp"
#include "il2cpp-utils-methods.hpp"
#include "il2cpp-type-check.hpp"

// forward declarations of the list wrapper
template<typename T, typename Ptr>
struct ListWrapper;

// forward declaration of the string wrapper
template<typename Ptr>
struct StringWrapper;

#include "typedefs-array.hpp"
#include "typedefs-delegate.hpp"
#include "typedefs-string.hpp"
#include "typedefs-list.hpp"
#include "typedefs-wrappers.hpp"

#include <stdint.h>

namespace il2cpp_utils {
    namespace array_utils {
        static char* il2cpp_array_addr_with_size(Il2CppArray *array, int32_t size, uintptr_t idx)
        {
            return ((char*)array) + kIl2CppSizeOfArray + size * idx;
        }
        #define load_array_elema(arr, idx, size) ((((uint8_t*)(arr)) + kIl2CppSizeOfArray) + ((size) * (idx)))

        #define il2cpp_array_setwithsize(array, elementSize, index, value)  \
            do {    \
                void*__p = (void*) il2cpp_utils::array_utils::il2cpp_array_addr_with_size ((array), elementSize, (index)); \
                memcpy(__p, &(value), elementSize); \
            } while (0)
        #define il2cpp_array_setrefwithsize(array, elementSize, index, value)  \
            do {    \
                void*__p = (void*) il2cpp_utils::array_utils::il2cpp_array_addr_with_size ((array), elementSize, (index)); \
                memcpy(__p, value, elementSize); \
                } while (0)
        #define il2cpp_array_addr(array, type, index) ((type*)(void*) il2cpp_utils::array_utils::il2cpp_array_addr_with_size (array, sizeof (type), index))
        #define il2cpp_array_get(array, type, index) ( *(type*)il2cpp_array_addr ((array), type, (index)) )
        #define il2cpp_array_set(array, type, index, value)    \
            do {    \
                type *__p = (type *) il2cpp_array_addr ((array), type, (index));    \
                *__p = (value); \
            } while (0)
        #define il2cpp_array_setref(array, index, value)  \
            do {    \
                void* *__p = (void* *) il2cpp_array_addr ((array), void*, (index)); \
                /* il2cpp_gc_wbarrier_set_arrayref ((array), __p, (MonoObject*)(value));    */\
                *__p = (value);    \
            } while (0)
    }
}




// From Runtime.cpp (some may need the * removed):
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppMulticastDelegate*, multicastdelegate);
NEED_NO_BOX(Il2CppMulticastDelegate);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppAsyncCall*, async_call);
NEED_NO_BOX(Il2CppAsyncCall);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppInternalThread*, internal_thread);
NEED_NO_BOX(Il2CppInternalThread);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionEvent*, event_info);
NEED_NO_BOX(Il2CppReflectionEvent);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppStringBuilder*, stringbuilder);
NEED_NO_BOX(Il2CppStringBuilder);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppStackFrame*, stack_frame);
NEED_NO_BOX(Il2CppStackFrame);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionAssemblyName*, assembly_name);
NEED_NO_BOX(Il2CppReflectionAssemblyName);
// DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionAssembly*, assembly);
#ifndef UNITY_2021
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionAssembly*, mono_assembly);
NEED_NO_BOX(Il2CppReflectionAssembly);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionField*, mono_field);
NEED_NO_BOX(Il2CppReflectionField);
// DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionParameter*, parameter_info);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionParameter*, mono_parameter_info);
NEED_NO_BOX(Il2CppReflectionParameter);
#endif
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionModule*, module);
NEED_NO_BOX(Il2CppReflectionModule);
#ifndef UNITY_2021
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionPointer*, pointer);
NEED_NO_BOX(Il2CppReflectionPointer);
#endif
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppSystemException*, system_exception);
NEED_NO_BOX(Il2CppSystemException);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppArgumentException*, argument_exception);
NEED_NO_BOX(Il2CppArgumentException);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppMarshalByRefObject*, marshalbyrefobject);
NEED_NO_BOX(Il2CppMarshalByRefObject);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppSafeHandle*, safe_handle);
NEED_NO_BOX(Il2CppSafeHandle);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppSortKey*, sort_key);
NEED_NO_BOX(Il2CppSortKey);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppErrorWrapper*, error_wrapper);
NEED_NO_BOX(Il2CppErrorWrapper);
// TODO: attempt to move out of this conditional if codegen ever gets an Il2CppComObject?
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppComObject*, il2cpp_com_object);
NEED_NO_BOX(Il2CppComObject);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppTypedRef, typed_reference);

DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppDelegate*, delegate);
NEED_NO_BOX(Il2CppDelegate);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionMonoType*, monotype);
NEED_NO_BOX(Il2CppReflectionMonoType);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppThread*, thread);
NEED_NO_BOX(Il2CppThread);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionRuntimeType*, runtimetype);
NEED_NO_BOX(Il2CppReflectionRuntimeType);
#ifndef UNITY_2021
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionMonoEventInfo*, mono_event_info);
NEED_NO_BOX(Il2CppReflectionMonoEventInfo);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppReflectionMethod*, mono_method);
NEED_NO_BOX(Il2CppReflectionMethod);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppMethodInfo*, mono_method_info);
NEED_NO_BOX(Il2CppMethodInfo);
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppPropertyInfo*, mono_property_info);
NEED_NO_BOX(Il2CppPropertyInfo);
#endif
DEFINE_IL2CPP_DEFAULT_TYPE(Il2CppException*, exception);
NEED_NO_BOX(Il2CppException);

DEFINE_IL2CPP_ARG_TYPE(long double, "System", "Decimal");

template<class T>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<ListW<T>> {
    static inline Il2CppClass* get() {
        auto klass = ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<typename ListW<T>::WrappedType>::get();
        return klass;
    }
};

#include "utils/Il2CppHashMap.h"
#include "utils/HashUtils.h"
#include "utils/StringUtils.h"

struct NamespaceAndNamePairHash
{
    size_t operator()(const std::pair<const char*, const char*>& pair) const
    {
        return il2cpp::utils::HashUtils::Combine(il2cpp::utils::StringUtils::Hash(pair.first), il2cpp::utils::StringUtils::Hash(pair.second));
    }
};

struct NamespaceAndNamePairEquals
{
    bool operator()(const std::pair<const char*, const char*>& p1, const std::pair<const char*, const char*>& p2) const
    {
        return !strcmp(p1.first, p2.first) && !strcmp(p1.second, p2.second);
    }
};

struct Il2CppNameToTypeHandleHashTable : public Il2CppHashMap<std::pair<const char*, const char*>, Il2CppMetadataTypeHandle, NamespaceAndNamePairHash, NamespaceAndNamePairEquals>
{
    typedef Il2CppHashMap<std::pair<const char*, const char*>, Il2CppMetadataTypeHandle, NamespaceAndNamePairHash, NamespaceAndNamePairEquals> Base;
    Il2CppNameToTypeHandleHashTable() : Base()
    {
    }
};

typedef struct Il2CppImageGlobalMetadata
{
    TypeDefinitionIndex typeStart;
    TypeDefinitionIndex exportedTypeStart;
    CustomAttributeIndex customAttributeStart;
    MethodIndex entryPointIndex;
    const Il2CppImage* image;
} Il2CppImageGlobalMetadata;

#pragma pack(pop)

#endif /* TYPEDEFS_H */

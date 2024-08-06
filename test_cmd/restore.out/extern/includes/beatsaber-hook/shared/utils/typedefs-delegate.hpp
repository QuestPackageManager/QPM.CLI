#pragma once

#include "manual-il2cpp-typedefs.h"
#include "typedefs-object.hpp"

// System.MulticastDelegate
typedef struct MulticastDelegate : Il2CppDelegate {
    ::ArrayW<Il2CppDelegate*> delegates;
} MulticastDelegate;

// System.DelegateData
typedef struct DelegateData : Il2CppObject {
    Il2CppReflectionType* target_type;
    Il2CppString* method_name;
    bool curied_first_arg;
} DelegateData;

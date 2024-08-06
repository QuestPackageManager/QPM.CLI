#pragma once

#include "type-concepts.hpp"
#include "il2cpp-config.h"

typedef Il2CppClass Il2CppVTable;
struct MonitorData;
typedef struct Il2CppObject
{
    union
    {
        Il2CppClass *klass;
        Il2CppVTable *vtable;
    };
    MonitorData *monitor;
} Il2CppObject;

MARK_REF_PTR_T(Il2CppObject);

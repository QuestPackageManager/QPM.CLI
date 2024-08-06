#pragma once

// Include this file to assert that you have defined the necessary functions with the correct signatures

#include <type_traits>
#include "scotland2/shared/loader.hpp"

static_assert(std::is_same_v<modloader::SetupFunc, decltype(&setup)>, "Must match the specified signature with setup!");
static_assert(std::is_same_v<modloader::LoadFunc, decltype(&load)>, "Must match the specified signature with load!");

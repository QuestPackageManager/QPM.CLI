#pragma once

#include <locale>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include "il2cpp-type-check.hpp"
#include "il2cpp-utils-exceptions.hpp"
#include "utils-functions.h"
#include "type-concepts.hpp"

#include "manual-il2cpp-typedefs.h"

struct UseBeforeInitError : il2cpp_utils::exceptions::StackTraceException {
    UseBeforeInitError(const char* v) : il2cpp_utils::exceptions::StackTraceException(v) {}
};

struct Il2CppString;

namespace il2cpp_utils {
namespace detail {
void convstr(char const* inp, char16_t* outp, int sz);
std::size_t convstr(char16_t const* inp, char* outp, int isz, int osz);

static std::string to_string(Il2CppString* str) {
    std::string val(str->length * sizeof(wchar_t) + 1, '\0');
    auto resSize = il2cpp_utils::detail::convstr(str->chars, val.data(), str->length, val.size());
    val.resize(resSize);
    return val;
}
static std::u16string to_u16string(Il2CppString* str) {
    return { str->chars, static_cast<std::size_t>(str->length) };
}
static std::wstring to_wstring(Il2CppString* str) {
    return { str->chars, str->chars + str->length };
}
static std::u16string_view to_u16string_view(Il2CppString* inst) {
    return { inst->chars, inst->chars + inst->length };
}
static std::u16string_view to_u16string_view(Il2CppString const* inst) {
    return { inst->chars, inst->chars + inst->length };
}

Il2CppString* alloc_str(std::string_view str);
Il2CppString* alloc_str(std::u16string_view str);

Il2CppString* strappend(Il2CppString const* lhs, Il2CppString const* rhs) noexcept;
Il2CppString* strappend(Il2CppString const* lhs, std::u16string_view const rhs) noexcept;
Il2CppString* strappend(Il2CppString const* lhs, std::string_view const rhs) noexcept;
Il2CppString* strappend(std::string_view const lhs, Il2CppString const* rhs) noexcept;
Il2CppString* strappend(std::u16string_view const lhs, Il2CppString const* rhs) noexcept;

bool strcomp(Il2CppString const* lhs, std::string_view const rhs) noexcept;
bool strcomp(Il2CppString const* lhs, std::u16string_view const rhs) noexcept;
bool strcomp(Il2CppString const* lhs, Il2CppString const* rhs) noexcept;

bool strless(Il2CppString const* lhs, std::string_view const rhs) noexcept;
bool strless(Il2CppString const* lhs, std::u16string_view const rhs) noexcept;
bool strless(Il2CppString const* lhs, Il2CppString const* rhs) noexcept;

bool strstart(Il2CppString const* lhs, std::string_view const rhs) noexcept;
bool strstart(Il2CppString const* lhs, std::u16string_view const rhs) noexcept;
bool strstart(Il2CppString const* lhs, Il2CppString const* rhs) noexcept;

bool strend(Il2CppString const* lhs, std::string_view const rhs) noexcept;
bool strend(Il2CppString const* lhs, std::u16string_view const rhs) noexcept;
bool strend(Il2CppString const* lhs, Il2CppString const* rhs) noexcept;
}  // namespace detail
}  // namespace il2cpp_utils

// C# strings can only have 'int' max length.
template <int sz>
struct ConstString {
    // Manually allocated string, dtor destructs in place
    ConstString(const char (&st)[sz]) {
        length = sz - 1;
        il2cpp_utils::detail::convstr(st, chars, sz - 1);
    }
    constexpr ConstString(const char16_t (&st)[sz]) noexcept {
        length = sz - 1;
        for (int i = 0; i < sz - 1; i++) {
            chars[i] = st[i];
        }
    }
    // Copies allowed? But should probably be avoided.
    ConstString(ConstString const&) = default;
    // Moves allowed
    ConstString(ConstString&&) = default;

    void init() noexcept {
        klass = il2cpp_functions::defaults->string_class;
    }

    constexpr operator Il2CppString*() {
        if (!klass) {
            if (il2cpp_functions::initialized) {
                klass = il2cpp_functions::defaults->string_class;
            } else {
                throw UseBeforeInitError("Il2CppClass* must be initialized before conversion! Call il2cpp_functions::Init before this conversion!");
            }
        }
        return reinterpret_cast<Il2CppString*>(&klass);
    }

    constexpr operator Il2CppString const*() const {
        if (!klass) {
            if (il2cpp_functions::initialized) {
                // due to klass being initialized being essential to the functionality of this type, we agreed that ignoring the const here is warranted
                // usually const casting is bad, but due to the reasons stated above we are doing it anyways
                const_cast<ConstString<sz>*>(this)->klass = il2cpp_functions::defaults->string_class;
            } else {
                throw UseBeforeInitError("Il2CppClass* must be initialized before conversion! Call il2cpp_functions::Init before this conversion!");
            }
        }
        return reinterpret_cast<Il2CppString const*>(&klass);
    }

    constexpr Il2CppString* operator->() {
        return operator Il2CppString*();
    }

    operator std::string() {
        std::string val((sz - 1) * 2 + 1, '\0');
        auto resSize = il2cpp_utils::detail::convstr(chars, val.data(), sz - 1, val.size());
        val.resize(resSize);
        return val;
    }
    operator std::u16string() {
        return { chars, chars + length };
    }
    operator std::wstring() {
        return { chars, chars + length };
    }
    constexpr operator std::u16string_view() {
        return { chars, static_cast<std::size_t>(sz) };
    }

    template<typename Ptr>
    friend struct StringWrapper;

   private:
    void* klass = nullptr;
    void* monitor = nullptr;
    int length = 0;
    char16_t chars[sz] = {};
};

template <typename Ptr>
struct StringWrapper {
    // Dynamically allocated string
    template <class T>
        requires(!std::is_convertible_v<T, Il2CppString*> && (std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T>))
    StringWrapper(T str) noexcept : inst(il2cpp_utils::detail::alloc_str(str)) {}
    constexpr StringWrapper(void* ins) noexcept : inst(static_cast<Il2CppString*>(ins)) {}
    constexpr StringWrapper(Il2CppString* ins) noexcept : inst(ins) {}
    template <int sz>
    constexpr StringWrapper(ConstString<sz>& conststring) noexcept : inst(static_cast<Il2CppString*>(conststring)) {}

    template <typename U>
    constexpr StringWrapper(StringWrapper<U> const& s) noexcept : inst(static_cast<Il2CppString*>(s)) {}
    constexpr StringWrapper(std::nullptr_t npt) noexcept : inst(npt) {}
    constexpr StringWrapper() noexcept : inst(nullptr) {}

    constexpr void* convert() const noexcept {
        return const_cast<void*>(static_cast<void*>(inst));
    }
    constexpr operator Il2CppString const*() const noexcept {
        return inst;
    }
    constexpr operator Il2CppString*() const noexcept {
        return inst;
    }
    constexpr Ptr operator->() noexcept {
        return reinterpret_cast<Ptr>(inst);
    }

    constexpr Ptr const operator->() const noexcept {
        return reinterpret_cast<Ptr const>(inst);
    }
    constexpr operator bool() const noexcept {
        return inst != nullptr;
    }

    constexpr bool operator==(std::nullptr_t rhs) const noexcept {
        return inst == rhs;
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    StringWrapper& operator+=(T const& rhs) noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            inst = il2cpp_utils::detail::strappend(inst, rhs.inst);
        else
            inst = il2cpp_utils::detail::strappend(inst, rhs);
        return *this;
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    StringWrapper operator+(T const& rhs) const noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            return il2cpp_utils::detail::strappend(inst, rhs.inst);
        else
            return il2cpp_utils::detail::strappend(inst, rhs);
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    bool operator<(T const& rhs) const noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            return il2cpp_utils::detail::strless(inst, rhs.inst);
        else
            return il2cpp_utils::detail::strless(inst, rhs);
    }

    template <int sz>
    bool operator==(ConstString<sz> const& rhs) const noexcept {
        return il2cpp_utils::detail::strcomp(inst, rhs.operator Il2CppString const*());
    }

    template <typename T>
    bool operator==(StringWrapper<T> const& rhs) const noexcept {
        return il2cpp_utils::detail::strcomp(inst, static_cast<Il2CppString const*>(rhs));
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    bool operator==(T const& rhs) const noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            return il2cpp_utils::detail::strcomp(inst, rhs.inst);
        else
            return il2cpp_utils::detail::strcomp(inst, rhs);
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    bool starts_with(T const& rhs) const noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            return il2cpp_utils::detail::strstart(inst, rhs.inst);
        else
            return il2cpp_utils::detail::strstart(inst, rhs);
    }

    template <typename T>
        requires(std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T> || std::is_same_v<T, StringWrapper>)
    bool ends_with(T const& rhs) const noexcept {
        if constexpr (std::is_same_v<T, StringWrapper>)
            return il2cpp_utils::detail::strend(inst, rhs.inst);
        else
            return il2cpp_utils::detail::strend(inst, rhs);
    }

    using iterator = Il2CppChar*;
    using const_iterator = Il2CppChar const*;

    iterator begin() {
        return inst->chars;
    }
    const_iterator begin() const {
        return inst->chars;
    }
    iterator end() {
        return inst->chars + inst->length;
    }
    const_iterator end() const {
        return inst->chars + inst->length;
    }
    operator std::span<Il2CppChar>() {
        return { begin(), end() };
    }
    operator std::span<Il2CppChar const> const() const {
        return { begin(), end() };
    }

    Il2CppChar const& operator[](size_t const& idx) const {
        return inst->chars[idx];
    }
    Il2CppChar& operator[](size_t const& idx) {
        return inst->chars[idx];
    }
    operator std::string() const {
        return il2cpp_utils::detail::to_string(inst);
    }
    operator std::u16string() const {
        return il2cpp_utils::detail::to_u16string(inst);
    }
    operator std::wstring() const {
        return il2cpp_utils::detail::to_wstring(inst);
    }
    operator std::u16string_view() {
        return il2cpp_utils::detail::to_u16string_view(inst);
    }
    operator std::u16string_view const() const {
        return il2cpp_utils::detail::to_u16string_view(inst);
    }

   private:
    Il2CppString* inst;
};
MARK_GEN_REF_T(StringWrapper);
MARK_REF_PTR_T(Il2CppString);

template <typename T, typename Ptr>
    requires(!std::is_constructible_v<T, StringWrapper<Ptr>> && (std::is_constructible_v<std::u16string_view, T> || std::is_constructible_v<std::string_view, T>))
StringWrapper<Ptr> operator+(T const lhs, StringWrapper<Ptr> const& rhs) noexcept {
    return il2cpp_utils::detail::strappend(lhs, rhs.operator Il2CppString const*());
}

template <class Ptr>
struct BS_HOOKS_HIDDEN ::il2cpp_utils::il2cpp_type_check::need_box<StringWrapper<Ptr>> {
    constexpr static bool value = false;
};

// if system string exists, we can use it in StringW, but with a compile definition it can be disabled
#if !defined(NO_CODEGEN_WRAPPERS) && __has_include("System/String.hpp")
// forward declare
namespace System {
    class String;
}
// put using statement
using StringW = StringWrapper<System::String*>;
// include actual type
#include "System/String.hpp"
#else
using StringW = StringWrapper<Il2CppString*>;
#endif

static_assert(sizeof(StringW) == sizeof(void*));
static_assert(il2cpp_utils::has_il2cpp_conversion<StringW>);
DEFINE_IL2CPP_DEFAULT_TYPE(StringW, string);
NEED_NO_BOX(StringW);

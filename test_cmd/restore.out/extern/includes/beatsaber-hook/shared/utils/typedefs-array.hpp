#pragma once
#include <cstddef>
#include <iterator>
#include <optional>
#include <type_traits>
#include <utility>
#include <vector>
#include <span>
#include "il2cpp-type-check.hpp"
#include "type-concepts.hpp"
#include <ranges>
#include <stdexcept>

#if __has_include(<concepts>)
#include <concepts>
#elif __has_include(<experimental/concepts>)
#include <experimental/concepts>
#else
#warning "Please have some form of concepts support!"
#endif

template<class T, class U>
/// @brief If type T can be assigned to by type U&&
/// @tparam T The left hand side of the assignment
/// @tparam U The right hand side of the assignment
concept has_assignment = requires (T arg, U&& other) {
    arg = other;
};

template<class T>
requires (!std::is_reference_v<T>)
/// @brief A std::wrapper_reference implementation that perfect forwards to contained assignment operators.
struct WrapperRef {
    /// @brief Explicitly create a wrapper reference from a reference who MUST LIVE LONGER THAN THIS INSTANCE!
    explicit constexpr WrapperRef(T& instance) : ptr(std::addressof(instance)) {}
    // Assignment for any U type that is valid to be assigned.
    template<class U>
    requires (has_assignment<T, U>)
    WrapperRef& operator=(U&& other) {
        *ptr = other;
        return *this;
    }
    // Standard Assignment
    WrapperRef& operator=(const WrapperRef& x) noexcept = default;
    // Getter operation, implicit conversion
    constexpr operator T& () const noexcept {
        return *ptr;
    }
    // Getter operation, explicit conversion
    constexpr T& get() const noexcept {
        return *ptr;
    }
    // Invoke operation
    template<class... ArgTypes>
    constexpr std::invoke_result_t<T&, ArgTypes...> operator() (ArgTypes&&... args) const {
        return std::invoke(get(), std::forward<ArgTypes>(args)...);
    }
    private:
    T* ptr;
};

#pragma pack(push)

#include "typedefs-object.hpp"
typedef int32_t il2cpp_array_lower_bound_t;
#define IL2CPP_ARRAY_MAX_INDEX ((int32_t) 0x7fffffff)
#define IL2CPP_ARRAY_MAX_SIZE  ((uint32_t) 0xffffffff)

typedef struct Il2CppArrayBounds
{
    il2cpp_array_size_t length;
    il2cpp_array_lower_bound_t lower_bound;
} Il2CppArrayBounds;

#if IL2CPP_COMPILER_MSVC
#pragma warning( push )
#pragma warning( disable : 4200 )
#elif defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Winvalid-offsetof"
#endif

//Warning: Updates to this struct must also be made to IL2CPPArraySize C code
#ifdef __cplusplus
typedef struct Il2CppArray : public Il2CppObject
{
#else
typedef struct Il2CppArray
{
    Il2CppObject obj;
#endif //__cplusplus
    /* bounds is NULL for szarrays */
    Il2CppArrayBounds *bounds;
    /* total number of elements of the array */
    il2cpp_array_size_t max_length;
} Il2CppArray;

#ifdef __cplusplus
typedef struct Il2CppArraySize : public Il2CppArray
{
#else
//mono code has no inheritance, so its members must be available from this type
typedef struct Il2CppArraySize
{
    Il2CppObject obj;
    Il2CppArrayBounds *bounds;
    il2cpp_array_size_t max_length;
#endif //__cplusplus
    ALIGN_TYPE(8) void* vector[IL2CPP_ZERO_LEN_ARRAY];
} Il2CppArraySize;

static const size_t kIl2CppSizeOfArray = (offsetof(Il2CppArraySize, vector));
static const size_t kIl2CppOffsetOfArrayBounds = (offsetof(Il2CppArray, bounds));
static const size_t kIl2CppOffsetOfArrayLength = (offsetof(Il2CppArray, max_length));

#include "utils.h"
#include "il2cpp-utils-methods.hpp"
#include <initializer_list>

/// @brief Represents an exception thrown from usage of an Array.
struct ArrayException : il2cpp_utils::exceptions::StackTraceException {
    void* arrayInstance;
    ArrayException(void* instance, std::string_view msg) : il2cpp_utils::exceptions::StackTraceException(msg.data()), arrayInstance(instance) {}
};


// forward declares for interfaces that System::Array implements, to allow conversion to these
#ifdef HAS_CODEGEN
namespace System{
    namespace Collections {
        class ICollection;
        class IEnumerable;
        class IList;
        class IStructuralComparable;
        class IStructuralEquatable;
    }

    class ICloneable;
}
#endif

template<class T>
struct Array : public Il2CppArray
{
    static_assert(is_value_type_v<T>, "T must be a C# value type! (primitive, pointer or Struct)");
    ALIGN_TYPE(8) T _values[IL2CPP_ZERO_LEN_ARRAY];

    auto begin() noexcept { return _values; }
    auto begin() const noexcept { return _values; }

    auto rbegin() noexcept { return std::reverse_iterator(_values + get_Length()); }
    auto rbegin() const noexcept { return std::reverse_iterator(_values + get_Length()); }

    auto end() noexcept { return _values + get_Length(); }
    auto end() const noexcept { return _values + get_Length(); }

    auto rend() noexcept { return std::reverse_iterator(_values); }
    auto rend() const noexcept { return std::reverse_iterator(_values); }

    inline il2cpp_array_size_t get_Rank() const noexcept {
        return bounds ? bounds->length : 0;
    }

    inline il2cpp_array_size_t get_Length() const noexcept {
        if (bounds) {
            return bounds->length;
        }
        return max_length;
    }

    inline void AssertBounds(size_t i) {
        if (i < 0 || i >= get_Length()) {
            throw ArrayException(this, string_format("%zu is out of bounds for array of length: %zu", i, get_Length()));
        }
    }

    static Array<T>* New(std::initializer_list<T> vals) {
        il2cpp_functions::Init();
        auto* arr = reinterpret_cast<Array<T>*>(il2cpp_functions::array_new(
            il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<T>::get(), vals.size()));
        if (!arr) {
            throw ArrayException(nullptr, "Could not create Array!");
        }
        memcpy(arr->_values, vals.begin(), sizeof(T)*vals.size());
        return arr;
    }

    static Array<T>* NewLength(il2cpp_array_size_t size) {
        il2cpp_functions::Init();
        auto arr = reinterpret_cast<Array<T>*>(il2cpp_functions::array_new(
            il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<T>::get(), size));
        if (!arr) {
            throw ArrayException(nullptr, "Could not create Array!");
        }
        return arr;
    }

    template<typename... TArgs>
    static Array<T>* New(TArgs&&... args) {
        return New({args...});
    }

    template<class U = Il2CppObject*>
    U GetEnumerator() {
        static auto* method = CRASH_UNLESS(il2cpp_utils::FindMethodUnsafe(
            this, "System.Collections.Generic.IEnumerable`1.GetEnumerator", 0));
        return il2cpp_utils::RunMethodRethrow<U, false>(this, method);
    }

    bool Contains(T item) {
        // TODO: should this use System.Object::Equals ?
        auto itr = std::find_if(begin(), end(), [&item](auto& x){ return x == item; });
        return itr != end();
    }

    T First() {
        if (get_Length() > 0)
            return _values[0];
        else
            throw ArrayException(this, "First called on empty array!");
    }

    T FirstOrDefault() requires(std::is_default_constructible_v<T>) {
        if (get_Length() > 0)
            return _values[0];
        else
            return {};
    }

    T Last() {
        if (get_Length() > 0)
            return _values[get_Length() - 1];
        else
            throw ArrayException(this, "Last called on empty array!");
    }

    T LastOrDefault() requires(std::is_default_constructible_v<T>) {
        if (get_Length() > 0)
            return _values[get_Length() - 1];
        else
            return {};
    }

    template<typename Predicate>
    T First(Predicate&& pred) {
        auto itr = std::find_if(begin(), end(), pred);
        if (itr == end()) throw ArrayException(this, "Unable to find First item with given predicate!");
        return *itr;
    }

    template<typename Predicate>
    T FirstOrDefault(Predicate&& pred) requires(std::is_default_constructible_v<T>) {
        auto itr = std::find_if(begin(), end(), pred);
        if (itr == end()) return T{};
        return *itr;
    }

    template<typename Predicate>
    T Last(Predicate&& pred) {
        auto itr = std::find_if(rbegin(), rend(), pred);
        if (itr == rend()) throw ArrayException(this, "Unable to find Last item with given predicate!");
        return *itr;
    }

    template<typename Predicate>
    T LastOrDefault(Predicate&& pred) requires(std::is_default_constructible_v<T>) {
        auto itr = std::find_if(rbegin(), rend(), pred);
        if (itr == rend()) return T{};
        return *itr;
    }

    void CopyTo(Array<T>* array, int arrayIndex) {
        if (array && array->get_Rank() <= 1) throw ArrayException(array, "Only single dimensional arrays are supported for the requested action");
        Copy(this, 0, array, arrayIndex);
    }

    static void Copy(Array<T>* sourceArray, int sourceIndex, Array<T>* destinationArray, int destinationIndex, int length) {
        if (!sourceArray) throw ArrayException(sourceArray, "null source");
        if (!destinationArray) throw ArrayException(destinationArray, "null destination");
        if (length < 0) throw ArrayException(sourceArray, "length value has to be >= 0");
        if (sourceArray->get_Rank() != destinationArray->get_Rank()) throw ArrayException(sourceArray, "Rank has to be equal between the two arrays");
        if (sourceIndex < 0) throw ArrayException(sourceArray, "source index value has to be >= 0");
        if (destinationIndex < 0) throw ArrayException(sourceArray, "destination index value has to be >= 0");

        auto srcLength = sourceArray->get_Length();
        auto dstLength = destinationArray->get_Length();

        if ((sourceIndex + length) > srcLength) throw ArrayException(sourceArray, "Attempted to copy more elements than available");
        if ((destinationIndex + length) > dstLength) throw ArrayException(destinationArray, "Attempted to copy elements into an array that was too short");

        // at this point, src and dst are both valid, and we know we have enough elements, and we have enough space to fit those elements
        auto src = std::next(sourceArray->begin(), sourceIndex);
        auto dst = std::next(destinationArray->begin(), destinationIndex);
        std::copy_n(src, length, dst);
    }

    int IndexOf(T item) {
        auto itr = std::find_if(begin(), end(), [&item](auto& x){ return x == item; });

        if (itr == end()) return -1;
        return itr - begin();
    }

    #ifdef HAS_CODEGEN
        explicit constexpr operator ::System::Collections::ICollection*() noexcept { return static_cast<::System::Collections::ICollection*>(static_cast<void*>(this)); }
        explicit constexpr operator ::System::Collections::IEnumerable*() noexcept { return static_cast<::System::Collections::IEnumerable*>(static_cast<void*>(this)); }
        explicit constexpr operator ::System::Collections::IList*() noexcept { return static_cast<::System::Collections::IList*>(static_cast<void*>(this)); }
        explicit constexpr operator ::System::Collections::IStructuralComparable*() noexcept { return static_cast<::System::Collections::IStructuralComparable*>(static_cast<void*>(this)); }
        explicit constexpr operator ::System::Collections::IStructuralEquatable*() noexcept { return static_cast<::System::Collections::IStructuralEquatable*>(static_cast<void*>(this)); }
        explicit constexpr operator ::System::ICloneable*() noexcept { return static_cast<::System::ICloneable*>(static_cast<void*>(this)); }
    #endif
};
MARK_GEN_REF_PTR_T(Array);

template <typename TArg>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<Array<TArg>*> {
    static inline Il2CppClass* get() {
        static Il2CppClass* klass;
        if (klass) return klass;

        il2cpp_functions::Init();
        if constexpr (std::is_same_v<std::decay_t<TArg>, Il2CppObject*>) {
            il2cpp_functions::CheckS_GlobalMetadata();
            klass = il2cpp_functions::array_class_get(il2cpp_functions::defaults->object_class, 1);
        } else {
            auto& logger = getLogger();
            Il2CppClass* eClass = RET_0_UNLESS(logger, il2cpp_no_arg_class<TArg>::get());
            klass = il2cpp_functions::array_class_get(eClass, 1);
        }
        return klass;
    }
};

template<typename T, class Ptr = Array<T>*>
/// @brief An Array wrapper type that is responsible for holding an (ideally valid) pointer to an array on the GC heap.
/// Allows for C++ array semantics. Ex, [], begin(), end(), etc...
struct ArrayW {
    static_assert(sizeof(Ptr) == sizeof(void*), "Size of Ptr type must be the same as a void*!");

    /// @brief Create an ArrayW from a pointer
    constexpr ArrayW(Ptr initVal) noexcept : val(initVal) {}
    /// @brief Create an ArrayW from an arbitrary pointer
    constexpr ArrayW(void* alterInit) noexcept : val(reinterpret_cast<Ptr>(alterInit)) {}
    /// @brief Constructs an ArrayW that wraps a null value
    constexpr ArrayW(std::nullptr_t nptr) noexcept : val(nptr) {}
    /// @brief Default constructor wraps a nullptr array
    constexpr ArrayW() noexcept : val(nullptr) {}
    template<class U>
    requires (!std::is_same_v<std::nullptr_t, U> && std::is_convertible_v<U, T>)
    ArrayW(std::initializer_list<U> vals) : val(Array<T>::New(vals)) {}
    ArrayW(il2cpp_array_size_t size) : val(Array<T>::NewLength(size)) {}

    inline il2cpp_array_size_t size() const noexcept {
        return val->get_Length();
    }

    using value = T;
    using const_value = const T;
    using pointer = T*;
    using const_pointer = T const*;
    using reference = T&;
    using const_reference = T const&;

    using iterator = pointer;
    using const_iterator = const_pointer;

    /// @brief forward begin
    iterator begin() { return iterator(val->_values + 0); }
    /// @brief forward end
    iterator end() { return iterator(val->_values + size()); }
    /// @brief reverse begin
    auto rbegin() { return std::reverse_iterator(iterator(val->_values + size())); }
    /// @brief reverse end
    auto rend() { return std::reverse_iterator(iterator(val->_values + 0)); }

    /// @brief forward const begin
    const_iterator begin() const { return const_iterator(val->_values + 0); }
    /// @brief forward const end
    const_iterator end() const { return const_iterator(val->_values + size()); }
    /// @brief reverse const begin
    auto rbegin() const { return std::reverse_iterator(const_iterator(val->_values + size())); }
    /// @brief reverse const end
    auto rend() const { return std::reverse_iterator(const_iterator(val->_values + 0)); }

    /// @brief index into array
    reference operator[](size_t i) noexcept {
        return val->_values[i];
    }
    /// @brief const index into array
    const_reference operator[](size_t i) const noexcept {
        return val->_values[i];
    }

    /// @brief assert sizes
    inline void assert_bounds(size_t i) {
        if (i < 0 || i >= size()) {
            throw ArrayException(this, string_format("%zu is out of bounds for array of length: %zu", i, size()));
        }
    }

    /// @brief Get a given index, performs bound checking and throws std::runtime_error on failure.
    /// @param i The index to get.
    /// @return The reference to the item.
    reference get(size_t i) {
        assert_bounds(i);
        return val->_values[i];
    }
    /// @brief Get a given index, performs bound checking and throws std::runtime_error on failure.
    /// @param i The index to get.
    /// @return The const reference to the item.
    const_reference get(size_t i) const {
        assert_bounds(i);
        return val->_values[i];
    }
    /// @brief Tries to get a given index, performs bound checking and returns a std::nullopt on failure.
    /// @param i The index to get.
    /// @return The WrapperRef<T> to the item, mostly considered to be a T&.
    std::optional<WrapperRef<value>> try_get(size_t i) noexcept {
        if (i >= size() || i < 0) {
            return std::nullopt;
        }
        return WrapperRef<value>(val->_values[i]);
    }
    /// @brief Tries to get a given index, performs bound checking and returns a std::nullopt on failure.
    /// @param i The index to get.
    /// @return The WrapperRef<const T> to the item, mostly considered to be a const T&.
    std::optional<WrapperRef<const_value>> try_get(size_t i) const noexcept {
        if (i >= size() || i < 0) {
            return std::nullopt;
        }
        return const_value(val->_values + i);
    }

    iterator find(T&& item) {
        return std::find_if(begin(), end(), [&item](auto& x){ return x == item; });
    }

    const_iterator find(T&& item) const {
        return std::find_if(begin(), end(), [&item](auto& x){ return x == item; });
    }

    auto rfind(T&& item) {
        return std::find_if(rbegin(), rend(), [&item](auto& x){ return x == item; });
    }

    auto rfind(T&& item) const {
        return std::find_if(rbegin(), rend(), [&item](auto& x){ return x == item; });
    }

    template<typename Predicate>
    iterator find_if(Predicate&& pred) {
        return std::find_if(begin(), end(), pred);
    }

    template<typename Predicate>
    const_iterator find_if(Predicate&& pred) const {
        return std::find_if(begin(), end(), pred);
    }

    template<typename Predicate>
    auto rfind_if(Predicate&& pred) {
        return std::find_if(rbegin(), rend(), pred);
    }

    template<typename Predicate>
    auto rfind_if(Predicate&& pred) const {
        return std::find_if(rbegin(), rend(), pred);
    }

    std::optional<T> front() {
        if (size() > 0)
            return get(0);
        else
            return std::nullopt;
    }

    T front_or_default() requires(std::is_default_constructible_v<T>) {
        return front().value_or(T{});
    }

    template<typename Predicate>
    std::optional<T> front(Predicate&& pred) {
        auto itr = std::find_if(begin(), end(), pred);
        if (itr == end()) return std::nullopt;
        return *itr;
    }

    template<typename Predicate>
    T front_or_default(Predicate&& pred) requires(std::is_default_constructible_v<T>) {
        auto itr = std::find_if(begin(), end(), pred);

        if (itr == end()) return T{};
        return *itr;
    }

    std::optional<T> back() {
        if (size() > 0)
            return get(size() - 1);
        else
            return std::nullopt;
    }

    T back_or_default() requires(std::is_default_constructible_v<T>) {
        return back().value_or(T{});
    }

    template<typename Predicate>
    std::optional<T> back(Predicate&& pred) {
        auto itr = std::find_if(rbegin(), rend(), pred);
        if (itr == rend()) return std::nullopt;
        return *itr;
    }

    template<typename Predicate>
    T back_or_default(Predicate&& pred) requires(std::is_default_constructible_v<T>) {
        auto itr = std::find_if(rbegin(), rend(), pred);

        if (itr == rend()) return T{};
        return *itr;
    }

    bool contains(T item) {
        auto itr = std::find_if(begin(), end(), [&item](auto& x){ return x == item; });
        return itr != end();
    }

    void copy_to(std::span<T> destination, int index) {
        auto dstLength = destination.size();
        if (index + size() > dstLength) throw ArrayException(val, "Can't copy into destination span that's too short");

        // at this point we know our destination can take our full length
        auto dstBegin = std::next(destination.begin(), index);
        std::copy_n(begin(), size(), dstBegin);
    }

    std::optional<int> index_of(T item) {
        auto itr = std::find_if(begin(), end(), [&item](auto& x){ return x == item; });
        if (itr == end()) return std::nullopt;
        return itr - begin();
    }

    /// @brief Provides a reference span of the held data within this array. The span should NOT outlive this instance.
    /// @return The created span.
    std::span<T> ref_to() {
        return std::span(val->_values, size());
    }
    /// @brief Provides a reference span of the held data within this array. The span should NOT outlive this instance.
    /// @return The created span.
    std::span<const T> ref_to() const {
        return std::span(const_cast<T*>(val->_values), size());
    }

    operator bool() const noexcept { return val != nullptr; }

    explicit operator const Ptr() const { return val; }
    explicit operator Ptr() { return val; }
    explicit operator Il2CppArray*() {
        return reinterpret_cast<Il2CppArray*>(val);
    }
    explicit operator Il2CppArray* const() const {
        return reinterpret_cast<Il2CppArray* const>(val);
    }
    constexpr const Ptr operator -> () const noexcept { return val; }
    constexpr Ptr operator -> () noexcept { return val; }

    static ArrayW<T, Ptr> Empty() noexcept {
        return ArrayW<T, Ptr>(il2cpp_array_size_t(0));
    }

    constexpr void* convert() const noexcept {
        return const_cast<void*>(static_cast<void*>(val));
    }

    #ifdef HAS_CODEGEN
        explicit constexpr operator ::System::Collections::ICollection*() noexcept { return static_cast<::System::Collections::ICollection*>(convert()); }
        explicit constexpr operator ::System::Collections::IEnumerable*() noexcept { return static_cast<::System::Collections::IEnumerable*>(convert()); }
        explicit constexpr operator ::System::Collections::IList*() noexcept { return static_cast<::System::Collections::IList*>(convert()); }
        explicit constexpr operator ::System::Collections::IStructuralComparable*() noexcept { return static_cast<::System::Collections::IStructuralComparable*>(convert()); }
        explicit constexpr operator ::System::Collections::IStructuralEquatable*() noexcept { return static_cast<::System::Collections::IStructuralEquatable*>(convert()); }
        explicit constexpr operator ::System::ICloneable*() noexcept { return static_cast<::System::ICloneable*>(convert()); }
    #endif

    private:
    Ptr val;
};
MARK_GEN_REF_T(ArrayW);

static_assert(il2cpp_utils::has_il2cpp_conversion<ArrayW<int, Array<int>*>>);
template<class T, class Ptr>
struct ::il2cpp_utils::il2cpp_type_check::need_box<ArrayW<T, Ptr>> {
    constexpr static bool value = false;
};

template<class T, class Ptr>
struct ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<ArrayW<T, Ptr>> {
    static inline Il2CppClass* get() {
        auto klass = ::il2cpp_utils::il2cpp_type_check::il2cpp_no_arg_class<Array<T>*>::get();
        return klass;
    }
};

#pragma pack(pop)

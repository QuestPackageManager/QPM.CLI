#pragma once

#include <optional>
#include <type_traits>
#include <variant>

#include "il2cpp-utils-exceptions.hpp"

namespace il2cpp_utils {

namespace exceptions {
struct ResultException : il2cpp_utils::exceptions::StackTraceException {
    ResultException(std::string_view msg) : il2cpp_utils::exceptions::StackTraceException(msg) {}
};
}  // namespace exceptions

namespace {

/// Converts void to std::monostate for variant
template <typename T>
using TypeOrMonostate = std::conditional_t<std::is_same_v<T, void>, std::monostate, T>;

template <typename T, typename E>
    requires(!std::is_same_v<E, void>)
struct Result {
    using SuccessValue = TypeOrMonostate<T>;
    using ExceptionValue = E;

    Result()
        requires(std::is_default_constructible_v<SuccessValue> || std::is_void_v<T>)
        : Result(SuccessValue()) {}

    Result(Result&&) noexcept = default;
    Result(Result const&) noexcept = default;

    Result(SuccessValue&& result) noexcept : success(true), result(std::forward<SuccessValue>(result)) {}

    Result(SuccessValue const& result) noexcept : success(true), result(std::forward<SuccessValue>(result)) {}

    /// exception
    Result(E&& exception) noexcept : success(false), exception(std::forward<E>(exception)) {}
    Result(E const& exception) noexcept : success(false), exception(std::forward<E>(exception)) {}

    ~Result() {
        if (success) {
            this->result.~SuccessValue();

        } else {
            this->exception.~ExceptionValue();
        }
    }

    [[nodiscard]] inline bool has_result() const noexcept {
        return success;
    }
    [[nodiscard]] inline bool has_exception() const noexcept {
        return !success;
    }

    [[nodiscard]] constexpr SuccessValue const& get_result() const {
        if (!success) throw il2cpp_utils::exceptions::ResultException("Result does not contain a success result!");

        return result;
    }

    [[nodiscard]] constexpr SuccessValue& get_result() {
        if (!success) throw il2cpp_utils::exceptions::ResultException("Result does not contain a success result!");

        return result;
    }

    /// move result value out of this wrapper
    [[nodiscard]] constexpr SuccessValue move_result() {
        if (!success) throw il2cpp_utils::exceptions::ResultException("Result does not contain a success result!");

        return std::move(result);
    }

    [[nodiscard]] constexpr ExceptionValue const& get_exception() const {
        if (success) throw il2cpp_utils::exceptions::ResultException("Result does not contain an exception result!");

        return exception;
    }
    [[nodiscard]] constexpr ExceptionValue& get_exception() {
        if (success) throw il2cpp_utils::exceptions::ResultException("Result does not contain an exception result!");

        return exception;
    }

    /// move result value out of this wrapper
    [[nodiscard]] constexpr ExceptionValue move_exception() {
        if (success) throw il2cpp_utils::exceptions::ResultException("Result does not contain an exception result!");

        return std::move(exception);
    }

    /// Gets the current success result or rethrows if an exception
    [[nodiscard]] constexpr SuccessValue const& get_or_rethrow() const {
        if (!success) {
            throw this->get_exception();
        }

        return this->get_result();
    }
    /// Gets the current success result or rethrows if an exception
    [[nodiscard]] constexpr SuccessValue& get_or_rethrow() {
        if (!success) {
            throw this->get_exception();
        }

        return this->get_result();
    }
    /// Gets the current success result or rethrows if an exception
    constexpr void rethrow() const {
        if (success) {
            return;
        }
        throw this->get_exception();
    }

    /// Moves this result into a variant
    /// if T is void, returns std::monostate
    [[nodiscard]] constexpr std::variant<SuccessValue, ExceptionValue> into_variant() noexcept {
        if (success) {
            return this->move_result();
        }

        return this->move_exception();
    }

#pragma region Optional Result
    /// Moves this result into an optional
    /// if T is void, returns std::monostate
    [[nodiscard]] constexpr std::optional<SuccessValue> into_optional_result() noexcept {
        if (success) {
            return this->move_result();
        }

        return std::nullopt;
    }

    /// Moves this result into an optional
    /// if T is void, returns std::monostate
    [[nodiscard]] constexpr std::optional<SuccessValue*> as_optional_result() noexcept {
        if (success) {
            return &this->get_result();
        }

        return std::nullopt;
    }
    /// Moves this result into an optional
    /// if T is void, returns std::monostate
    [[nodiscard]] constexpr std::optional<SuccessValue const*> as_optional_result() const noexcept {
        if (success) {
            return &this->get_result();
        }

        return std::nullopt;
    }
#pragma endregion

#pragma region Optional Exception
    /// Moves this result into an optional
    /// if T is void, returns std::monostate
    [[nodiscard]] constexpr std::optional<E> into_optional_exception() noexcept {
        if (!success) {
            return this->move_exception();
        }

        return std::nullopt;
    }

    /// Wraps this exception into an optional
    [[nodiscard]] constexpr std::optional<E*> as_optional_exception() noexcept {
        if (!success) {
            return &this->get_exception();
        }

        return std::nullopt;
    }
    /// Wraps this exception into an optional
    [[nodiscard]] constexpr std::optional<E const*> as_optional_exception() const noexcept {
        if (!success) {
            return &this->get_exception();
        }

        return std::nullopt;
    }
#pragma endregion

   private:
    bool success;

    union {
        SuccessValue result;
        ExceptionValue exception;
    };
};
}  // namespace
}  // namespace il2cpp_utils
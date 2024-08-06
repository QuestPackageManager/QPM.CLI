#ifndef IL2CPP_UTILS_EXCEPTIONS
#define IL2CPP_UTILS_EXCEPTIONS

#include <exception>
#include <string_view>
#include <string>
#include <stdexcept>
#include <typeinfo>
#include "utils-functions.h"

struct Il2CppException;
struct MethodInfo;
struct Il2CppClass;
struct Il2CppObject;


namespace il2cpp_utils {
    // forward declare
    std::string ClassStandardName(const Il2CppClass* klass, bool generics);

    namespace exceptions {
        // TODO: Move all custom exceptions to this namespace?
        struct StackTraceException : std::runtime_error {
            constexpr static uint16_t STACK_TRACE_SIZE = 256;

            void* stacktrace_buffer[STACK_TRACE_SIZE];
            uint16_t stacktrace_size;

            StackTraceException(std::string_view msg) : std::runtime_error(msg.data()) {
                // TODO: Eventually skip two frames (assuming no inlined methods) for this constructor and the captured backtrace call.
                stacktrace_size = backtrace_helpers::captureBacktrace(stacktrace_buffer, STACK_TRACE_SIZE, 0);
            }
            void log_backtrace() const;

            [[nodiscard]] virtual char const* what() const noexcept override {
                log_backtrace();
                return std::runtime_error::what();
            }
        };

        struct NullException : public il2cpp_utils::exceptions::StackTraceException {
            using StackTraceException::StackTraceException;

            NullException(std::string_view msg) : il2cpp_utils::exceptions::StackTraceException(msg) {}
        };
        struct BadCastException : public il2cpp_utils::exceptions::StackTraceException {
            using StackTraceException::StackTraceException;
            ::Il2CppClass const* klass;
            ::Il2CppClass const* targetKlass;
            Il2CppObject* inst;

            BadCastException(::Il2CppClass const* klass, ::Il2CppClass const* targetKlass, Il2CppObject* inst)
                : il2cpp_utils::exceptions::StackTraceException(string_format("Failed to cast %s to %s", ClassStandardName(klass, true).c_str(),
                                                                              ClassStandardName(targetKlass, true).c_str())),
                  klass(klass),
                  targetKlass(targetKlass),
                  inst(inst) {}
        };
    }
    // Returns a legible string from an Il2CppException*
    ::std::string ExceptionToString(const Il2CppException* exp) noexcept;

    #if defined(UNITY_2019) || defined(UNITY_2021)
    /// @brief Raises the provided Il2CppException to be used within il2cpp.
    /// @param exp The exception instance to throw
    [[noreturn]] void raise(const Il2CppException* exp);
    #endif

    #if __has_feature(cxx_exceptions)
    struct Il2CppUtilsException : exceptions::StackTraceException {
        std::string context;
        std::string msg;
        std::string func = "unknown";
        std::string file = "unknown";
        int line = -1;
        Il2CppUtilsException(std::string_view msg_) : exceptions::StackTraceException(CreateMessage(msg_.data())), msg(msg_.data()) {}
        Il2CppUtilsException(std::string_view context_, std::string_view msg_) : exceptions::StackTraceException(CreateMessage(msg_.data(), context_.data())), context(context_.data()), msg(msg_.data()) {}
        Il2CppUtilsException(std::string_view msg_, std::string_view func_, std::string_view file_, int line_)
            : exceptions::StackTraceException(CreateMessage(msg_.data(), "", func_.data(), file_.data(), line_)), msg(msg_.data()), func(func_.data()), file(file_.data()), line(line_) {}
        Il2CppUtilsException(std::string_view context_, std::string_view msg_, std::string_view func_, std::string_view file_, int line_)
            : exceptions::StackTraceException(CreateMessage(msg_.data(), context_.data(), func_.data(), file_.data(), line_)), context(context_.data()), msg(msg_.data()), func(func_.data()), file(file_.data()), line(line_) {}

        std::string CreateMessage(std::string msg, std::string context = "", std::string func = "unknown", std::string file = "unknown", int line = -1) {
            return ((context.size() > 0 ? ("(" + context + ") ") : "") + msg + " in: " + func + " " + file + ":" + std::to_string(line));
        }
    };
    struct RunMethodException : std::runtime_error {
        constexpr static uint16_t STACK_TRACE_SIZE = 256;

        const Il2CppException* ex;
        const MethodInfo* info;
        void* stacktrace_buffer[STACK_TRACE_SIZE];
        uint16_t stacktrace_size;

        RunMethodException(std::string_view msg, const MethodInfo* inf) __attribute__((noinline)) : std::runtime_error(msg.data()), ex(nullptr), info(inf) {
            // TODO: Eventually skip two frames (assuming no inlined methods) for this constructor and the captured backtrace call.
            stacktrace_size = backtrace_helpers::captureBacktrace(stacktrace_buffer, STACK_TRACE_SIZE, 0);
        }
        RunMethodException(Il2CppException* exp, const MethodInfo* inf) __attribute__((noinline)) : std::runtime_error(ExceptionToString(exp).c_str()), ex(exp), info(inf) {
            // TODO: Eventually skip two frames (assuming no inlined methods) for this constructor and the captured backtrace call.
            stacktrace_size = backtrace_helpers::captureBacktrace(stacktrace_buffer, STACK_TRACE_SIZE, 0);
        }
        // TODO: Add a logger argument here so we could better write out to a targetted buffer.
        // For now, we will stick to using the UtilsLogger.
        // It will be our caller's responsibility to determine what to do AFTER the backtrace is logged-- whether it be to terminate or rethrow.
        // Logs the backtrace with the Logging::ERROR level, using the global logger instance.
        void log_backtrace() const;
        [[noreturn]] void rethrow() const {
            #if defined(UNITY_2019) || defined(UNITY_2021)
            il2cpp_utils::raise(ex);
            #else
            #warning "The exception being rethrown like this is unlikely to behave correctly!"
            throw Il2CppExceptionWrapper(ex);
            #endif
        }

        [[nodiscard]] virtual char const* what() const noexcept override {
            log_backtrace();
            return std::runtime_error::what();
        }
    };
    #endif
}

#ifdef MOD_ID
#define _CATCH_HANDLER_ID MOD_ID
#else
#define _CATCH_HANDLER_ID "UNKNOWN"
#endif

// Implements a try-catch handler which will first attempt to run the provided body.
// If there is an uncaught RunMethodException, it will first attempt to log the backtrace.
// If it holds a valid C# exception, it will attempt to raise it, such that it is caught in the il2cpp domain.
// If an exception is thrown that is otherwise what-able is caught, it will attempt to call the what() method
// and then rethrow the exception to the il2cpp domain.
// If an unknown exception is caught, it will terminate explicitly, as opposed to letting it be thrown across the il2cpp domain.
// All logs that occur as a result of this function will be under the core beatsaber-hook global logger.
#define IL2CPP_CATCH_HANDLER(...) try { \
    __VA_ARGS__ \
} catch (::il2cpp_utils::RunMethodException const& exc) { \
    ::Logger::get().error("Caught in mod ID: " _CATCH_HANDLER_ID ": Uncaught RunMethodException! what(): %s", exc.what()); \
    exc.log_backtrace(); \
    ::Logger::get().error("Catch handler backtrace..."); \
    ::Logger::get().Backtrace(100); \
    if (exc.ex) { \
        exc.rethrow(); \
    } \
    SAFE_ABORT(); \
} catch (::il2cpp_utils::exceptions::StackTraceException const& exc) { \
    ::Logger::get().error("Caught in mod ID: " _CATCH_HANDLER_ID ": Uncaught StackTraceException! what(): %s", exc.what()); \
    exc.log_backtrace(); \
    ::Logger::get().error("Catch handler backtrace..."); \
    ::Logger::get().Backtrace(100); \
    SAFE_ABORT(); \
} catch (::std::exception const& exc) { \
    ::Logger::get().error("Caught in mod ID: " _CATCH_HANDLER_ID ": Uncaught C++ exception! type name: %s, what(): %s", typeid(exc).name(), exc.what()); \
    ::Logger::get().error("Catch handler backtrace..."); \
    ::Logger::get().Backtrace(100); \
    ::il2cpp_utils::raise(exc); \
} catch (...) { \
    ::Logger::get().error("Caught in mod ID: " _CATCH_HANDLER_ID ": Uncaught, unknown C++ exception (not std::exception) with no known what() method!"); \
    ::Logger::get().error("Catch handler backtrace..."); \
    ::Logger::get().Backtrace(100); \
    SAFE_ABORT(); \
}

#endif

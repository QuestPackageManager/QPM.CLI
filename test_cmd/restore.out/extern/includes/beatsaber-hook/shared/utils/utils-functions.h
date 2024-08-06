#ifndef UTILS_FUNCTIONS_H
#define UTILS_FUNCTIONS_H

#include <stdio.h>
#include <stdlib.h>
#include <string>
#include <string_view>
#include <vector>
#include <unwind.h>

// logs the function, file and line, sleeps to allow logs to flush, then terminates program
__attribute__((noreturn)) void safeAbort(const char* func, const char* file, int line, uint16_t frameCount = 512);
// logs the function, file and line, and provided message, sleeps to allow logs to flush, then terminates program
__attribute__((noreturn)) __attribute__((format(printf, 4, 5))) void safeAbortMsg(const char* func, const char* file, int line, const char* fmt, ...);
// sets "file" and "line" to the file and line you call this macro from
#ifndef SUPPRESS_MACRO_LOGS
#define SAFE_ABORT() safeAbort(__PRETTY_FUNCTION__, __FILE__, __LINE__)
#else
#define SAFE_ABORT() safeAbort("undefined_function", "undefined_file", -1)
#endif

#ifndef SUPPRESS_MACRO_LOGS
#define SAFE_ABORT_MSG(...) safeAbortMsg(__PRETTY_FUNCTION__, __FILE__, __LINE__, __VA_ARGS__)
#else
#define SAFE_ABORT_MSG(...) safeAbortMsg("undefined_function", "undefined_file", -1, __VA_ARGS__)
#endif

struct Il2CppString;
#ifndef __cplusplus
bool = uchar8_t;
#endif /* __cplusplus */

// va_list wrapper for string_format
std::string string_vformat(const std::string_view format, va_list args);
// Returns a string_view of the given Il2CppString*
std::u16string_view csstrtostr(Il2CppString* in);
// Sets the given cs_string using the given string/char16 array
void setcsstr(Il2CppString* in, std::u16string_view str);
// Converts a UTF16 string to a UTF8 string
std::string to_utf8(std::u16string_view view);
// Converts a UTF8 string to a UTF16 string
std::u16string to_utf16(std::string_view view);
// Dumps the 'before' bytes before and 'after' bytes after the given pointer to log
void dump(int before, int after, void* ptr);
// Reads all of the text of a file at the given filename. If the file does not exist, returns an empty string.
std::string readfile(std::string_view filename);
// Reads all bytes from the provided file at the given filename. If the file does not exist, returns an empty vector.
std::vector<char> readbytes(std::string_view filename);
// Writes all of the text to a file at the given filename. Returns true on success, false otherwise
bool writefile(std::string_view filename, std::string_view text);
// Deletes a file at the given filename. Returns true on success, false otherwise
bool deletefile(std::string_view filename);
// Returns if a file exists and can be written to / read from
bool fileexists(std::string_view filename);
// Returns if a directory exists and can be written to / read from
bool direxists(std::string_view dirname);
// Yoinked from: https://stackoverflow.com/questions/2342162/stdstring-formatting-like-sprintf
// TODO: This should be removed once std::format exists
__attribute__((format(printf, 1, 2))) std::string string_format(const char* format, ...);

/// @brief Get the size of the libil2cpp.so file
/// @returns The size of the .so
uintptr_t getLibil2cppSize();

namespace backtrace_helpers {
    struct BacktraceState {
        void **current;
        void **end;
        uint16_t skip;
    };
    _Unwind_Reason_Code unwindCallback(struct _Unwind_Context *context, void *arg);
    size_t captureBacktrace(void **buffer, uint16_t max, uint16_t skip = 0);
}

#endif /* UTILS_FUNCTIONS_H */

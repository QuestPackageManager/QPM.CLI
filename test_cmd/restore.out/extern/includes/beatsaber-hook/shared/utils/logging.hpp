#pragma once

#include <android/log.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <algorithm>
#include <cmath>  // Included to support cmath's definition of log
#include <list>
#include <mutex>
#include <string>
#include <string_view>
#include <thread>
#include <unordered_map>
#include <unordered_set>
#include "utils-functions.h"

#include "scotland2/shared/loader.hpp"
#include "scotland2/shared/modloader.h"

namespace Logging {
enum Level { CRITICAL = ANDROID_LOG_FATAL, ERROR = ANDROID_LOG_ERROR, WARNING = ANDROID_LOG_WARN, INFO = ANDROID_LOG_INFO, DEBUG = ANDROID_LOG_DEBUG };
}

#ifdef log
#undef log
#endif

// Note that the LOG_PATH directory is still the files directory, so that stale logs will be cleaned up automatically.
#ifndef LOG_PATH
#define LOG_PATH "/sdcard/ModData/%s/logs/"
#endif

class Logger;

/// @class Logger Buffer
/// @brief A buffer for logger data. Used for logging to file in a buffered fashion.
/// Each LoggerBuffer exists to wrap around a single logger instance.
/// Every time log is called on the instance, this buffer is updated (assuming options.toFile is true for the instance)
/// Each buffer contains multiple messages that need to be written out, stored as an std::list
class LoggerBuffer {
    friend Logger;

   public:
    std::list<std::string> messages;
    std::mutex messageLock;
    const modloader::ModInfo modInfo;
    bool closed = false;
    static std::string get_logDir();
    std::string get_path() {
        std::string cpy = modInfo.version;
        std::replace(cpy.begin(), cpy.end(), '.', '_');
        auto val = get_logDir() + modInfo.id + "_" + cpy + ".log";
        return val;
    }
    std::size_t length();
    void addMessage(std::string_view msg);
    void flush();

   private:
    std::string path;

   public:
    LoggerBuffer(const modloader::ModInfo info) : modInfo(info), path(get_path()) {}
};

/// @struct Logger Options
/// @brief Provides various options for loggers, including silencing them and logging to file.
struct LoggerOptions {
    bool silent = false;
    bool toFile = false;
    std::string contextSeparator = "::";
    LoggerOptions(bool silent_ = false, bool toFile_ = false) : silent(silent_), toFile(toFile_) {}
    LoggerOptions(std::string_view contextSeparator_, bool silent_ = false, bool toFile_ = false) : silent(silent_), toFile(toFile_), contextSeparator(contextSeparator_) {}
};

class LoggerContextObject;
class Consumer;

class Logger {
    friend Consumer;
    friend LoggerBuffer;
    friend LoggerContextObject;

   public:
    Logger(const modloader::ModInfo info, LoggerOptions options_) : options(options_), tag("QuestHook[" + info.id + "|v" + info.version + "]"), modInfo(info), buffer(modInfo) {
        if (!init()) {
            buffer.closed = true;
        }
        emplace_safe(buffer);
    }
    Logger(const modloader::ModInfo info) : options(LoggerOptions(false, false)), tag("QuestHook[" + info.id + "|v" + info.version + "]"), modInfo(info), buffer(modInfo) {
        if (!init()) {
            buffer.closed = true;
        }
        emplace_safe(buffer);
    }
    ~Logger() = delete;
    void log(Logging::Level lvl, std::string str);
    __attribute__((format(printf, 3, 4))) void log(Logging::Level lvl, const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(lvl, string_vformat(fmt, lst));
        va_end(lst);
    }
    __attribute__((format(printf, 2, 3))) void critical(const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(Logging::CRITICAL, string_vformat(fmt, lst));
        va_end(lst);
    }
    __attribute__((format(printf, 2, 3))) void error(const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(Logging::ERROR, string_vformat(fmt, lst));
        va_end(lst);
    }
    __attribute__((format(printf, 2, 3))) void warning(const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(Logging::WARNING, string_vformat(fmt, lst));
        va_end(lst);
    }
    __attribute__((format(printf, 2, 3))) void info(const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(Logging::INFO, string_vformat(fmt, lst));
        va_end(lst);
    }
    __attribute__((format(printf, 2, 3))) void debug(const char* fmt, ...) {
        if (options.silent) {
            return;
        }
        va_list lst;
        va_start(lst, fmt);
        log(Logging::DEBUG, string_vformat(fmt, lst));
        va_end(lst);
    }
    /// @brief Flushes the buffer for this logger instance.
    void flush();
    /// @brief Closes the buffer for this logger instance, flushing as necessary.
    /// After this call, this logger will no longer log to a buffer, nor will it log to a file.
    void close();
    /// @brief Returns the logger that is used within the utils library. This function should not be used outside of the main library
    static Logger& get();
    /// @brief Close all open LoggerBuffer objects. Should only be called on a crash or exit of the game.
    static void closeAll();
    /// @brief Flush all open LoggerBuffer objects.
    static void flushAll();
    /// @brief Initialize this logger. Deletes existing file logs.
    /// This happens on default when this instance is constructed.
    /// This should also be called anytime the options field is modified.
    /// @returns True if the initialization was successful, false otherwise. If false is returned, you should set buffer.closed to true.
    bool init();
    /// @brief Call this to silence logs from this logger. Should improve performance slightly.
    /// Note that this call causes ALL calls to this particular logger to be silent, including from other mods.
    /// Should only be used in particular cases.
    constexpr void disable() {
        options.silent = true;
    }
    /// @brief Call this to re-enable logs for this logger. Decreases performance slightly, but provides debug information.
    /// Note that this call causes ALL calls to this particular logger to be enabled again, including from other mods that want silence.
    /// Should only be used in particular cases.
    constexpr void enable() {
        options.silent = false;
    }
    /// @brief Returns the current options for this logger
    const LoggerOptions getOptions() const {
        return options;
    }
    /// @brief Writes a backtrace for the provided number of frames.
    /// @param frameCount The number of frames to backtrace
    void Backtrace(uint16_t frameCount);
    /// @brief Enters a logging context. Should be used for more specific logging information.
    /// Avoid entering contexts with names that contain % characters.
    /// @param context The context name to enter
    /// @returns LoggerContextObject that is used within the context.
    LoggerContextObject WithContext(std::string_view context);
    /// @brief Disable logging for any contexts that start with the provided string.
    /// This is thread independent, and will silence across all threads
    void DisableContext(std::string_view context);
    /// @brief Enables logging for the context.
    /// This function does nothing if DisableContext was not called with an exactly matching string before this call.
    void EnableContext(std::string_view context);
    /// @brief Gets all disabled contexts on this instance.
    /// @returns The set of disabled context strings.
    const std::unordered_set<std::string> GetDisabledContexts();

   private:
    /// @brief The options associated with this logger
    LoggerOptions options;

    std::unordered_set<std::string> disabledContexts;
    /// @brief All created contexts for this instance
    std::vector<LoggerContextObject*> contexts;
    /// @brief The mutex for the contexts maps/sets
    std::mutex contextMutex;

    std::string tag;
    const modloader::ModInfo modInfo;
    // TODO: Each Logger instance is responsible for their own buffer.
    // This means that if a logger instance is disposed (for whatever reason) it needs to clear its buffer pointer from the buffers list.
    // This is done in the destructor, but for all intents and purposes, it doesn't need to happen at all.
    LoggerBuffer buffer;
    static bool consumerStarted;
    static std::list<LoggerBuffer*> buffers;
    static std::mutex bufferMutex;

    /// @brief Constructs a context with a parent. This is called by LoggerContextObject.WithContext.
    LoggerContextObject WithContext(LoggerContextObject* parent, std::string_view context);
    /// @brief Recurses over all children contexts and disables/enables if they start with context.
    void RecurseChangeContext(LoggerContextObject* ctx, std::string_view context, bool enable);

    static void emplace_safe(LoggerBuffer& buffer) {
        // Obtain lock
        std::scoped_lock<std::mutex> lock(bufferMutex);
        // Emplace, lock is released
        Logger::buffers.push_back(&buffer);
    }
    static void startConsumer();
};

class LoggerContextObject {
    friend Logger;
    // The actual message to indicate the context.
    std::string tag;
    bool enabled = true;

    /// @brief Iterates over all children and either enables or disables them.
    void changeChildren(bool enable) {
        for (auto* item : childrenContexts) {
            item->enabled = enable;
        }
    }

    LoggerContextObject* parentContext = nullptr;
    std::list<LoggerContextObject*> childrenContexts;

   public:
    /// @brief Constructs a LoggerContextObject. Should only be called from Logger.WithContext or LoggerContextObject.WithContext
    /// @param l Logger instance to use
    /// @param context_ The context for this object
    /// @param enabled_ If it is enabled or not
    LoggerContextObject(Logger& l, std::string_view context_, bool enabled_) : enabled(enabled_), logger(l), context(context_.data()) {
        tag.append("(").append(context_.data()).append(") ");
        std::scoped_lock<std::mutex> lock(logger.contextMutex);
        logger.contexts.push_back(this);
    }

    /// @brief Constructs a nested LoggerContextObject. Should only be called from Logger.WithContext or LoggerContextObject.WithContext
    /// @param parent LoggerContextObject parent
    /// @param context_ The context for this object
    /// @param enabled_ If it is enabled or not
    LoggerContextObject(LoggerContextObject* const parent, std::string_view context_, bool enabled_)
        : enabled(enabled_ && parent->enabled), parentContext(parent), logger(parent->logger), context(context_) {
        tag.append("(").append(context.data()).append(") ");
        parentContext->childrenContexts.push_back(this);
        std::scoped_lock<std::mutex> lock(logger.contextMutex);
        logger.contexts.push_back(this);
    }
    /// @brief Equality operator.
    bool operator==(const LoggerContextObject& other) const {
        return tag == other.tag && enabled == other.enabled && parentContext == other.parentContext;
    }
    /// @brief The Logger reference.
    Logger& logger;
    /// @brief The context of this LoggerContextObject
    const std::string context;
    /// @brief The parent context (or nullptr if there is no parent)
    constexpr const LoggerContextObject* getParent() {
        return parentContext;
    }
    /// @brief The children contexts (empty if there are no children)
    const std::list<LoggerContextObject*> getChildren() {
        return childrenContexts;
    }
    // Cannot copy a context object, see the move constructor instead
    LoggerContextObject(const LoggerContextObject&) = delete;
    /// @brief Move constructor. Modifies the Logger's contexts collection.
    LoggerContextObject(LoggerContextObject&& other)
        : tag(std::move(other.tag)),
          enabled(std::move(other.enabled)),
          parentContext(other.parentContext),
          childrenContexts(std::move(other.childrenContexts)),
          logger(other.logger),
          context(std::move(other.context)) {
        // Potential race condition if move is called, interrupted, and DisableContext is called (should not result in much issue, though)
        logger.contextMutex.try_lock();
        // We we move the context, we need to update the pointer in the contexts collection
        for (auto itr = logger.contexts.begin(); itr != logger.contexts.end(); ++itr) {
            if (*itr == &other) {
                logger.contexts.erase(itr);
                break;
            }
        }
        logger.contexts.push_back(this);
        logger.contextMutex.unlock();
    }
    /// @brief Destructor. Modified the Logger's contexts collection.
    ~LoggerContextObject() {
        // We delete all of our children
        childrenContexts.clear();
        // Then we remove ourselves from our parent
        if (parentContext) {
            parentContext->childrenContexts.remove(this);
        }
        // Remove ourselves from logger.contexts
        logger.contextMutex.lock();
        for (auto itr = logger.contexts.begin(); itr != logger.contexts.end(); ++itr) {
            if (*itr == this) {
                logger.contexts.erase(itr);
                break;
            }
        }
        logger.contextMutex.unlock();
    }

    void log(Logging::Level lvl, std::string str) const {
        if (enabled) {
            logger.log(lvl, tag + str);
        }
    }

    void log_v(Logging::Level lvl, std::string_view fmt, va_list lst) const {
        logger.log(lvl, tag + string_vformat(fmt, lst));
    }

    __attribute__((format(printf, 3, 4))) void log(Logging::Level lvl, const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(lvl, fmt, lst);
            va_end(lst);
        }
    }
    __attribute__((format(printf, 2, 3))) void critical(const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(Logging::CRITICAL, fmt, lst);
            va_end(lst);
        }
    }
    __attribute__((format(printf, 2, 3))) void error(const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(Logging::ERROR, fmt, lst);
            va_end(lst);
        }
    }
    __attribute__((format(printf, 2, 3))) void warning(const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(Logging::WARNING, fmt, lst);
            va_end(lst);
        }
    }
    __attribute__((format(printf, 2, 3))) void info(const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(Logging::INFO, fmt, lst);
            va_end(lst);
        }
    }
    __attribute__((format(printf, 2, 3))) void debug(const char* fmt, ...) const {
        if (enabled) {
            va_list lst;
            va_start(lst, fmt);
            log_v(Logging::DEBUG, fmt, lst);
            va_end(lst);
        }
    }
    /// @brief Writes a backtrace for the provided number of frames.
    /// @param frameCount The number of frames to backtrace
    void Backtrace(uint16_t frameCount);
    /// @brief Enter a new context. This call forwards to logger.WithContext(this, ctx).
    /// Avoid entering contexts with names that contain % characters.
    /// @param ctx The context name to enter
    /// @returns The LoggerContextObject in the context
    LoggerContextObject WithContext(std::string_view ctx) {
        auto tmp = logger.WithContext(this, ctx);
        // Copy over enabled
        tmp.enabled = enabled;
        return tmp;
    }
};

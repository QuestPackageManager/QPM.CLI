#pragma once
#include "../../shared/utils/utils.h"
#include "capstone/shared/capstone/capstone.h"
#include "capstone/shared/platform.h"
#include <array>
#include <tuple>
#include <optional>

namespace cs {
csh getHandle();

uint32_t* readb(const uint32_t* addr);

template<arm64_insn... args>
constexpr bool insnMatch(cs_insn* insn) {
    if constexpr (sizeof...(args) > 0) {
        return (((insn->id == args) || ...));
    }
    return false;
};

struct AddrSearchPair {
    AddrSearchPair(uint32_t const* addr_, uint32_t remSearchSize_) : addr(addr_), remSearchSize(remSearchSize_) {}
    uint32_t const* addr;
    uint64_t remSearchSize;
};

auto find_through_hooks(void const* hook, uint32_t initialSearchSize, auto&& func) {
    // First, check to see if we are hooked.
    Logger::get().debug("Finding through potential hook: %p and size: %u", hook, initialSearchSize);
    auto hooks = HookTracker::GetHooks(hook);
    if (!hooks.empty()) {
        uint32_t const* addr = hooks.front().original_data.data();
        uint32_t size = hooks.front().original_data.size() * sizeof(uint32_t);
        Logger::get().debug("Hook found (%s)! Original data: %p with size: %u", hooks.front().name.c_str(), addr, size);
        return func(cs::AddrSearchPair(addr, size), cs::AddrSearchPair(reinterpret_cast<uint32_t const*>(hook), initialSearchSize));
    }
    Logger::get().debug("No hook found! Searching: %p, %u", hook, initialSearchSize);
    return func(cs::AddrSearchPair(reinterpret_cast<uint32_t const*>(hook), initialSearchSize));
}

template<std::size_t sz, class F1, class F2>
decltype(auto) findNth(std::array<AddrSearchPair, sz>& addrs, uint32_t nToRetOn, int retCount, F1&& match, F2&& skip) {
    cs_insn* insn = cs_malloc(getHandle());
    for (std::size_t searchIdx = 0; searchIdx < addrs.size(); searchIdx++) {
        while (addrs[searchIdx].remSearchSize > 0) {
            auto ptr = reinterpret_cast<uint64_t>(addrs[searchIdx].addr);
            bool res = cs_disasm_iter(getHandle(), reinterpret_cast<const uint8_t**>(&addrs[searchIdx].addr), &addrs[searchIdx].remSearchSize, &ptr, insn);
            Logger::get().debug("%p diassemb: %s (rCount: %i, nToRetOn: %u, sz: %zu)", (void*)ptr, insn->mnemonic, retCount, nToRetOn, addrs[searchIdx].remSearchSize);
            if (res) {
                // Valid decode, so lets check to see if it is a match or we need to break.
                if (insn->id == ARM64_INS_RET) {
                    if (retCount == 0) {
                        // Early termination!
                        cs_free(insn, 1);
                        Logger::get().warning("Could not find: %u call at: %p within: %i rets! Found all of the rets first!", nToRetOn, addrs[searchIdx].addr, retCount);
                        return (decltype(match(insn)))std::nullopt;
                    }
                    retCount--;
                } else {
                    auto testRes = match(insn);
                    if (testRes) {
                        if (nToRetOn == 1) {
                            cs_free(insn, 1);
                            return testRes;
                        } else {
                            nToRetOn--;
                        }
                    } else if (skip(insn)) {
                        if (nToRetOn == 1) {
                            std::string name(insn->mnemonic);
                            cs_free(insn, 1);
                            Logger::get().warning("Found: %u match, at: %p within: %i rets, but the result was a %s! Cannot compute destination address!", nToRetOn, addrs[searchIdx].addr, retCount, name.c_str());
                            return (decltype(match(insn)))std::nullopt;
                        } else {
                            nToRetOn--;
                        }
                    }
                }
                // Other instructions are ignored silently
            }
            else {
                // Invalid instructions are ignored silently.
                // In order to skip these properly, we must increment our instructions, ptr, and size accordingly.
                addrs[searchIdx].remSearchSize -= 4;
                addrs[searchIdx].addr++;
            }
        }
        // We didn't find it. Let's instead look at the next address/size pair for a match.
        Logger::get().debug("Could not find: %u call at: %p within: %i rets at idx: %zu!", nToRetOn, addrs[searchIdx].addr, retCount, searchIdx);
    }
    // If we run out of bytes to parse, we fail
    cs_free(insn, 1);
    return (decltype(match(insn)))std::nullopt;
}

template<uint32_t nToRetOn, int retCount = -1, size_t szBytes = 4096, class F1, class F2>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNth(const uint32_t* addr, F1&& match, F2&& skip) {
    cs_insn* insn = cs_malloc(getHandle());
    auto ptr = reinterpret_cast<uint64_t>(addr);
    auto instructions = reinterpret_cast<const uint8_t*>(addr);

    int rCount = retCount;
    uint32_t nCalls = nToRetOn;
    size_t sz = szBytes;
    while (sz > 0) {
        bool res = cs_disasm_iter(getHandle(), &instructions, &sz, &ptr, insn);
        Logger::get().debug("%p diassemb: %s (rCount: %i, nCalls: %u, sz: %zu)", (void*)ptr, insn->mnemonic, rCount, nCalls, sz);
        if (res) {
            // Valid decode, so lets check to see if it is a match or we need to break.
            if (insn->id == ARM64_INS_RET) {
                if (rCount == 0) {
                    // Early termination!
                    cs_free(insn, 1);
                    Logger::get().warning("Could not find: %u call at: %p within: %i rets! Found all of the rets first!", nToRetOn, (void*)ptr, retCount);
                    return (decltype(match(insn)))std::nullopt;
                }
                rCount--;
            } else {
                auto testRes = match(insn);
                if (testRes) {
                    if (nCalls == 1) {
                        cs_free(insn, 1);
                        return testRes;
                    } else {
                        nCalls--;
                    }
                } else if (skip(insn)) {
                    if (nCalls == 1) {
                        std::string name(insn->mnemonic);
                        cs_free(insn, 1);
                        Logger::get().warning("Found: %u match, at: %p within: %i rets, but the result was a %s! Cannot compute destination address!", nToRetOn, (void*)ptr, retCount, name.c_str());
                        return (decltype(match(insn)))std::nullopt;
                    } else {
                        nCalls--;
                    }
                }
            }
            // Other instructions are ignored silently
        }
        else {
            // Invalid instructions are ignored silently.
            // In order to skip these properly, we must increment our instructions, ptr, and size accordingly.
            sz -= 4;
            ptr += 4;
            instructions += 4;
        }
    }
    // If we run out of bytes to parse, we fail
    cs_free(insn, 1);
    Logger::get().warning("Could not find: %u call at: %p within: %i rets, within size: %zu!", nToRetOn, addr, retCount, szBytes);
    return (decltype(match(insn)))std::nullopt;
}

template<uint32_t nToRetOn, auto match, auto skip, int retCount = -1, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNth(const uint32_t* addr) {
    cs_insn* insn = cs_malloc(getHandle());
    auto ptr = reinterpret_cast<uint64_t>(addr);
    auto instructions = reinterpret_cast<const uint8_t*>(addr);

    int rCount = retCount;
    uint32_t nCalls = nToRetOn;
    size_t sz = szBytes;
    while (sz > 0) {
        bool res = cs_disasm_iter(getHandle(), &instructions, &sz, &ptr, insn);
        Logger::get().debug("%p diassemb: %s (rCount: %i, nCalls: %u, sz: %zu)", (void*)ptr, insn->mnemonic, rCount, nCalls, sz);
        if (res) {
            // Valid decode, so lets check to see if it is a match or we need to break.
            if (insn->id == ARM64_INS_RET) {
                if (rCount == 0) {
                    // Early termination!
                    cs_free(insn, 1);
                    Logger::get().warning("Could not find: %u call at: %p within: %i rets! Found all of the rets first!", nToRetOn, (void*)ptr, retCount);
                    return (decltype(match(insn)))std::nullopt;
                }
                rCount--;
            } else {
                auto testRes = match(insn);
                if (testRes) {
                    if (nCalls == 1) {
                        cs_free(insn, 1);
                        return testRes;
                    } else {
                        nCalls--;
                    }
                } else if (skip(insn)) {
                    if (nCalls == 1) {
                        std::string name(insn->mnemonic);
                        cs_free(insn, 1);
                        Logger::get().warning("Found: %u match, at: %p within: %i rets, but the result was a %s! Cannot compute destination address!", nToRetOn, (void*)ptr, retCount, name.c_str());
                        return (decltype(match(insn)))std::nullopt;
                    } else {
                        nCalls--;
                    }
                }
            }
            // Other instructions are ignored silently
        }
        else {
            // Invalid instructions are ignored silently.
            // In order to skip these properly, we must increment our instructions, ptr, and size accordingly.
            Logger::get().warning("FAILED PARSE: %p diassemb: 0x%x", (void*)ptr, *(uint32_t*)ptr);
            sz -= 4;
            ptr += 4;
            instructions += 4;
        }
    }
    // If we run out of bytes to parse, we fail
    cs_free(insn, 1);
    return (decltype(match(insn)))std::nullopt;
}

std::optional<uint32_t*> blConv(cs_insn* insn);

template<uint32_t nToRetOn, bool includeR = false, int retCount = -1, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNthBl(const uint32_t* addr) {
    return find_through_hooks(addr, szBytes, [](auto... pairs) {
        std::array addrs{pairs...};
        if constexpr (includeR) {
            return findNth(addrs, nToRetOn, retCount, &blConv, &insnMatch<ARM64_INS_BLR>);
        } else {
            return findNth(addrs, nToRetOn, retCount, &blConv, &insnMatch<>);
        }
    });
}

std::optional<uint32_t*> bConv(cs_insn* insn);

template<uint32_t nToRetOn, bool includeR = false, int retCount = -1, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNthB(const uint32_t* addr) {
    return find_through_hooks(addr, szBytes, [](auto... pairs) {
        std::array addrs{pairs...};
        if constexpr (includeR) {
            return findNth(addrs, nToRetOn, retCount, &bConv, &insnMatch<ARM64_INS_BR>);
        } else {
            return findNth(addrs, nToRetOn, retCount, &bConv, &insnMatch<>);
        }
    });
}

std::optional<std::tuple<uint32_t*, arm64_reg, uint32_t*>> pcRelConv(cs_insn* insn);

template<uint32_t nToRetOn, int retCount = -1, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNthPcRel(const uint32_t* addr) {
    return find_through_hooks(addr, szBytes, [](auto... pairs) {
        std::array addrs{pairs...};
        return findNth(addrs, nToRetOn, retCount, &pcRelConv, &insnMatch<>);
    });
}

std::optional<std::tuple<uint32_t*, arm64_reg, int64_t>> regMatchConv(cs_insn* match, arm64_reg toMatch);

template<uint32_t nToRetOn, int retCount = -1, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && (szBytes % 4) == 0))
auto findNthReg(const uint32_t* addr, arm64_reg reg) {
    auto lmd = [reg](cs_insn* in) -> std::optional<std::tuple<uint32_t*, arm64_reg, int64_t>> { return regMatchConv(in, reg); };
    return find_through_hooks(addr, szBytes, [lmd = std::move(lmd)](auto... pairs) {
        std::array addrs{pairs...};
        return findNth(addrs, nToRetOn, retCount, lmd, &insnMatch<>);
    });
}

template<uint32_t nToRetOn, uint32_t nImmOff, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && nImmOff >= 1 && (szBytes % 4) == 0))
std::optional<std::tuple<uint32_t*, arm64_reg, uint32_t*>> getpcaddr(const uint32_t* addr) {
    auto pcrel = findNthPcRel<nToRetOn, -1, szBytes>(addr);
    // SAFE_ABORT_MSG("Could not find: %u pcrel at: %p within: %i rets, within size: %zu!", nToRetOn, addr, -1, szBytes);
    if (!pcrel) return std::nullopt;
    // addr is in first slot of tuple, reg in second, dst imm in third
    // TODO: decrease size correctly
    auto reginst = findNthReg<nImmOff, -1, szBytes>(std::get<0>(*pcrel), std::get<1>(*pcrel));
    // SAFE_ABORT_MSG("Could not find: %u reg with reg: %u at: %p within: %i rets, within size: %zu!", nImmOff, std::get<1>(*pcrel), std::get<0>(*pcrel), -1, szBytes);
    if (!reginst) return std::nullopt;
    return std::make_tuple(std::get<0>(*reginst), std::get<1>(*reginst), reinterpret_cast<uint32_t*>(reinterpret_cast<uint64_t>(std::get<2>(*pcrel)) + std::get<2>(*reginst)));
}

template<uint32_t nToRetOn, uint32_t nImmOff, int match, size_t szBytes = 4096>
requires ((nToRetOn >= 1 && nImmOff >= 1 && (szBytes % 4) == 0))
std::optional<uint32_t*> evalswitch(const uint32_t* addr) {
    // Get matching adr/adrp + offset on register
    auto res = getpcaddr<nToRetOn, nImmOff, szBytes>(addr);
    // SAFE_ABORT_MSG("Could not find: %u pcrel at: %p within: %i rets, within size: %zu!", nToRetOn, addr, -1, szBytes);
    if (!res) return std::nullopt;
    // Convert destination to the switch table address
    auto switchTable = reinterpret_cast<int32_t*>(std::get<2>(*res));
    // Index into switch table, which holds int32s, offset from start of switch table
    auto val = switchTable[match - 1];
    // Add offset to switch table and convert back to pointer type
    return reinterpret_cast<uint32_t*>(reinterpret_cast<uint64_t>(switchTable) + val);
}
}
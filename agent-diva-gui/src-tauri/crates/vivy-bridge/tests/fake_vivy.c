/*
 * fake_vivy.c — deterministic stand-in for the sealed library's ABI v1.
 * Used only to exercise vivy-bridge error paths the real library cannot
 * produce on demand (null returns, non-UTF-8 output, malformed JSON).
 * Behavior is driven by request content and internal state, never by
 * randomness. Live acceptance always means the real packed artifact.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned allocs = 0, frees = 0;
static int open_flag = 0;
static int arm_gap = 0;

static char *dupstr(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    memcpy(p, s, n);
    allocs++;
    return p;
}

static char *ok(const char *value) {
    char buf[1024];
    snprintf(buf, sizeof buf, "{\"ok\":true,\"value\":%s}", value);
    return dupstr(buf);
}

static char *fail(const char *kind, const char *msg) {
    char buf[1024];
    snprintf(buf, sizeof buf,
             "{\"ok\":false,\"error\":{\"kind\":\"%s\",\"message\":\"%s\"}}",
             kind, msg);
    return dupstr(buf);
}

static unsigned abi_of(const char *json) {
    const char *p = strstr(json, "\"abi_version\"");
    if (!p) return 0;
    p = strchr(p, ':');
    return p ? (unsigned)atoi(p + 1) : 0;
}

__attribute__((visibility("default")))
char *VivyInit(char *initJSON) {
    if (!initJSON) return fail("invalid_input", "init params required");
    if (open_flag) return fail("already_initialized", "a host is already initialized");
    open_flag = 1;
    unsigned abi = abi_of(initJSON);
    if (abi != 1u) return fail("incompatible_abi", "abi mismatch");
    return ok("{\"abi_version\":1,\"handle\":42}");
}

__attribute__((visibility("default")))
char *VivyCall(uint64_t handle, char *requestJSON) {
    (void)handle;
    if (!open_flag) return fail("closed", "unknown or closed handle");
    if (!requestJSON) return fail("invalid_input", "request required");

    if (strstr(requestJSON, "fake/null")) return NULL;
    if (strstr(requestJSON, "fake/utf8")) {
        /* malloc'd but deliberately not valid UTF-8 */
        char *p = malloc(8);
        p[0] = (char)0xff; p[1] = (char)0xfe; p[2] = 0;
        allocs++;
        return p;
    }
    if (strstr(requestJSON, "fake/malformed")) return dupstr("{oops");
    if (strstr(requestJSON, "fake/noenvelope")) return dupstr("{\"hello\":1}");
    if (strstr(requestJSON, "fake/arm-gap")) {
        arm_gap = 1;
        return ok("{\"armed\":true}");
    }
    if (strstr(requestJSON, "fake/counters")) {
        char v[160];
        snprintf(v, sizeof v, "{\"allocs\":%u,\"frees\":%u}", allocs, frees);
        return ok(v);
    }
    /* default: echo the request as the value */
    {
        size_t n = strlen(requestJSON) + 64;
        char *buf = malloc(n);
        snprintf(buf, n, "{\"ok\":true,\"value\":{\"echo\":%s}}", requestJSON);
        allocs++;
        return buf;
    }
}

__attribute__((visibility("default")))
char *VivyPollEvents(uint64_t handle, uint32_t maxEvents) {
    (void)handle; (void)maxEvents;
    if (!open_flag) return fail("closed", "unknown or closed handle");
    if (arm_gap) {
        arm_gap = 0;
        return ok("{\"events\":[{\"method\":\"fake/event\",\"params\":{\"n\":1}}],\"gap\":true}");
    }
    return ok("{\"events\":[],\"gap\":false}");
}

__attribute__((visibility("default")))
char *VivyShutdown(uint64_t handle) {
    (void)handle;
    if (!open_flag) return fail("closed", "unknown or closed handle");
    open_flag = 0;
    return ok("{\"shutdown\":true}");
}

__attribute__((visibility("default")))
void VivyFree(char *ptr) {
    if (ptr) { frees++; free(ptr); }
}

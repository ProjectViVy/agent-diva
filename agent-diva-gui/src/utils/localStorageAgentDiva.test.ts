import { describe, expect, it, beforeEach } from "vitest";
import {
  SESSION_CACHE_PREFIX,
  invalidateSessionCache,
  isSessionCacheStaleAgainstList,
  sessionCacheStorageKeys,
} from "./localStorageAgentDiva";

describe("sessionCacheStorageKeys", () => {
  it("normalizes bare chat id to gui: prefix key", () => {
    expect(sessionCacheStorageKeys("chat-1")).toEqual([
      `${SESSION_CACHE_PREFIX}gui:chat-1`,
      `${SESSION_CACHE_PREFIX}chat-1`,
    ]);
  });

  it("keeps full session key and does not duplicate", () => {
    expect(sessionCacheStorageKeys("gui:chat-1")).toEqual([
      `${SESSION_CACHE_PREFIX}gui:chat-1`,
    ]);
  });

  it("returns empty for blank input", () => {
    expect(sessionCacheStorageKeys("")).toEqual([]);
  });
});

describe("isSessionCacheStaleAgainstList", () => {
  it("is stale when list reports more visible messages than cache", () => {
    expect(isSessionCacheStaleAgainstList(2, 10)).toBe(true);
  });

  it("is not stale when counts match or cache is ahead", () => {
    expect(isSessionCacheStaleAgainstList(10, 10)).toBe(false);
    expect(isSessionCacheStaleAgainstList(12, 10)).toBe(false);
  });

  it("is not stale when list metadata is missing", () => {
    expect(isSessionCacheStaleAgainstList(2, null)).toBe(false);
    expect(isSessionCacheStaleAgainstList(2, undefined)).toBe(false);
  });
});

describe("invalidateSessionCache", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("removes both bare and normalized keys for a session", () => {
    localStorage.setItem(`${SESSION_CACHE_PREFIX}gui:chat-1`, "a");
    localStorage.setItem(`${SESSION_CACHE_PREFIX}chat-1`, "b");
    localStorage.setItem(`${SESSION_CACHE_PREFIX}gui:other`, "c");

    invalidateSessionCache("chat-1");

    expect(localStorage.getItem(`${SESSION_CACHE_PREFIX}gui:chat-1`)).toBeNull();
    expect(localStorage.getItem(`${SESSION_CACHE_PREFIX}chat-1`)).toBeNull();
    expect(localStorage.getItem(`${SESSION_CACHE_PREFIX}gui:other`)).toBe("c");
  });
});

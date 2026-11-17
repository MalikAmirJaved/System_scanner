import { describe, expect, it } from "vitest";
import {
  IpcContractError,
  at,
  boolean,
  count,
  index,
  listOf,
  object,
  oneOf,
  optional,
  optionalString,
  parsePayload,
  string,
  text,
  version,
  type Validator,
} from "./validate";

describe("path helpers", () => {
  it("builds dotted and indexed paths", () => {
    expect(at("", "cpu")).toBe("cpu");
    expect(at("cpu", "model")).toBe("cpu.model");
    expect(index("disks", 0)).toBe("disks[0]");
    expect(at(index("items", 2), "name")).toBe("items[2].name");
  });
});

describe("parsePayload", () => {
  const parseThing: Validator<{ name: string }> = (value, path) => {
    const raw = object(value, path);
    return { name: string(raw.name, at(path, "name")) };
  };

  it("returns the validated value", () => {
    expect(parsePayload("get_thing", parseThing, { name: "x" })).toEqual({ name: "x" });
  });

  it("attributes a field failure to the command and never echoes the value", () => {
    const call = () => parsePayload("get_thing", parseThing, { name: 42, secret: "C:\\Users\\me" });

    expect(call).toThrow(IpcContractError);
    expect(call).toThrow(/get_thing/);
    expect(call).toThrow(/name is not a string/);
    expect(call).not.toThrow(/Users/);
  });

  it("rethrows anything that is not a contract failure", () => {
    const boom = new Error("boom");
    const explode: Validator<never> = () => {
      throw boom;
    };

    expect(() => parsePayload("get_thing", explode, {})).toThrow(boom);
  });
});

describe("primitives", () => {
  it("object rejects arrays, null, and primitives", () => {
    expect(object({}, "p")).toEqual({});
    for (const value of [[], null, 1, "s", undefined]) {
      expect(() => object(value, "p")).toThrow(/p is not an object/);
    }
  });

  it("string and text reject wrong types and over-long text", () => {
    expect(string("ok", "p")).toBe("ok");
    expect(() => string(1, "p")).toThrow(/not a string/);
    expect(text("abc", "p", 3)).toBe("abc");
    expect(() => text("abcd", "p", 3)).toThrow(/at most 3 characters/);
  });

  it("boolean does not coerce", () => {
    expect(boolean(false, "p")).toBe(false);
    for (const value of [0, 1, "true", null]) {
      expect(() => boolean(value, "p")).toThrow(/not a boolean/);
    }
  });

  it("count rejects negatives, fractions, and non-finite numbers", () => {
    expect(count(0, "p")).toBe(0);
    expect(count(1_000, "p")).toBe(1_000);
    for (const value of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, "1", null]) {
      expect(() => count(value, "p")).toThrow(/not a (finite number|non-negative integer)/);
    }
  });

  it("optional treats absence as undefined and still rejects a present wrong type", () => {
    expect(optional(undefined, "p", string)).toBeUndefined();
    expect(optional("x", "p", string)).toBe("x");
    expect(optionalString(undefined, "p")).toBeUndefined();
    expect(() => optionalString(7, "p")).toThrow(/not a string/);
  });

  it("listOf enforces the item cap and validates each item", () => {
    expect(listOf([1, 2], "p", 2, count)).toEqual([1, 2]);
    expect(() => listOf([1, 2, 3], "p", 2, count)).toThrow(/at most 2 items/);
    expect(() => listOf([1, -1], "p", 2, count)).toThrow(/p\[1\] is not a non-negative integer/);
    expect(() => listOf({}, "p", 2, count)).toThrow(/not an array/);
  });

  it("oneOf accepts only the documented values", () => {
    const allowed = ["a", "b"] as const;
    expect(oneOf(allowed, "b", "p")).toBe("b");
    expect(() => oneOf(allowed, "c", "p")).toThrow(/p is not one of a, b/);
  });

  it("version reports the expected and actual versions", () => {
    expect(version(1, 1, "schemaVersion")).toBe(1);
    expect(() => version(1, 2, "schemaVersion")).toThrow(
      /version 2, but this build reads version 1/,
    );
    expect(() => version(1, "1", "schemaVersion")).toThrow(
      /schemaVersion is a string, but this build reads version 1/,
    );
  });
});

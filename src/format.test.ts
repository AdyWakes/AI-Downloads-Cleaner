import { describe, expect, it } from "vitest";
import { baseName, formatBytes, groupBy, plural } from "./format";

describe("format", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024 * 1024)).toBe("5.0 GB");
    expect(formatBytes(-1)).toBe("–");
  });
  it("pluralizes", () => {
    expect(plural(1, "file")).toBe("1 file");
    expect(plural(2, "file")).toBe("2 files");
  });
  it("basename handles windows and unix paths", () => {
    expect(baseName("C:\\Users\\a\\Downloads")).toBe("Downloads");
    expect(baseName("/home/a/Downloads/")).toBe("Downloads");
    expect(baseName("रसीद.pdf")).toBe("रसीद.pdf");
  });
  it("groups in config order", () => {
    const g = groupBy([{ c: "B" }, { c: "A" }, { c: "Z" }, { c: "B" }], (x) => x.c, ["A", "B"]);
    expect(g.map(([k, v]) => [k, v.length])).toEqual([["A", 1], ["B", 2], ["Z", 1]]);
  });
});

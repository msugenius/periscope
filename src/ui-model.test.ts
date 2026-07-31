import { describe, expect, it } from "vitest";
import {
  displayHotkey,
  escapeHtml,
  isModifierCode,
  normalizeShortcut,
  shortcutFromEvent,
} from "./ui-model";

describe("UI model helpers", () => {
  it("escapes every HTML-significant character", () => {
    expect(escapeHtml(`<button title="'&">`)).toBe(
      "&lt;button title=&quot;&#39;&amp;&quot;&gt;",
    );
  });

  it("formats canonical shortcuts for people", () => {
    expect(displayHotkey("Control+Super+KeyQ+Digit7")).toBe(
      "Ctrl + Win + Q + 7",
    );
  });

  it("recognizes modifier keys without treating normal keys as modifiers", () => {
    expect(isModifierCode("ControlLeft")).toBe(true);
    expect(isModifierCode("MetaRight")).toBe(true);
    expect(isModifierCode("KeyQ")).toBe(false);
  });

  it("normalizes aliases, modifier order, letters, and digits", () => {
    expect(normalizeShortcut("shift + ctrl + q")).toBe("Control+Shift+KeyQ");
    expect(normalizeShortcut("win+7")).toBe("Super+Digit7");
  });

  it("converts keyboard events into canonical shortcuts", () => {
    const event = new KeyboardEvent("keydown", {
      code: "KeyK",
      ctrlKey: true,
      altKey: true,
      shiftKey: true,
    });

    expect(shortcutFromEvent(event)).toBe("Control+Alt+Shift+KeyK");
  });
});

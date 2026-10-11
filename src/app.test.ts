import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

const idleUpdate = {
  phase: "idle",
  installedVersion: "0.1.0",
  candidate: null,
  downloadedBytes: null,
  totalBytes: null,
  failureCode: null,
  message: null,
};

const visual = {
  color: "#35E8FF",
  opacity: 100,
  length: 10,
  thickness: 1,
  gap: 3,
  centerDot: true,
  dotSize: 2,
  tStyle: false,
  dotOnly: false,
  outline: true,
  outlineThickness: 1,
  outlineColor: "#000000",
  xOffset: 0,
  yOffset: 0,
};

const defaultSettings = {
  hideWhenAds: false,
  enabled: true,
  color: "#35E8FF",
  opacity: 100,
  length: 10,
  thickness: 1,
  gap: 3,
  centerDot: true,
  dotSize: 2,
  tStyle: false,
  dotOnly: false,
  outline: true,
  outlineThickness: 1,
  outlineColor: "#000000",
  xOffset: 0,
  yOffset: 0,
  activePreset: "classic" as const,
  presets: [
    {
      id: "dot",
      name: "Dot",
      settings: { ...visual, dotSize: 3, dotOnly: true },
    },
    { id: "classic", name: "Classic", settings: { ...visual } },
    {
      id: "precision",
      name: "T-Shape",
      settings: { ...visual, length: 14, gap: 2, dotSize: 1, tStyle: true },
    },
  ],
  hotkeys: {
    toggleCrosshair: "F2",
    toggleAds: "F5",
  },
  hotkeyErrors: {},
};

async function startApp() {
  vi.resetModules();
  const { boot } = await import("./app");
  await boot();
}

function emitRecordedShortcut(shortcut: string) {
  const listener = mocks.listen.mock.calls.find(
    ([name]) => name === "hotkey-recorded",
  )?.[1];
  expect(listener).toBeDefined();
  listener({ payload: shortcut });
}

beforeEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = '<div id="app"></div>';
  mocks.invoke.mockReset();
  mocks.listen.mockReset();
  mocks.unlisten.mockReset();
  mocks.listen.mockResolvedValue(mocks.unlisten);
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === "get_settings") return structuredClone(defaultSettings);
    if (command === "save_preset_settings")
      return structuredClone(defaultSettings);
    if (command === "select_preset") return structuredClone(defaultSettings);
    if (command === "get_update_status") return structuredClone(idleUpdate);
    if (command === "start_update_check") return structuredClone(idleUpdate);
    return undefined;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("settings application", () => {
  it("shows Settings with only the supported hotkeys", async () => {
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="hotkeys"]')!.click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]'),
      ).not.toBeNull(),
    );
    expect(
      document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
    ).toBe("F5");
    expect(document.querySelector("h1")?.textContent).toBe("Settings");
    expect(document.querySelector(".panel-heading h2")?.textContent).toBe(
      "Hotkeys",
    );
    expect(document.querySelectorAll("[data-hotkey]")).toHaveLength(2);
    expect(document.querySelectorAll(".hotkey-info-trigger")).toHaveLength(2);
    expect(document.querySelectorAll(".hotkey-tooltip")).toHaveLength(2);
    expect(
      document.querySelector(".hotkey-copy > span:not(.hotkey-info)"),
    ).toBeNull();
    expect(document.body.textContent).not.toContain("Close app");
    expect(document.body.textContent).not.toContain("Show settings");
    expect(document.querySelector("#ads-status")).toBeNull();
    expect(document.querySelector("#reset-hotkeys")).toBeNull();
    expect(document.body.textContent).not.toContain("Default F2");
    expect(document.body.textContent).not.toContain("Default F5");
    expect(document.querySelector("#hotkey-status")?.textContent).toBe(
      "Changes save automatically.",
    );
  });

  it("records Alt+Tab through the native hotkey flow", async () => {
    mocks.invoke.mockImplementation(
      async (
        command: string,
        args?: { hotkeys?: typeof defaultSettings.hotkeys },
      ) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "update_hotkeys") return structuredClone(args?.hotkeys);
        return undefined;
      },
    );
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="hotkeys"]')!.click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]'),
      ).not.toBeNull(),
    );
    document
      .querySelector<HTMLButtonElement>('[data-hotkey="toggleAds"]')!
      .click();
    await vi.waitFor(() =>
      expect(
        document.querySelector(
          '[data-hotkey="toggleAds"][aria-pressed="true"]',
        ),
      ).not.toBeNull(),
    );
    emitRecordedShortcut("Alt+Tab");
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
        hotkeys: { ...defaultSettings.hotkeys, toggleAds: "Alt+Tab" },
      }),
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
      ).toBe("Alt + Tab"),
    );
  });

  it("records a focused key press and ignores the matching native event", async () => {
    mocks.invoke.mockImplementation(
      async (
        command: string,
        args?: { hotkeys?: typeof defaultSettings.hotkeys },
      ) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "update_hotkeys") return structuredClone(args?.hotkeys);
        return undefined;
      },
    );
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="hotkeys"]')!.click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]'),
      ).not.toBeNull(),
    );
    document
      .querySelector<HTMLButtonElement>('[data-hotkey="toggleAds"]')!
      .click();
    await vi.waitFor(() =>
      expect(
        document.querySelector(
          '[data-hotkey="toggleAds"][aria-pressed="true"]',
        ),
      ).not.toBeNull(),
    );

    window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyS" }));
    emitRecordedShortcut("KeyS");

    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
      ).toBe("S"),
    );
    expect(
      mocks.invoke.mock.calls.filter(
        ([command]) => command === "update_hotkeys",
      ),
    ).toHaveLength(1);
    expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
      hotkeys: { ...defaultSettings.hotkeys, toggleAds: "KeyS" },
    });
  });

  it("accepts Caps Lock as a passive shortcut", async () => {
    mocks.invoke.mockImplementation(
      async (
        command: string,
        args?: { hotkeys?: typeof defaultSettings.hotkeys },
      ) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "update_hotkeys") return structuredClone(args?.hotkeys);
        return undefined;
      },
    );
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="hotkeys"]')!.click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]'),
      ).not.toBeNull(),
    );
    document
      .querySelector<HTMLButtonElement>('[data-hotkey="toggleAds"]')!
      .click();
    await vi.waitFor(() =>
      expect(
        document.querySelector(
          '[data-hotkey="toggleAds"][aria-pressed="true"]',
        ),
      ).not.toBeNull(),
    );
    emitRecordedShortcut("CapsLock");
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
        hotkeys: { ...defaultSettings.hotkeys, toggleAds: "CapsLock" },
      }),
    );
    expect(document.querySelector("#hotkey-status")?.textContent).toBe(
      "Changes save automatically.",
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
      ).toBe("CapsLock"),
    );
  });
  it("moves the three default shapes into a dedicated Presets tab", async () => {
    await startApp();
    expect(document.querySelector(".quick-shapes-card")).toBeNull();
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector("h1")?.textContent).toBe("Presets"),
    );
    expect(
      document.querySelectorAll(".preset-card:not(.add-preset)"),
    ).toHaveLength(3);
    expect(
      document.querySelector('[data-preset="precision"]')?.textContent,
    ).toContain("T-Shape");
    expect(document.querySelector("#import-preset")).not.toBeNull();
  });

  it("previews Cross, T-Shape, and Dot as an unsaved style change", async () => {
    await startApp();
    expect(
      document
        .querySelector('[data-shape-style="cross"]')
        ?.getAttribute("aria-pressed"),
    ).toBe("true");

    document
      .querySelector<HTMLButtonElement>('[data-shape-style="t"]')!
      .click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("preview_settings", {
        settings: expect.objectContaining({ tStyle: true, dotOnly: false }),
      }),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );

    document
      .querySelector<HTMLButtonElement>('[data-shape-style="dot"]')!
      .click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("preview_settings", {
        settings: expect.objectContaining({ dotOnly: true, centerDot: true }),
      }),
    );
    expect(document.querySelector('[data-key="length"]')).toBeNull();
    expect(document.querySelector('[data-key="dotSize"]')).not.toBeNull();

    document.querySelector<HTMLButtonElement>("#cancel")!.click();
    await vi.waitFor(() =>
      expect(
        document
          .querySelector('[data-shape-style="cross"]')
          ?.getAttribute("aria-pressed"),
      ).toBe("true"),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      true,
    );
  });

  it("renders settings before a delayed update snapshot completes", async () => {
    const updateCalls: string[] = [];
    let resolveStatus: ((value: typeof idleUpdate) => void) | undefined;
    mocks.listen.mockImplementation(async () => {
      updateCalls.push("listen");
      return mocks.unlisten;
    });
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "get_update_status") {
        updateCalls.push("get_update_status");
        return new Promise<typeof idleUpdate>((resolve) => {
          resolveStatus = resolve;
        });
      }
      if (command === "start_update_check") return idleUpdate;
      return undefined;
    });

    await startApp();

    expect(document.querySelector("h1")?.textContent).toBe("Crosshair");
    expect(updateCalls).toEqual([
      "listen",
      "listen",
      "listen",
      "listen",
      "listen",
      "get_update_status",
    ]);
    expect(mocks.invoke).not.toHaveBeenCalledWith("start_update_check");

    resolveStatus?.(idleUpdate);
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("start_update_check"),
    );
  });

  it("loads settings, renders the editor, and navigates to Settings", async () => {
    await startApp();

    expect(document.querySelector("h1")?.textContent).toBe("Crosshair");
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(document.querySelector("h1")?.textContent).toBe("Settings"),
    );
    expect(document.querySelector('[data-page="hotkeys"]')?.textContent).toBe(
      "Settings",
    );
    expect(document.querySelector(".panel-heading h2")?.textContent).toBe(
      "Hotkeys",
    );
    expect(document.body.textContent).toContain("Toggle crosshair");
    expect(document.body.textContent).toContain("F2");
    expect(document.body.textContent).toContain("F5");
  });

  it("suppresses modifier and repeated keys while recording", async () => {
    await startApp();
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(document.querySelector("[data-hotkey]")).toBeTruthy(),
    );
    (
      document.querySelector('[data-hotkey="toggleAds"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_hotkey_recording", {
        recording: true,
      }),
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey][aria-pressed="true"]'),
      ).toBeTruthy(),
    );

    window.dispatchEvent(
      new KeyboardEvent("keydown", { code: "ControlLeft", ctrlKey: true }),
    );
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        code: "KeyQ",
        ctrlKey: true,
        repeat: true,
      }),
    );

    expect(mocks.invoke).not.toHaveBeenCalledWith(
      "update_hotkeys",
      expect.anything(),
    );
  });

  it("clears an individual shortcut and renders it as unassigned", async () => {
    mocks.invoke.mockImplementation(
      async (
        command: string,
        payload?: { hotkeys?: typeof defaultSettings.hotkeys },
      ) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "update_hotkeys")
          return structuredClone(payload?.hotkeys);
        return undefined;
      },
    );
    await startApp();
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-clear-hotkey="toggleAds"]'),
      ).toBeTruthy(),
    );

    (
      document.querySelector(
        '[data-clear-hotkey="toggleAds"]',
      ) as HTMLButtonElement
    ).click();

    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
        hotkeys: { ...defaultSettings.hotkeys, toggleAds: "" },
      }),
    );
    expect(
      document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
    ).toBe("Not set");
    expect(
      (
        document.querySelector(
          '[data-clear-hotkey="toggleAds"]',
        ) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(document.querySelector("#hotkey-status")?.textContent).toBe(
      "Changes save automatically.",
    );
  });

  it("shows native hotkey errors without losing the active binding", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "update_hotkeys") throw new Error("shortcut unavailable");
      return undefined;
    });
    await startApp();
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(document.querySelector("[data-hotkey]")).toBeTruthy(),
    );
    (document.querySelector("[data-hotkey]") as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_hotkey_recording", {
        recording: true,
      }),
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey][aria-pressed="true"]'),
      ).toBeTruthy(),
    );

    emitRecordedShortcut("Control+KeyQ");

    await vi.waitFor(() =>
      expect(document.querySelector("#hotkey-feedback")?.textContent).toContain(
        "shortcut unavailable",
      ),
    );
    expect(document.querySelector("#hotkey-status")?.textContent).toBe(
      "Changes save automatically.",
    );
    emitRecordedShortcut("Escape");
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleCrosshair"]')?.textContent,
      ).toBe("F2"),
    );
  });

  it("accepts a recorded shortcut and restores native dispatch on key release", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "update_hotkeys") {
        return {
          toggleCrosshair: "Control+KeyQ",
          toggleAds: "F5",
        };
      }
      return undefined;
    });
    await startApp();
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(document.querySelector("[data-hotkey]")).toBeTruthy(),
    );
    (
      document.querySelector(
        '[data-hotkey="toggleCrosshair"]',
      ) as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey][aria-pressed="true"]'),
      ).toBeTruthy(),
    );

    emitRecordedShortcut("Control+KeyQ");
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey="toggleCrosshair"]')?.textContent,
      ).toBe("Ctrl + Q"),
    );
    expect(document.querySelector("#hotkey-status")?.textContent).toBe(
      "Changes save automatically.",
    );
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_hotkey_recording", {
        recording: false,
      }),
    );
  });

  it("selects a preset and hides settings from the titlebar", async () => {
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-preset="dot"]')).not.toBeNull(),
    );
    document.querySelector<HTMLButtonElement>('[data-preset="dot"]')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("select_preset", {
        preset: "dot",
      }),
    );
    document.querySelector<HTMLButtonElement>("#minimize")!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("hide_settings"),
    );
  });

  it("restores every saved preset and moves the active highlight", async () => {
    const selectedPresets: string[] = [];
    const presets = structuredClone(defaultSettings.presets);
    mocks.invoke.mockImplementation(
      async (command: string, payload?: { preset?: string }) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "select_preset" && payload?.preset) {
          selectedPresets.push(payload.preset);
          const chosen = presets.find(
            (preset) => preset.id === payload.preset,
          )!;
          return {
            activePreset: payload.preset,
            presets,
            enabled: true,
            ...chosen.settings,
          };
        }
        return undefined;
      },
    );
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-preset="dot"]')).not.toBeNull(),
    );

    expect(
      document.querySelector('[data-preset="precision"]')?.textContent,
    ).toContain("T-Shape");
    expect(document.querySelectorAll("[data-preset]")).toHaveLength(3);
    expect(document.body.textContent).not.toContain("Compact");
    expect(document.body.textContent).not.toContain("Open");

    for (const { id } of presets) {
      (
        document.querySelector(`[data-preset="${id}"]`) as HTMLButtonElement
      ).click();
      await vi.waitFor(() => {
        if (id !== "classic") expect(selectedPresets.at(-1)).toBe(id);
        expect(
          document
            .querySelector(`[data-preset="${id}"]`)
            ?.getAttribute("aria-pressed"),
        ).toBe("true");
        expect(
          document
            .querySelector(`[data-preset="${id}"]`)
            ?.closest(".preset-card")
            ?.classList.contains("active"),
        ).toBe(true);
      });
    }
  });

  it("restores the saved active preset on startup", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") {
        const presets = structuredClone(defaultSettings.presets);
        presets[0].settings.dotSize = 7;
        return {
          ...structuredClone(defaultSettings),
          activePreset: "dot",
          dotSize: 7,
          presets,
        };
      }
      return undefined;
    });

    await startApp();
    expect((document.querySelector("#dotSize") as HTMLInputElement).value).toBe(
      "7",
    );
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-preset="dot"]')).not.toBeNull(),
    );
    const dot = document.querySelector('[data-preset="dot"]');
    expect(dot?.closest(".preset-card")?.classList).toContain("active");
    expect(dot?.getAttribute("aria-pressed")).toBe("true");
    expect(
      document.querySelector('[data-preset="classic"]')?.closest(".preset-card")
        ?.classList,
    ).not.toContain("active");
  });

  it("keeps a draft after save failure and clears it after recovery", async () => {
    let updateAttempts = 0;
    mocks.invoke.mockImplementation(
      async (command: string, args?: { settings?: typeof visual }) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "save_preset_settings") {
          updateAttempts += 1;
          if (updateAttempts === 1) throw new Error("disk full");
          return {
            ...defaultSettings,
            ...args?.settings,
            presets: defaultSettings.presets.map((preset) =>
              preset.id === "classic"
                ? { ...preset, settings: args!.settings! }
                : preset,
            ),
          };
        }
        return undefined;
      },
    );
    await startApp();
    const length = document.querySelector<HTMLInputElement>(
      '.number[data-key="length"]',
    )!;
    length.value = "20";
    length.dispatchEvent(new Event("change", { bubbles: true }));
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );
    document.querySelector<HTMLButtonElement>("#save")!.click();
    await vi.waitFor(() =>
      expect(document.querySelector(".save-state")?.textContent).toContain(
        "Could not save",
      ),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );
    document.querySelector<HTMLButtonElement>("#save")!.click();
    await vi.waitFor(() =>
      expect(document.querySelector(".save-state")?.textContent).toContain(
        "Preset saved",
      ),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      true,
    );
  });

  it("previews edits without persisting, then cancels or saves them", async () => {
    mocks.invoke.mockImplementation(
      async (command: string, args?: { settings?: typeof visual }) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "save_preset_settings") {
          return {
            ...defaultSettings,
            ...args!.settings,
            presets: defaultSettings.presets.map((preset) =>
              preset.id === "classic"
                ? { ...preset, settings: args!.settings! }
                : preset,
            ),
          };
        }
        return undefined;
      },
    );
    await startApp();
    const changeLength = (value: string) => {
      const input = document.querySelector<HTMLInputElement>(
        '.number[data-key="length"]',
      )!;
      input.value = value;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    };
    changeLength("21");
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("preview_settings", {
        settings: expect.objectContaining({ length: 21 }),
      }),
    );
    expect(mocks.invoke).not.toHaveBeenCalledWith(
      "save_preset_settings",
      expect.anything(),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );
    document.querySelector<HTMLButtonElement>("#cancel")!.click();
    await vi.waitFor(() =>
      expect(
        (document.querySelector("#length") as HTMLInputElement).value,
      ).toBe("10"),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("cancel_preview");
    changeLength("24");
    document.querySelector<HTMLButtonElement>("#save")!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("save_preset_settings", {
        preset: "classic",
        settings: expect.objectContaining({ length: 24 }),
      }),
    );
    await vi.waitFor(() =>
      expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
        true,
      ),
    );
  });

  it("keeps edits made while a save is in flight as a new draft", async () => {
    let finishSave: ((value: unknown) => void) | undefined;
    mocks.invoke.mockImplementation(
      async (command: string, args?: { settings?: typeof visual }) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "save_preset_settings") {
          return new Promise((resolve) => {
            finishSave = resolve;
          });
        }
        if (command === "preview_settings") return args?.settings;
        return undefined;
      },
    );
    await startApp();
    const changeLength = (value: string) => {
      const input = document.querySelector<HTMLInputElement>(
        '.number[data-key="length"]',
      )!;
      input.value = value;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    };
    changeLength("21");
    document.querySelector<HTMLButtonElement>("#save")!.click();
    await vi.waitFor(() => expect(finishSave).toBeDefined());
    changeLength("24");
    finishSave!({
      ...defaultSettings,
      length: 21,
      presets: defaultSettings.presets.map((preset) =>
        preset.id === "classic"
          ? { ...preset, settings: { ...visual, length: 21 } }
          : preset,
      ),
    });
    await vi.waitFor(() =>
      expect(
        (document.querySelector("#length") as HTMLInputElement).value,
      ).toBe("24"),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );
  });

  it("creates an unnamed preset and renames it", async () => {
    const created = {
      id: "preset-new",
      name: "Lunar Ray",
      settings: { ...visual },
    };
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "create_preset")
        return {
          ...defaultSettings,
          activePreset: created.id,
          presets: [...defaultSettings.presets, created],
        };
      if (command === "rename_preset")
        return {
          ...defaultSettings,
          activePreset: created.id,
          presets: [
            ...defaultSettings.presets,
            { ...created, name: "My crosshair" },
          ],
        };
      return undefined;
    });
    await startApp();
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector("#create-preset")).not.toBeNull(),
    );
    document.querySelector<HTMLButtonElement>("#create-preset")!.click();
    document
      .querySelector<HTMLFormElement>(".app-dialog")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("create_preset", {
        name: null,
      }),
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-rename-preset="preset-new"]'),
      ).not.toBeNull(),
    );
    document
      .querySelector<HTMLButtonElement>('[data-rename-preset="preset-new"]')!
      .click();
    document.querySelector<HTMLInputElement>(
      '.app-dialog input[name="value"]',
    )!.value = "My crosshair";
    document
      .querySelector<HTMLFormElement>(".app-dialog")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("rename_preset", {
        preset: "preset-new",
        name: "My crosshair",
      }),
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-preset="preset-new"]')?.textContent,
      ).toContain("My crosshair"),
    );
  });

  it("keeps unsaved edits when switching presets is cancelled", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "select_preset")
        return {
          ...defaultSettings,
          ...defaultSettings.presets[0].settings,
          activePreset: "dot",
        };
      return undefined;
    });
    await startApp();
    const length = document.querySelector<HTMLInputElement>(
      '.number[data-key="length"]',
    )!;
    length.value = "25";
    length.dispatchEvent(new Event("change", { bubbles: true }));
    document.querySelector<HTMLButtonElement>('[data-page="presets"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-preset="dot"]')).not.toBeNull(),
    );
    expect(
      document.querySelector<HTMLButtonElement>('[data-copy-preset="classic"]')
        ?.disabled,
    ).toBe(true);
    document.querySelector<HTMLButtonElement>('[data-preset="dot"]')!.click();
    await vi.waitFor(() =>
      expect(document.querySelector(".app-dialog")).not.toBeNull(),
    );
    document.querySelector<HTMLButtonElement>('[data-choice="stay"]')!.click();
    expect(mocks.invoke).not.toHaveBeenCalledWith(
      "select_preset",
      expect.anything(),
    );
    expect(document.querySelector<HTMLButtonElement>("#save")?.disabled).toBe(
      false,
    );
  });

  it("falls back to defaults when native settings cannot be loaded", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    mocks.invoke.mockRejectedValueOnce(new Error("native bridge unavailable"));

    await startApp();

    expect(document.querySelector("h1")?.textContent).toBe("Crosshair");
    expect((document.querySelector("#length") as HTMLInputElement).value).toBe(
      "10",
    );
    expect(consoleError).toHaveBeenCalled();
  });
});

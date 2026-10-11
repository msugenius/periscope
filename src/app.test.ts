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
  outline: true,
  outlineThickness: 1,
  outlineColor: "#000000",
  xOffset: 0,
  yOffset: 0,
  activePreset: "classic" as const,
  hotkeys: {
    toggleCrosshair: "F2",
    closeApp: "F3",
    showSettings: "F4",
    toggleAds: "F5",
  },
  hotkeyErrors: {},
};

async function startApp() {
  vi.resetModules();
  const { boot } = await import("./app");
  await boot();
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
    if (command === "update_settings") return structuredClone(defaultSettings);
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
  it("shows the ADS hotkey and updates its active state from native events", async () => {
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
    expect(document.querySelector("#ads-status")?.textContent).toContain("off");
    await vi.waitFor(() =>
      expect(mocks.listen).toHaveBeenCalledWith(
        "hide-when-ads-changed",
        expect.any(Function),
      ),
    );
    const listener = mocks.listen.mock.calls.find(
      ([name]) => name === "hide-when-ads-changed",
    )![1];
    listener({ payload: true });
    expect(document.querySelector("#ads-status")?.textContent).toContain("on");
  });

  it("records an ADS shortcut through the normal hotkey flow", async () => {
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
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "F6" }));
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
        hotkeys: { ...defaultSettings.hotkeys, toggleAds: "F6" },
      }),
    );
    expect(
      document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
    ).toBe("F6");
  });

  it("rejects Caps Lock while recording without replacing the active binding", async () => {
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
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "CapsLock" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith(
      "update_hotkeys",
      expect.anything(),
    );
    expect(document.querySelector("#hotkey-status")?.textContent).toContain(
      "Caps Lock",
    );
    expect(
      document.querySelector('[data-hotkey="toggleAds"]')?.textContent,
    ).toBe("Press shortcut...");
  });
  it("uses a muted master switch when the overlay is disabled", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") {
        return { ...structuredClone(defaultSettings), enabled: false };
      }
      return undefined;
    });

    await startApp();

    const masterSwitch = document.querySelector(".master-switch");
    expect(masterSwitch?.classList).toContain("is-disabled");
    expect(masterSwitch?.textContent).toContain("Disabled");
  });

  it("renders quick shapes as a standalone panel without a live preview", async () => {
    await startApp();

    const panel = document.querySelector(".quick-shapes-card");
    expect(panel?.querySelector("h2")?.textContent).toBe("Quick shapes");
    expect(panel?.querySelector("p")?.textContent).toBe(
      "Choose a base, then fine-tune",
    );
    expect(document.querySelector("#preview")).toBeNull();
    expect(document.body.textContent).not.toContain("Live preview");
    expect(document.querySelector(".controls-column")).toBeNull();
    expect(document.querySelector(".placement-card")).toBeNull();
    expect(document.body.textContent).not.toContain("Placement");
    expect(
      document.querySelector(".shape-card .panel-heading p")?.textContent,
    ).toBe("Relative to preset");
    expect(
      document.querySelector(".visibility-card .panel-heading p")?.textContent,
    ).toBe("Shared across all presets");
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
      "get_update_status",
    ]);
    expect(mocks.invoke).not.toHaveBeenCalledWith("start_update_check");

    resolveStatus?.(idleUpdate);
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("start_update_check"),
    );
  });

  it("loads settings, renders the editor, and navigates to hotkeys", async () => {
    await startApp();

    expect(document.querySelector("h1")?.textContent).toBe("Crosshair");
    (
      document.querySelector('[data-page="hotkeys"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(document.querySelector("h1")?.textContent).toBe("Hotkeys"),
    );
    expect(document.body.textContent).toContain("Toggle crosshair");
    expect(document.body.textContent).toContain("F2");
    expect(document.body.textContent).toContain("F3");
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
      document.querySelector('[data-hotkey="closeApp"]') as HTMLButtonElement
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
        document.querySelector('[data-clear-hotkey="closeApp"]'),
      ).toBeTruthy(),
    );

    (
      document.querySelector(
        '[data-clear-hotkey="closeApp"]',
      ) as HTMLButtonElement
    ).click();

    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_hotkeys", {
        hotkeys: { ...defaultSettings.hotkeys, closeApp: "" },
      }),
    );
    expect(
      document.querySelector('[data-hotkey="closeApp"]')?.textContent,
    ).toBe("Not set");
    expect(
      (
        document.querySelector(
          '[data-clear-hotkey="closeApp"]',
        ) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
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

    window.dispatchEvent(
      new KeyboardEvent("keydown", { code: "KeyQ", ctrlKey: true }),
    );

    await vi.waitFor(() =>
      expect(document.querySelector("#hotkey-status")?.textContent).toContain(
        "shortcut unavailable",
      ),
    );
    expect(document.body.textContent).toContain("F3");
  });

  it("accepts a recorded shortcut and restores native dispatch on key release", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "update_hotkeys") {
        return {
          toggleCrosshair: "F2",
          closeApp: "Control+KeyQ",
          showSettings: "F4",
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
      document.querySelector('[data-hotkey="closeApp"]') as HTMLButtonElement
    ).click();
    await vi.waitFor(() =>
      expect(
        document.querySelector('[data-hotkey][aria-pressed="true"]'),
      ).toBeTruthy(),
    );

    window.dispatchEvent(
      new KeyboardEvent("keydown", { code: "KeyQ", ctrlKey: true }),
    );
    await vi.waitFor(() =>
      expect(document.querySelector("#hotkey-status")?.textContent).toContain(
        "saved as Ctrl + Q",
      ),
    );
    window.dispatchEvent(new KeyboardEvent("keyup", { code: "KeyQ" }));

    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_hotkey_recording", {
        recording: false,
      }),
    );
  });

  it("wires window controls, persistent presets, and reset", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "reset_settings") {
        const {
          hotkeys: _hotkeys,
          hotkeyErrors: _errors,
          ...crosshair
        } = defaultSettings;
        return crosshair;
      }
      if (command === "update_settings")
        return structuredClone(defaultSettings);
      if (command === "select_preset") return structuredClone(defaultSettings);
      return undefined;
    });
    await startApp();

    (document.querySelector("#minimize") as HTMLButtonElement).click();
    (
      document.querySelector('[data-preset="dot"]') as HTMLButtonElement
    ).click();
    (document.querySelector("#reset") as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("hide_settings"),
    );
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("reset_settings"),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("select_preset", {
      preset: "dot",
    });
  });

  it("restores every saved preset and moves the active highlight", async () => {
    const selectedPresets: string[] = [];
    const presets = {
      classic: { length: 10, thickness: 1, gap: 3, dotSize: 2 },
      dot: { length: 1, thickness: 1, gap: 0, dotSize: 5 },
      precision: {
        length: 18,
        thickness: 1,
        gap: 2,
        dotSize: 1,
        tStyle: true,
      },
    };
    mocks.invoke.mockImplementation(
      async (command: string, payload?: { preset?: keyof typeof presets }) => {
        if (command === "get_settings") return structuredClone(defaultSettings);
        if (command === "select_preset" && payload?.preset) {
          selectedPresets.push(payload.preset);
          return {
            ...structuredClone(defaultSettings),
            ...presets[payload.preset],
          };
        }
        return undefined;
      },
    );
    await startApp();

    expect(
      document.querySelector('[data-preset="precision"]')?.textContent,
    ).toContain("T-Shape");
    expect(document.querySelectorAll("[data-preset]")).toHaveLength(3);
    expect(document.body.textContent).not.toContain("Compact");
    expect(document.body.textContent).not.toContain("Open");

    for (const [name, expected] of Object.entries(presets)) {
      (
        document.querySelector(`[data-preset="${name}"]`) as HTMLButtonElement
      ).click();
      await vi.waitFor(() => {
        expect(selectedPresets.at(-1)).toBe(name);
        expect(
          document
            .querySelector(`[data-preset="${name}"]`)
            ?.classList.contains("active"),
        ).toBe(true);
        expect(
          (document.querySelector("#length") as HTMLInputElement).value,
        ).toBe(String(expected.length));
      });
    }
  });

  it("restores the saved active preset on startup", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") {
        return {
          ...structuredClone(defaultSettings),
          activePreset: "dot",
          dotSize: 7,
        };
      }
      return undefined;
    });

    await startApp();

    const dot = document.querySelector('[data-preset="dot"]');
    expect(dot?.classList).toContain("active");
    expect(dot?.getAttribute("aria-pressed")).toBe("true");
    expect(
      document.querySelector('[data-preset="classic"]')?.classList,
    ).not.toContain("active");
    expect((document.querySelector("#dotSize") as HTMLInputElement).value).toBe(
      "7",
    );
  });

  it("reports a settings save failure and clears it after recovery", async () => {
    let updateAttempts = 0;
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return structuredClone(defaultSettings);
      if (command === "update_settings") {
        updateAttempts += 1;
        if (updateAttempts === 1) throw new Error("disk full");
        return { ...defaultSettings, enabled: false };
      }
      return undefined;
    });
    await startApp();

    const enabled = document.querySelector("#enabled") as HTMLInputElement;
    enabled.checked = false;
    enabled.dispatchEvent(new Event("change", { bubbles: true }));
    await vi.waitFor(() =>
      expect(document.querySelector(".save-state")?.textContent).toContain(
        "Could not save",
      ),
    );

    const recovered = document.querySelector("#enabled") as HTMLInputElement;
    recovered.checked = false;
    recovered.dispatchEvent(new Event("change", { bubbles: true }));
    await vi.waitFor(() =>
      expect(document.querySelector(".save-state")?.textContent).toContain(
        "Changes save automatically",
      ),
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

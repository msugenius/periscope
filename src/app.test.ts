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
  enabled: true,
  color: "#35E8FF",
  opacity: 100,
  length: 20,
  thickness: 2,
  gap: 6,
  centerDot: true,
  dotSize: 3,
  tStyle: false,
  outline: true,
  outlineThickness: 1,
  outlineColor: "#000000",
  xOffset: 0,
  yOffset: 0,
  hotkeys: { closeApp: "F3", showSettings: "F4" },
  hotkeyErrors: {},
};

const canvasContext = {
  beginPath: vi.fn(),
  clearRect: vi.fn(),
  fillRect: vi.fn(),
  lineTo: vi.fn(),
  moveTo: vi.fn(),
  setTransform: vi.fn(),
  stroke: vi.fn(),
  fillStyle: "",
  globalAlpha: 1,
  lineCap: "butt",
  lineWidth: 1,
  strokeStyle: "",
};

async function startApp() {
  vi.resetModules();
  const { boot } = await import("./app");
  await boot();
}

beforeEach(() => {
  document.body.innerHTML = '<div id="app"></div>';
  mocks.invoke.mockReset();
  mocks.listen.mockReset();
  mocks.unlisten.mockReset();
  mocks.listen.mockResolvedValue(mocks.unlisten);
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === "get_settings") return structuredClone(defaultSettings);
    if (command === "update_settings") return structuredClone(defaultSettings);
    if (command === "get_update_status") return structuredClone(idleUpdate);
    if (command === "start_update_check") return structuredClone(idleUpdate);
    return undefined;
  });
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    canvasContext as unknown as CanvasRenderingContext2D,
  );
  vi.spyOn(
    HTMLCanvasElement.prototype,
    "getBoundingClientRect",
  ).mockReturnValue({
    width: 720,
    height: 440,
    top: 0,
    right: 720,
    bottom: 440,
    left: 0,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("settings application", () => {
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
    expect(updateCalls).toEqual(["listen", "get_update_status"]);
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
        return { closeApp: "Control+KeyQ", showSettings: "F4" };
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
    (document.querySelector("[data-hotkey]") as HTMLButtonElement).click();
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

  it("wires window controls, presets, reset, and position reset", async () => {
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
      return undefined;
    });
    await startApp();

    (document.querySelector("#minimize") as HTMLButtonElement).click();
    (
      document.querySelector('[data-preset="compact"]') as HTMLButtonElement
    ).click();
    (document.querySelector("#center-position") as HTMLButtonElement).click();
    (document.querySelector("#reset") as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("minimize_settings"),
    );
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("reset_settings"),
    );
    expect(mocks.invoke).toHaveBeenCalledWith(
      "update_settings",
      expect.anything(),
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
      "20",
    );
    expect(consoleError).toHaveBeenCalled();
  });
});

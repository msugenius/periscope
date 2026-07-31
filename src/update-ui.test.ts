import { describe, expect, it, vi } from "vitest";
import {
  connectUpdater,
  renderUpdateSnapshot,
  type UpdateSnapshot,
} from "./update-ui";

const base: UpdateSnapshot = {
  phase: "idle",
  installedVersion: "0.1.0",
  candidate: null,
  downloadedBytes: null,
  totalBytes: null,
  failureCode: null,
  message: null,
};

function host() {
  const element = document.createElement("section");
  document.body.replaceChildren(element);
  return element;
}

describe("update status rendering", () => {
  it("renders an offered version and treats release notes as text", () => {
    const element = host();
    renderUpdateSnapshot(element, {
      ...base,
      phase: "available",
      candidate: {
        version: "0.2.0",
        notes: '<img src=x onerror="alert(1)">',
        sourceCommit: "0123456789abcdef0123456789abcdef01234567",
        platform: "windows-x86_64",
      },
    });

    expect(element.textContent).toContain("0.2.0");
    expect(element.textContent).toContain("<img src=x");
    expect(element.querySelector("img")).toBeNull();
    expect(
      element.querySelector('[data-update-action="install"]'),
    ).toBeTruthy();
  });

  it("renders up-to-date and safe failure states without an offer", () => {
    const element = host();
    renderUpdateSnapshot(element, { ...base, phase: "up-to-date" });
    expect(element.textContent).toContain("up to date");

    renderUpdateSnapshot(element, {
      ...base,
      phase: "failed",
      failureCode: "offline",
      message: "Could not check for updates. Try again next time.",
    });
    expect(element.getAttribute("role")).toBe("status");
    expect(element.textContent).toContain("Try again next time");
    expect(element.querySelector('[data-update-action="install"]')).toBeNull();
  });
});

describe("update bridge", () => {
  it("subscribes before reading the snapshot and starts checking afterward", async () => {
    const calls: string[] = [];
    const listener = vi.fn();
    const unlisten = vi.fn();
    const invoke = vi.fn(async (command: string) => {
      calls.push(command);
      return base;
    });
    const listen = vi.fn(async (_event: string, callback: unknown) => {
      calls.push("listen");
      listener.mockImplementation(callback as (...args: unknown[]) => void);
      return unlisten;
    });

    const cleanup = await connectUpdater(host(), { invoke, listen });

    expect(calls).toEqual([
      "listen",
      "get_update_status",
      "start_update_check",
    ]);
    cleanup();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("recovers authoritative native state in a recreated webview", async () => {
    const available = {
      ...base,
      phase: "available" as const,
      candidate: {
        version: "0.2.0",
        notes: "Recovered offer",
        sourceCommit: "0123456789abcdef0123456789abcdef01234567",
        platform: "windows-x86_64",
      },
    };
    const invoke = vi.fn(async (command: string) =>
      command === "get_update_status" ? available : available,
    );
    const listen = vi.fn(async () => vi.fn());
    const element = host();

    await connectUpdater(element, { invoke, listen });

    expect(element.textContent).toContain("Recovered offer");
    expect(invoke).toHaveBeenCalledWith("start_update_check");
  });

  it("pins install and dismiss actions to the displayed version", async () => {
    const available: UpdateSnapshot = {
      ...base,
      phase: "available",
      candidate: {
        version: "0.2.0",
        notes: "Ready",
        sourceCommit: "0123456789abcdef0123456789abcdef01234567",
        platform: "windows-x86_64",
      },
    };
    const invoke = vi.fn(async (command: string) =>
      command === "get_update_status" ? available : available,
    );
    const element = host();
    await connectUpdater(element, {
      invoke,
      listen: vi.fn(async () => vi.fn()),
    });

    (
      element.querySelector(
        '[data-update-action="install"]',
      ) as HTMLButtonElement
    ).click();
    expect(element.querySelectorAll("button:disabled")).toHaveLength(2);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("install_update", {
        version: "0.2.0",
      }),
    );
  });

  it("dismisses the displayed candidate without downloading it", async () => {
    const available: UpdateSnapshot = {
      ...base,
      phase: "available",
      candidate: {
        version: "0.2.0",
        notes: "Ready",
        sourceCommit: "0123456789abcdef0123456789abcdef01234567",
        platform: "windows-x86_64",
      },
    };
    const dismissed: UpdateSnapshot = {
      ...base,
      phase: "dismissed",
    };
    const invoke = vi.fn(async (command: string) =>
      command === "dismiss_update" ? dismissed : available,
    );
    const element = host();
    await connectUpdater(element, {
      invoke,
      listen: vi.fn(async () => vi.fn()),
    });

    (
      element.querySelector(
        '[data-update-action="dismiss"]',
      ) as HTMLButtonElement
    ).click();

    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("dismiss_update", {
        version: "0.2.0",
      }),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "install_update",
      expect.anything(),
    );
    expect(element.textContent).toContain("dismissed until periScope restarts");
  });

  it("renders download progress, restart messaging, and actionable recovery", () => {
    const element = host();
    const candidate = {
      version: "0.2.0",
      notes: "Ready",
      sourceCommit: "0123456789abcdef0123456789abcdef01234567",
      platform: "windows-x86_64",
    };
    renderUpdateSnapshot(element, {
      ...base,
      phase: "downloading",
      candidate,
      downloadedBytes: 50,
      totalBytes: 100,
    });
    const progress = element.querySelector('[role="progressbar"]')!;
    expect(progress.getAttribute("aria-valuenow")).toBe("50");

    renderUpdateSnapshot(element, { ...base, phase: "installing", candidate });
    expect(element.textContent).toContain("restart automatically");

    renderUpdateSnapshot(element, {
      ...base,
      phase: "failed",
      failureCode: "download-failed",
      message:
        "The update could not be downloaded. Restart periScope to try again.",
    });
    expect(element.textContent).toContain("Restart periScope to try again");
  });
});

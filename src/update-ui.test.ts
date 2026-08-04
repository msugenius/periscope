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
  const element = document.createElement("button");
  element.type = "button";
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

    expect(element.textContent).toBe("Click to update");
    expect(element.title).toContain("<img src=x");
    expect(element.querySelector("img")).toBeNull();
    expect(element.dataset.updateAction).toBe("install");
    expect(element.disabled).toBe(true);
  });

  it("renders up-to-date and safe failure states without an offer", () => {
    const element = host();
    renderUpdateSnapshot(element, { ...base, phase: "up-to-date" });
    expect(element.textContent).toContain("Up to date");

    renderUpdateSnapshot(element, {
      ...base,
      phase: "failed",
      failureCode: "offline",
      message: "Could not check for updates. Try again next time.",
    });
    expect(element.getAttribute("role")).toBe("status");
    expect(element.textContent).toContain("Update unavailable");
    expect(element.title).toContain("Try again next time");
    expect(element.dataset.updateAction).toBeUndefined();
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

    expect(element.textContent).toContain("Click to update");
    expect(element.title).toContain("Recovered offer");
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

    element.click();
    expect(element.disabled).toBe(true);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("install_update", {
        version: "0.2.0",
      }),
    );
  });

  it("replaces the old banner actions with one compact install control", async () => {
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
    const invoke = vi.fn(async () => available);
    const element = host();
    await connectUpdater(element, {
      invoke,
      listen: vi.fn(async () => vi.fn()),
    });

    expect(element.dataset.updateAction).toBe("install");
    expect(element.querySelectorAll("button")).toHaveLength(0);
    expect(element.textContent).toBe("Click to update");
  });

  it("renders compact download, install, and failure states", () => {
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
    expect(element.textContent).toContain("Downloading 50%");
    expect(element.classList).toContain("update-downloading");
    expect(element.style.getPropertyValue("--update-progress")).toBe("50%");

    renderUpdateSnapshot(element, { ...base, phase: "installing", candidate });
    expect(element.textContent).toContain("Installing");
    expect(element.style.getPropertyValue("--update-progress")).toBe("");

    renderUpdateSnapshot(element, {
      ...base,
      phase: "failed",
      failureCode: "download-failed",
      message:
        "The update could not be downloaded. Restart periScope to try again.",
    });
    expect(element.textContent).toContain("Update unavailable");
    expect(element.title).toContain("Restart periScope to try again");
  });
});

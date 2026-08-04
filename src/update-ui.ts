import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "dismissed"
  | "downloading"
  | "installing"
  | "failed";

export interface ReleaseCandidate {
  version: string;
  notes: string;
  sourceCommit: string;
  platform: string;
}

export interface UpdateSnapshot {
  phase: UpdatePhase;
  installedVersion: string;
  candidate: ReleaseCandidate | null;
  downloadedBytes: number | null;
  totalBytes: number | null;
  failureCode: string | null;
  message: string | null;
}

interface UpdateEvent {
  payload: UpdateSnapshot;
}

type Unlisten = () => void;

export interface UpdateBridge {
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  listen: (
    event: string,
    callback: (event: UpdateEvent) => void,
  ) => Promise<Unlisten>;
}

const defaultBridge: UpdateBridge = {
  invoke: (command, args) =>
    args === undefined
      ? tauriInvoke<unknown>(command)
      : tauriInvoke<unknown>(command, args),
  listen: (event, callback) => tauriListen(event, callback),
};

const safeIdle: UpdateSnapshot = {
  phase: "idle",
  installedVersion: "",
  candidate: null,
  downloadedBytes: null,
  totalBytes: null,
  failureCode: null,
  message: null,
};

let currentSnapshot = safeIdle;
let currentAction:
  ((action: "install" | "dismiss", version: string) => void) | undefined;

function text(tag: string, value: string, className?: string) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  element.textContent = value;
  return element;
}

export function renderUpdateSnapshot(
  host: HTMLElement,
  snapshot: UpdateSnapshot,
  onAction?: (action: "install" | "dismiss", version: string) => void,
): void {
  currentSnapshot = snapshot;
  host.replaceChildren();
  host.className = `status update-${snapshot.phase}`;
  host.setAttribute("aria-live", "polite");
  host.removeAttribute("aria-label");
  host.removeAttribute("data-update-action");
  host.removeAttribute("role");
  host.removeAttribute("title");

  const button = host instanceof HTMLButtonElement ? host : undefined;
  if (button) {
    button.disabled = true;
    button.onclick = null;
  }

  const renderBadge = (label: string) => {
    host.hidden = false;
    host.setAttribute("role", "status");
    host.append(document.createElement("i"), text("span", label));
  };

  if (snapshot.phase === "idle") {
    renderBadge("Checking…");
    return;
  }

  if (snapshot.phase === "checking") {
    renderBadge("Checking…");
    return;
  }
  if (snapshot.phase === "up-to-date") {
    renderBadge("Up to date");
    return;
  }
  if (snapshot.phase === "dismissed") {
    renderBadge("Update later");
    return;
  }
  if (snapshot.phase === "failed") {
    renderBadge("Update unavailable");
    host.title =
      snapshot.message ?? "Could not check for updates. Try again next time.";
    return;
  }

  const candidate = snapshot.candidate;
  if (!candidate) {
    renderBadge("Update unavailable");
    return;
  }

  if (snapshot.phase === "available") {
    renderBadge("Click to update");
    host.removeAttribute("role");
    host.dataset.updateAction = "install";
    host.title = `periScope ${candidate.version}: ${candidate.notes}`;
    host.setAttribute(
      "aria-label",
      `Update periScope to version ${candidate.version}`,
    );
    if (button) {
      button.disabled = false;
      button.onclick = () => {
        button.disabled = true;
        onAction?.("install", candidate.version);
      };
    }
    return;
  }

  if (snapshot.phase === "downloading") {
    const downloaded = snapshot.downloadedBytes ?? 0;
    const total = snapshot.totalBytes;
    if (total && total > 0) {
      const percent = Math.min(100, Math.round((downloaded / total) * 100));
      renderBadge(`Downloading ${percent}%`);
    } else {
      renderBadge("Downloading…");
    }
    return;
  }

  if (snapshot.phase === "installing") {
    renderBadge("Installing…");
  }
}

export function renderCurrentUpdate(host: HTMLElement): void {
  renderUpdateSnapshot(host, currentSnapshot, currentAction);
}

export async function connectUpdater(
  host: HTMLElement,
  bridge: UpdateBridge = defaultBridge,
): Promise<Unlisten> {
  const handleAction = (action: "install" | "dismiss", version: string) => {
    const command = action === "install" ? "install_update" : "dismiss_update";
    void bridge
      .invoke(command, { version })
      .then((result) => render(result as UpdateSnapshot))
      .catch(() =>
        render({
          ...safeIdle,
          phase: "failed",
          failureCode: "action-failed",
          message:
            "The update action could not be completed. Restart periScope to try again.",
        }),
      );
  };
  const render = (snapshot: UpdateSnapshot) =>
    renderUpdateSnapshot(host, snapshot, handleAction);
  currentAction = handleAction;
  try {
    const unlisten = await bridge.listen("periscope://updater-state", (event) =>
      render(event.payload),
    );
    const snapshot = (await bridge.invoke(
      "get_update_status",
    )) as UpdateSnapshot;
    render(snapshot);
    await bridge.invoke("start_update_check");
    return () => {
      unlisten();
      if (currentAction === handleAction) currentAction = undefined;
    };
  } catch {
    render({
      ...safeIdle,
      phase: "failed",
      message: "Could not check for updates. Try again next time.",
      failureCode: "bridge-unavailable",
    });
  }
  return () => undefined;
}

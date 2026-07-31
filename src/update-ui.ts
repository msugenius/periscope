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

function actionButton(action: string, label: string) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `button ${action === "install" ? "primary" : "secondary"}`;
  button.dataset.updateAction = action;
  button.textContent = label;
  return button;
}

export function renderUpdateSnapshot(
  host: HTMLElement,
  snapshot: UpdateSnapshot,
  onAction?: (action: "install" | "dismiss", version: string) => void,
): void {
  currentSnapshot = snapshot;
  host.replaceChildren();
  host.className = `update-status update-${snapshot.phase}`;
  host.setAttribute("role", "status");
  host.setAttribute("aria-live", "polite");

  if (snapshot.phase === "idle") {
    host.hidden = true;
    return;
  }
  host.hidden = false;

  if (snapshot.phase === "checking") {
    host.append(text("span", "Checking for updates…"));
    return;
  }
  if (snapshot.phase === "up-to-date") {
    host.append(text("span", "periScope is up to date."));
    return;
  }
  if (snapshot.phase === "dismissed") {
    host.append(text("span", "Update dismissed until periScope restarts."));
    return;
  }
  if (snapshot.phase === "failed") {
    host.append(
      text("strong", "Update check unavailable"),
      text(
        "span",
        snapshot.message ?? "Could not check for updates. Try again next time.",
      ),
    );
    return;
  }

  const candidate = snapshot.candidate;
  if (!candidate) {
    host.append(text("span", "Update status unavailable."));
    return;
  }

  if (snapshot.phase === "available") {
    const copy = document.createElement("div");
    copy.className = "update-copy";
    copy.append(
      text("strong", `periScope ${candidate.version} is available`),
      text("span", candidate.notes, "update-notes"),
    );
    const actions = document.createElement("div");
    actions.className = "update-actions";
    actions.append(
      actionButton("dismiss", "Not now"),
      actionButton("install", "Update and restart"),
    );
    actions.addEventListener("click", (event) => {
      const button = (event.target as Element).closest<HTMLButtonElement>(
        "[data-update-action]",
      );
      if (!button || !onAction) return;
      actions
        .querySelectorAll<HTMLButtonElement>("button")
        .forEach((action) => (action.disabled = true));
      onAction(
        button.dataset.updateAction as "install" | "dismiss",
        candidate.version,
      );
    });
    host.append(copy, actions);
    return;
  }

  if (snapshot.phase === "downloading") {
    const downloaded = snapshot.downloadedBytes ?? 0;
    const total = snapshot.totalBytes;
    const progress = document.createElement("div");
    progress.className = "update-progress";
    progress.setAttribute("role", "progressbar");
    progress.setAttribute(
      "aria-label",
      `Downloading periScope ${candidate.version}`,
    );
    progress.setAttribute("aria-valuemin", "0");
    if (total && total > 0) {
      const percent = Math.min(100, Math.round((downloaded / total) * 100));
      progress.setAttribute("aria-valuemax", "100");
      progress.setAttribute("aria-valuenow", String(percent));
      progress.append(text("span", `Downloading update… ${percent}%`));
      const meter = document.createElement("i");
      meter.style.width = `${percent}%`;
      progress.append(meter);
    } else {
      progress.append(text("span", "Downloading update…"));
    }
    host.append(progress);
    return;
  }

  if (snapshot.phase === "installing") {
    host.append(
      text("strong", `Installing periScope ${candidate.version}`),
      text(
        "span",
        "periScope will restart automatically when the update is ready.",
      ),
    );
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

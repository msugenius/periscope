import { invoke } from "@tauri-apps/api/core";
import {
  displayHotkey,
  escapeHtml,
  isModifierCode,
  shortcutFromEvent,
  type CrosshairKey,
  type CrosshairSettings,
  type HotkeySettings,
  type Settings,
} from "./ui-model";
import { connectUpdater, renderCurrentUpdate } from "./update-ui";

type SettingsPage = "crosshair" | "hotkeys";

const defaults: Settings = {
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
  hotkeys: { closeApp: "F3", showSettings: "F4" },
  hotkeyErrors: {},
};

let settings = { ...defaults };
let saveTimer: number | undefined;
let currentPage: SettingsPage = "crosshair";
let recordingHotkey: keyof HotkeySettings | null = null;
let recordingReleaseCode: string | null = null;
let nativeRecording = false;
let hotkeyStatus = "Changes save automatically.";
let hotkeyStatusError = false;
let saveStatus = "Changes save automatically";
let saveStatusError = false;

const icon = (name: string) => {
  const paths: Record<string, string> = {
    crosshair:
      '<circle cx="12" cy="12" r="5"/><path d="M12 2v4m0 12v4M2 12h4m12 0h4"/>',
    sliders:
      '<path d="M4 7h10m4 0h2M4 17h2m4 0h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>',
    tray: '<path d="M4 15h16l-2 5H6zM12 3v10m-4-4 4 4 4-4"/>',
    power: '<path d="M12 2v10m-6.4-6.4a9 9 0 1 0 12.8 0"/>',
    minus: '<path d="M5 12h14"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>',
    keyboard:
      '<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M18 10h.01M7 14h7m2 0h2"/>',
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true">${paths[name]}</svg>`;
};

const range = (
  key: CrosshairKey,
  label: string,
  min: number,
  max: number,
  suffix = "",
) => `
  <label class="control-row" for="${key}">
    <span>${label}</span>
    <input class="range" id="${key}" data-key="${key}" type="range" min="${min}" max="${max}" value="${settings[key]}" />
    <span class="number-wrap"><input class="number" data-key="${key}" type="number" min="${min}" max="${max}" value="${settings[key]}"/><small>${suffix}</small></span>
  </label>`;

const toggle = (key: CrosshairKey, label: string, help?: string) => `
  <label class="toggle-row" for="${key}">
    <span><strong>${label}</strong>${help ? `<small>${help}</small>` : ""}</span>
    <input id="${key}" data-key="${key}" type="checkbox" ${settings[key] ? "checked" : ""}/>
    <i aria-hidden="true"></i>
  </label>`;

function renderCrosshairPage() {
  return `
    <div class="page-heading">
      <div><span class="eyebrow">Overlay editor</span><h1>Crosshair</h1><p>Tune every detail and see it on-screen instantly.</p></div>
      <label class="master-switch"><input id="enabled" data-key="enabled" type="checkbox" ${settings.enabled ? "checked" : ""}/><i>${icon("power")}</i><span>${settings.enabled ? "Enabled" : "Disabled"}</span></label>
    </div>

    <div class="editor-grid">
      <section class="preview-panel panel">
        <div class="panel-heading"><div><h2>Live preview</h2><p>Pixel-accurate representation</p></div><span class="preview-badge">Center</span></div>
        <div class="preview-stage">
          <div class="ambient ambient-a"></div><div class="ambient ambient-b"></div>
          <div class="grid-floor"></div>
          <canvas id="preview" width="720" height="440"></canvas>
          <span class="axis axis-x"></span><span class="axis axis-y"></span>
          <div class="preview-coordinates"><span>X ${settings.xOffset}</span><span>Y ${settings.yOffset}</span></div>
        </div>
        <div class="preset-heading"><span>Quick shapes</span><small>Choose a base, then fine-tune</small></div>
        <div class="presets">
          <button class="preset active" data-preset="classic"><span class="mini-cross classic"></span><small>Classic</small></button>
          <button class="preset" data-preset="compact"><span class="mini-cross compact"></span><small>Compact</small></button>
          <button class="preset" data-preset="dot"><span class="mini-dot"></span><small>Dot</small></button>
          <button class="preset" data-preset="open"><span class="mini-cross open"></span><small>Open</small></button>
          <button class="preset" data-preset="precision"><span class="mini-cross t-shape"></span><small>T-Shape</small></button>
        </div>
      </section>

      <div class="controls-column">
        <section class="panel settings-card">
          <div class="panel-heading"><div><h2>Shape</h2><p>Geometry and composition</p></div>${icon("crosshair")}</div>
          ${range("length", "Length", 1, 64, "px")}
          ${range("thickness", "Thickness", 1, 16, "px")}
          ${range("gap", "Gap", 0, 32, "px")}
          <div class="divider"></div>
          ${toggle("centerDot", "Center dot")}
          <div class="conditional ${settings.centerDot ? "" : "muted"}">${range("dotSize", "Dot size", 1, 16, "px")}</div>
          ${toggle("tStyle", "T-style", "Remove the upper arm")}
        </section>

        <section class="panel settings-card">
          <div class="panel-heading"><div><h2>Color & visibility</h2><p>Contrast against any scene</p></div>${icon("sliders")}</div>
          <label class="color-row"><span>Crosshair color</span><div><input id="color" data-key="color" type="color" value="${settings.color}"/><input class="hex" data-key="color" value="${settings.color.toUpperCase()}" maxlength="7"/></div></label>
          ${range("opacity", "Opacity", 5, 100, "%")}
          <div class="divider"></div>
          ${toggle("outline", "Outline", "Improve visibility on bright scenes")}
          <div class="conditional ${settings.outline ? "" : "muted"}">${range("outlineThickness", "Outline size", 1, 8, "px")}</div>
        </section>

        <section class="panel settings-card placement-card">
          <div class="panel-heading"><div><h2>Placement</h2><p>Offset from exact screen center</p></div><button id="center-position" class="icon-button" title="Reset position">${icon("crosshair")}</button></div>
          ${range("xOffset", "Horizontal", -200, 200, "px")}
          ${range("yOffset", "Vertical", -200, 200, "px")}
        </section>
      </div>
    </div>`;
}

function hotkeyRow(
  key: keyof HotkeySettings,
  label: string,
  description: string,
  defaultValue: string,
) {
  const error = settings.hotkeyErrors[key];
  const recording = recordingHotkey === key;
  return `
    <div class="hotkey-row">
      <div class="hotkey-copy"><strong>${label}</strong><span>${description}</span></div>
      <div class="hotkey-control">
        <button class="hotkey-binding ${recording ? "recording" : ""}" data-hotkey="${key}" aria-label="Change ${label} shortcut" aria-pressed="${recording}">${recording ? "Press shortcut..." : escapeHtml(displayHotkey(settings.hotkeys[key]))}</button>
        <small>Default ${defaultValue}</small>
      </div>
      ${error ? `<p class="hotkey-error" role="alert">${escapeHtml(error)}</p>` : ""}
    </div>`;
}

function renderHotkeysPage() {
  const configurationError = settings.hotkeyErrors.configuration;
  return `
    <div class="page-heading hotkeys-heading">
      <div><span class="eyebrow">Application controls</span><h1>Hotkeys</h1><p>Use global shortcuts while periScope runs, even when Settings is hidden.</p></div>
      <span class="global-badge">${icon("keyboard")} System-wide</span>
    </div>
    <section class="panel hotkeys-card">
      <div class="panel-heading"><div><h2>Shortcut bindings</h2><p>Select a binding, then press a new key combination.</p></div>${icon("keyboard")}</div>
      ${configurationError ? `<div class="hotkey-banner" role="alert">${escapeHtml(configurationError)}</div>` : ""}
      <div class="hotkey-list">
        ${hotkeyRow("closeApp", "Close app", "Exit periScope, including the overlay and tray icon.", "F3")}
        ${hotkeyRow("showSettings", "Show settings", "Open, restore, and focus this Settings window.", "F4")}
      </div>
      <div class="hotkeys-actions">
        <p id="hotkey-status" class="hotkey-status ${hotkeyStatusError ? "error" : ""}" aria-live="polite">${escapeHtml(hotkeyStatus)}</p>
        <button id="reset-hotkeys" class="button secondary">Reset hotkeys</button>
      </div>
    </section>`;
}

function renderShell() {
  document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
    <main class="window-shell">
      <header class="titlebar" data-tauri-drag-region>
        <div class="brand" data-tauri-drag-region>
          <span class="brand-mark">${icon("crosshair")}</span>
          <span>periScope</span>
          <button id="update-status" class="status" type="button" hidden></button>
        </div>
        <div class="window-actions">
          <button id="minimize" aria-label="Minimize">${icon("minus")}</button>
          <button id="close" aria-label="Hide settings">${icon("close")}</button>
        </div>
      </header>

      <div class="workspace">
        <aside class="sidebar">
          <nav>
            <button class="nav-item ${currentPage === "crosshair" ? "active" : ""}" data-page="crosshair">${icon("crosshair")}<span>Crosshair</span></button>
            <button class="nav-item ${currentPage === "hotkeys" ? "active" : ""}" data-page="hotkeys">${icon("keyboard")}<span>Hotkeys</span></button>
          </nav>
        </aside>

        <section class="content">${currentPage === "crosshair" ? renderCrosshairPage() : renderHotkeysPage()}</section>
      </div>

      <footer>
        <div class="save-state ${saveStatusError ? "error" : ""}"><i></i><span>${escapeHtml(saveStatus)}</span></div>
        <div class="footer-actions">${currentPage === "crosshair" ? '<button id="reset" class="button secondary">Reset defaults</button>' : ""}<button id="hide" class="button primary">${icon("tray")}Hide settings<span>Overlay stays active</span></button></div>
      </footer>
    </main>`;

  bindEvents();
  drawPreview();
  renderCurrentUpdate(document.querySelector<HTMLElement>("#update-status")!);
}

function bindEvents() {
  document
    .querySelectorAll<HTMLButtonElement>("[data-page]")
    .forEach((button) => {
      button.addEventListener("click", async () => {
        await stopRecording();
        currentPage = button.dataset.page as SettingsPage;
        renderShell();
      });
    });

  document.querySelectorAll<HTMLInputElement>("[data-key]").forEach((input) => {
    const eventName =
      input.type === "range" || input.type === "color" ? "input" : "change";
    input.addEventListener(eventName, () => updateFromInput(input));
  });

  document
    .querySelector("#minimize")
    ?.addEventListener("click", () => invoke("minimize_settings"));
  document.querySelector("#close")?.addEventListener("click", async () => {
    await stopRecording();
    await invoke("hide_settings");
  });
  document.querySelector("#hide")?.addEventListener("click", async () => {
    await stopRecording();
    await invoke("hide_settings");
  });
  document.querySelector("#reset")?.addEventListener("click", async () => {
    const crosshair = await invoke<CrosshairSettings>("reset_settings");
    settings = { ...settings, ...crosshair };
    renderShell();
  });
  document.querySelector("#center-position")?.addEventListener("click", () => {
    settings.xOffset = 0;
    settings.yOffset = 0;
    syncAndSave(true);
  });
  document
    .querySelectorAll<HTMLButtonElement>("[data-preset]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        applyPreset(button.dataset.preset!),
      );
    });
  document
    .querySelectorAll<HTMLButtonElement>("[data-hotkey]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        beginRecording(button.dataset.hotkey as keyof HotkeySettings),
      );
    });
  document
    .querySelector("#reset-hotkeys")
    ?.addEventListener("click", resetHotkeys);
}

async function setNativeRecording(recording: boolean) {
  if (nativeRecording === recording) return;
  await invoke("set_hotkey_recording", { recording });
  nativeRecording = recording;
}

async function beginRecording(key: keyof HotkeySettings) {
  try {
    await setNativeRecording(true);
    recordingHotkey = key;
    recordingReleaseCode = null;
    hotkeyStatus = "Press a key combination. Escape cancels.";
    hotkeyStatusError = false;
    renderShell();
    document
      .querySelector<HTMLButtonElement>(`[data-hotkey="${key}"]`)
      ?.focus();
  } catch (error) {
    showHotkeyError(error);
  }
}

async function stopRecording() {
  recordingHotkey = null;
  recordingReleaseCode = null;
  try {
    await setNativeRecording(false);
  } catch (error) {
    console.error("Could not restore hotkey dispatch", error);
  }
}

async function handleRecordingKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  if (event.repeat || isModifierCode(event.code)) return;

  if (event.code === "Escape") {
    recordingReleaseCode = event.code;
    recordingHotkey = null;
    hotkeyStatus = "Shortcut change cancelled.";
    hotkeyStatusError = false;
    renderShell();
    return;
  }

  const key = recordingHotkey;
  const proposed = shortcutFromEvent(event);
  const otherKey: keyof HotkeySettings =
    key === "closeApp" ? "showSettings" : "closeApp";
  if (proposed.toLowerCase() === settings.hotkeys[otherKey].toLowerCase()) {
    hotkeyStatus = "That shortcut is already assigned to the other action.";
    hotkeyStatusError = true;
    renderShell();
    document
      .querySelector<HTMLButtonElement>(`[data-hotkey="${key}"]`)
      ?.focus();
    return;
  }

  const proposedSettings = { ...settings.hotkeys, [key]: proposed };
  try {
    const accepted = await invoke<HotkeySettings>("update_hotkeys", {
      hotkeys: proposedSettings,
    });
    settings.hotkeys = accepted;
    settings.hotkeyErrors = {};
    recordingReleaseCode = event.code;
    recordingHotkey = null;
    hotkeyStatus = `${key === "closeApp" ? "Close app" : "Show settings"} saved as ${displayHotkey(accepted[key])}.`;
    hotkeyStatusError = false;
    renderShell();
  } catch (error) {
    showHotkeyError(error);
    document
      .querySelector<HTMLButtonElement>(`[data-hotkey="${key}"]`)
      ?.focus();
  }
}

async function handleRecordingKeyUp(event: KeyboardEvent) {
  if (!recordingReleaseCode || event.code !== recordingReleaseCode) return;
  recordingReleaseCode = null;
  try {
    await setNativeRecording(false);
  } catch (error) {
    showHotkeyError(error);
  }
}

async function resetHotkeys() {
  await stopRecording();
  try {
    settings.hotkeys = await invoke<HotkeySettings>("reset_hotkeys");
    settings.hotkeyErrors = {};
    hotkeyStatus = "Hotkeys reset to F3 and F4.";
    hotkeyStatusError = false;
  } catch (error) {
    showHotkeyError(error, false);
    return;
  }
  renderShell();
}

function showHotkeyError(error: unknown, rerender = true) {
  hotkeyStatus =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : String(error);
  hotkeyStatusError = true;
  if (rerender) renderShell();
}

function updateFromInput(input: HTMLInputElement) {
  const key = input.dataset.key as CrosshairKey;
  if (!key) return;
  let value: string | number | boolean = input.value;
  if (input.type === "checkbox") value = input.checked;
  if (input.type === "range" || input.type === "number")
    value = Number(input.value);
  if (input.type === "color") value = input.value.toUpperCase();
  (settings as unknown as Record<string, unknown>)[key] = value;

  document
    .querySelectorAll<HTMLInputElement>(`[data-key="${key}"]`)
    .forEach((peer) => {
      if (peer === input) return;
      if (peer.type === "checkbox") peer.checked = Boolean(value);
      else peer.value = String(value);
    });

  if (["centerDot", "outline", "enabled"].includes(key)) void syncAndSave(true);
  else {
    drawPreview();
    scheduleSave();
  }
}

function applyPreset(preset: string) {
  const presets: Record<string, Partial<Settings>> = {
    classic: {
      length: 10,
      thickness: 1,
      gap: 3,
      centerDot: true,
      dotSize: 2,
      tStyle: false,
    },
    compact: {
      length: 5,
      thickness: 2,
      gap: 2,
      centerDot: false,
      tStyle: false,
    },
    dot: {
      length: 1,
      thickness: 1,
      gap: 0,
      centerDot: true,
      dotSize: 3,
      tStyle: false,
    },
    open: {
      length: 8,
      thickness: 1,
      gap: 6,
      centerDot: false,
      tStyle: false,
    },
    precision: {
      length: 14,
      thickness: 1,
      gap: 2,
      centerDot: true,
      dotSize: 1,
      tStyle: true,
    },
  };
  Object.assign(settings, presets[preset]);
  syncAndSave(true);
}

function scheduleSave() {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => syncAndSave(false), 40);
}

async function syncAndSave(rerender: boolean) {
  try {
    const crosshair = await invoke<CrosshairSettings>("update_settings", {
      settings,
    });
    settings = { ...settings, ...crosshair };
    setSaveStatus("Changes save automatically", false);
  } catch (error) {
    const detail =
      typeof error === "string"
        ? error
        : error instanceof Error
          ? error.message
          : String(error);
    setSaveStatus(`Could not save settings: ${detail}`, true);
  }
  if (rerender) renderShell();
}

function setSaveStatus(message: string, error: boolean) {
  saveStatus = message;
  saveStatusError = error;
  const element = document.querySelector<HTMLElement>(".save-state");
  element?.classList.toggle("error", error);
  const label = element?.querySelector("span");
  if (label) label.textContent = message;
}

function drawPreview() {
  const canvas = document.querySelector<HTMLCanvasElement>("#preview");
  if (!canvas) return;
  const ctx = canvas.getContext("2d")!;
  const scale = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.round(rect.width * scale));
  canvas.height = Math.max(1, Math.round(rect.height * scale));
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, rect.width, rect.height);
  if (!settings.enabled) return;

  const cx = rect.width / 2;
  const cy = rect.height / 2;
  ctx.globalAlpha = settings.opacity / 100;
  ctx.lineCap = "butt";

  const arms: [number, number, number, number][] = [
    [cx - settings.gap - settings.length, cy, cx - settings.gap, cy],
    [cx + settings.gap, cy, cx + settings.gap + settings.length, cy],
    [cx, cy + settings.gap, cx, cy + settings.gap + settings.length],
  ];
  if (!settings.tStyle)
    arms.push([cx, cy - settings.gap - settings.length, cx, cy - settings.gap]);

  if (settings.outline) {
    ctx.strokeStyle = settings.outlineColor;
    ctx.lineWidth = settings.thickness + settings.outlineThickness * 2;
    arms.forEach(([x1, y1, x2, y2]) => {
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    });
  }
  ctx.strokeStyle = settings.color;
  ctx.lineWidth = settings.thickness;
  arms.forEach(([x1, y1, x2, y2]) => {
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  });

  if (settings.centerDot) {
    if (settings.outline) {
      ctx.fillStyle = settings.outlineColor;
      ctx.beginPath();
      ctx.arc(
        cx,
        cy,
        settings.dotSize / 2 + settings.outlineThickness,
        0,
        Math.PI * 2,
      );
      ctx.fill();
    }
    ctx.fillStyle = settings.color;
    ctx.beginPath();
    ctx.arc(cx, cy, settings.dotSize / 2, 0, Math.PI * 2);
    ctx.fill();
  }
}

export async function boot() {
  try {
    settings = await invoke<Settings>("get_settings");
  } catch (error) {
    console.error("Could not load native settings; using defaults", error);
  }
  renderShell();
  const appWindow = window as Window & {
    __periScopeCleanup?: () => void;
  };
  appWindow.__periScopeCleanup?.();
  const events = new AbortController();
  let updaterCleanup: (() => void) | undefined;
  appWindow.__periScopeCleanup = () => {
    events.abort();
    updaterCleanup?.();
  };
  void connectUpdater(
    document.querySelector<HTMLElement>("#update-status")!,
  ).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else updaterCleanup = cleanup;
  });
  window.addEventListener("resize", drawPreview, { signal: events.signal });
  window.addEventListener("keydown", handleRecordingKeyDown, {
    capture: true,
    signal: events.signal,
  });
  window.addEventListener("keyup", handleRecordingKeyUp, {
    capture: true,
    signal: events.signal,
  });
  window.addEventListener(
    "beforeunload",
    () => {
      if (nativeRecording)
        void invoke("set_hotkey_recording", { recording: false });
    },
    { signal: events.signal },
  );
}

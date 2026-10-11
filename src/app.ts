import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  displayHotkey,
  escapeHtml,
  isModifierCode,
  reservedShortcutReason,
  shortcutFromEvent,
  type CrosshairKey,
  type HotkeyName,
  type HotkeySettings,
  type Preset,
  type PresetId,
  type Settings,
  type VisualSettings,
} from "./ui-model";
import { connectUpdater, renderCurrentUpdate } from "./update-ui";

type SettingsPage = "crosshair" | "presets" | "hotkeys";

const defaultVisual: VisualSettings = {
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

const visualKeys = Object.keys(defaultVisual) as (keyof VisualSettings)[];

const defaults: Settings = {
  hideWhenAds: false,
  enabled: true,
  ...defaultVisual,
  activePreset: "classic",
  presets: [
    {
      id: "dot",
      name: "Dot",
      settings: { ...defaultVisual, dotSize: 3, dotOnly: true },
    },
    { id: "classic", name: "Classic", settings: { ...defaultVisual } },
    {
      id: "precision",
      name: "T-Shape",
      settings: {
        ...defaultVisual,
        length: 14,
        gap: 2,
        dotSize: 1,
        tStyle: true,
      },
    },
  ],
  hotkeys: {
    toggleCrosshair: "F2",
    toggleAds: "F5",
  },
  hotkeyErrors: {},
};

let settings = { ...defaults };
let previewQueue: Promise<unknown> = Promise.resolve();
let visualRevision = 0;
let currentPage: SettingsPage = "crosshair";
let recordingHotkey: HotkeyName | null = null;
let recordingAttemptInFlight = false;
let nativeRecording = false;
let hotkeyError = "";
let saveStatus = "Preset saved";
let saveStatusError = false;

const icon = (name: string) => {
  const paths: Record<string, string> = {
    crosshair:
      '<circle cx="12" cy="12" r="5"/><path d="M12 2v4m0 12v4M2 12h4m12 0h4"/>',
    power: '<path d="M12 2v10m-6.4-6.4a9 9 0 1 0 12.8 0"/>',
    minus: '<path d="M5 12h14"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>',
    keyboard:
      '<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M18 10h.01M7 14h7m2 0h2"/>',
    trash: '<path d="M4 7h16m-10 4v6m4-6v6M9 7l1-3h4l1 3m3 0-1 14H7L6 7"/>',
    copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/>',
    plus: '<path d="M12 4v16M4 12h16"/>',
    edit: '<path d="m4 16-.5 4.5L8 20l11-11-4-4L4 16Z"/><path d="m13 7 4 4"/>',
    upload: '<path d="M12 16V3m-5 5 5-5 5 5M4 16v4h16v-4"/>',
    palette:
      '<path d="M12 2a10 10 0 0 0 0 20 2 2 0 0 0 2-2c0-.5-.2-1-.5-1.4-.3-.4-.5-.8-.5-1.1a2 2 0 0 1 2-2h2.5A4.5 4.5 0 0 0 22 11 9 9 0 0 0 12 2Z"/><circle cx="8" cy="8" r=".6" fill="currentColor"/><circle cx="13" cy="6" r=".6" fill="currentColor"/><circle cx="17.5" cy="10" r=".6" fill="currentColor"/><circle cx="7" cy="13" r=".6" fill="currentColor"/>',
    settings:
      '<polygon points="10,2 14,2 14.5,4.5 16,5.2 18.2,3.8 20.2,5.8 18.8,8 19.5,9.5 22,10 22,14 19.5,14.5 18.8,16 20.2,18.2 18.2,20.2 16,18.8 14.5,19.5 14,22 10,22 9.5,19.5 8,18.8 5.8,20.2 3.8,18.2 5.2,16 4.5,14.5 2,14 2,10 4.5,9.5 5.2,8 3.8,5.8 5.8,3.8 8,5.2 9.5,4.5"/><circle cx="12" cy="12" r="3"/>',
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

const toggle = (key: CrosshairKey, label: string) => `
  <label class="toggle-row" for="${key}">
    <span><strong>${label}</strong></span>
    <input id="${key}" data-key="${key}" type="checkbox" ${settings[key] ? "checked" : ""}/>
    <i aria-hidden="true"></i>
  </label>`;

function renderCrosshairPage() {
  const style = currentShapeStyle();
  return `
    <div class="page-heading crosshair-heading">
      <h1>Crosshair</h1>
      <div class="shape-switch" role="group" aria-label="Crosshair style">
        <button type="button" class="shape-option ${style === "cross" ? "active" : ""}" data-shape-style="cross" aria-pressed="${style === "cross"}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v18M3 12h18"/></svg>Cross</button>
        <button type="button" class="shape-option ${style === "t" ? "active" : ""}" data-shape-style="t" aria-pressed="${style === "t"}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 9h18M12 9v12"/></svg>T-Shape</button>
        <button type="button" class="shape-option ${style === "dot" ? "active" : ""}" data-shape-style="dot" aria-pressed="${style === "dot"}"><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none"/></svg>Dot</button>
      </div>
    </div>

    <div class="editor-grid">
      <section class="panel settings-card visibility-card">
        <div class="panel-heading"><h2>Color & visibility</h2>${icon("sliders")}</div>
        <label class="color-row"><span>Crosshair color</span><div><input id="color" data-key="color" type="color" value="${settings.color}"/><input class="hex" data-key="color" value="${settings.color.toUpperCase()}" maxlength="7"/></div></label>
        ${range("opacity", "Opacity", 5, 100, "%")}
        <div class="divider"></div>
        ${toggle("outline", "Outline")}
        <div class="conditional ${settings.outline ? "" : "muted"}">${range("outlineThickness", "Outline size", 1, 8, "px")}</div>
      </section>

      <section class="panel settings-card shape-card">
        <div class="panel-heading"><h2>Shape</h2>${icon("crosshair")}</div>
        ${
          settings.dotOnly
            ? range("dotSize", "Dot size", 1, 16, "px")
            : `
          ${range("length", "Length", 1, 64, "px")}
          ${range("thickness", "Thickness", 1, 16, "px")}
          ${range("gap", "Gap", 0, 32, "px")}
          <div class="divider"></div>
          ${toggle("centerDot", "Center dot")}
          <div class="conditional ${settings.centerDot ? "" : "muted"}">${range("dotSize", "Dot size", 1, 16, "px")}</div>`
        }
      </section>

    </div>`;
}

function presetPreview(preset: Preset) {
  const style = preset.settings;
  const scale = Math.min(1, 23 / (style.length + style.gap));
  const length = style.length * scale;
  const gap = style.gap * scale;
  const weight = Math.max(1.3, style.thickness * scale);
  const x = 40 + style.xOffset * 0.08;
  const y = 40 + style.yOffset * 0.08;
  const segments = style.dotOnly
    ? []
    : [
        [x - gap - length, y, x - gap, y],
        [x + gap, y, x + gap + length, y],
        [x, y + gap, x, y + gap + length],
        ...(style.tStyle ? [] : [[x, y - gap - length, x, y - gap]]),
      ];
  const strokes = (color: string, thickness: number) =>
    segments
      .map(
        ([x1, y1, x2, y2]) =>
          `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${escapeHtml(color)}" stroke-width="${thickness}"/>`,
      )
      .join("");
  const dot = (color: string, radius: number) =>
    style.dotOnly || style.centerDot
      ? `<circle cx="${x}" cy="${y}" r="${radius}" fill="${escapeHtml(color)}" stroke="none"/>`
      : "";
  return `<svg class="preset-preview" viewBox="0 0 80 80" aria-hidden="true"><g opacity="${style.opacity / 100}">
    ${style.outline ? strokes(style.outlineColor, weight + style.outlineThickness * 2) + dot(style.outlineColor, Math.max(1, (style.dotSize * scale) / 2) + style.outlineThickness) : ""}
    ${strokes(style.color, weight)}${dot(style.color, Math.max(1, (style.dotSize * scale) / 2))}</g></svg>`;
}

function renderPresetsPage() {
  return `<div class="page-heading preset-heading"><h1>Presets</h1><button id="import-preset" class="button secondary preset-import-button">${icon("upload")} Import by code</button></div>
    <div class="preset-grid">
        ${settings.presets
          .map((preset) => {
            const active = preset.id === settings.activePreset;
            return `<article class="preset-card ${active ? "active" : ""}">
            <button class="preset-select" data-preset="${escapeHtml(preset.id)}" aria-pressed="${active}" aria-label="Select ${escapeHtml(preset.name)}">${presetPreview(preset)}<span>${escapeHtml(preset.name)}</span></button>
            <div class="preset-card-actions">
              <button data-copy-preset="${escapeHtml(preset.id)}" title="Copy share code" aria-label="Copy ${escapeHtml(preset.name)} code" ${active && isDirty() ? "disabled" : ""}>${icon("copy")}</button>
              <button data-rename-preset="${escapeHtml(preset.id)}" title="Rename" aria-label="Rename ${escapeHtml(preset.name)}">${icon("edit")}</button>
              <button data-clone-preset="${escapeHtml(preset.id)}" title="Clone" aria-label="Clone ${escapeHtml(preset.name)}">${icon("plus")}</button>
              <button data-delete-preset="${escapeHtml(preset.id)}" title="Delete" aria-label="Delete ${escapeHtml(preset.name)}" ${settings.presets.length === 1 ? "disabled" : ""}>${icon("trash")}</button>
            </div>
          </article>`;
          })
          .join("")}
        <button id="create-preset" class="preset-card add-preset">${icon("plus")}<span>Add new</span></button>
    </div>`;
}

function hotkeyRow(key: HotkeyName, label: string, description: string) {
  const error = settings.hotkeyErrors[key];
  const recording = recordingHotkey === key;
  return `
    <div class="hotkey-row">
      <div class="hotkey-copy">
        <strong>${escapeHtml(label)}</strong>
        <span class="hotkey-info">
          <button type="button" class="hotkey-info-trigger" aria-label="${escapeHtml(description)}">i</button>
          <span class="hotkey-tooltip" role="tooltip" aria-hidden="true">${escapeHtml(description)}</span>
        </span>
      </div>
      <div class="hotkey-control">
        <div class="hotkey-input">
          <button class="hotkey-binding ${recording ? "recording" : ""} ${settings.hotkeys[key] ? "" : "is-empty"}" data-hotkey="${key}" aria-label="Change ${label} shortcut" aria-pressed="${recording}">${recording ? "Press shortcut..." : settings.hotkeys[key] ? escapeHtml(displayHotkey(settings.hotkeys[key])) : "Not set"}</button>
          <button class="hotkey-clear" data-clear-hotkey="${key}" aria-label="Clear ${label} shortcut" title="Clear shortcut" ${settings.hotkeys[key] ? "" : "disabled"}>${icon("trash")}</button>
        </div>
      </div>
      ${error ? `<p class="hotkey-error" role="alert">${escapeHtml(error)}</p>` : ""}
    </div>`;
}

function renderSettingsPage() {
  const configurationError = settings.hotkeyErrors.configuration;
  return `
    <div class="page-heading">
      <h1>Settings</h1>
    </div>
    <section class="panel hotkeys-card">
      <div class="panel-heading"><h2>Hotkeys</h2>${icon("keyboard")}</div>
      ${configurationError ? `<div class="hotkey-banner" role="alert">${escapeHtml(configurationError)}</div>` : ""}
      <div class="hotkey-list">
        ${hotkeyRow("toggleCrosshair", "Toggle crosshair", "Enable or disable the crosshair overlay without opening Settings.")}
        ${hotkeyRow("toggleAds", "Hide when ADS", "Toggle hiding the crosshair while holding the right mouse button.")}
      </div>
      <div class="hotkeys-actions">
        ${hotkeyError ? `<p id="hotkey-feedback" class="hotkey-error" role="alert">${escapeHtml(hotkeyError)}</p>` : ""}
        <p id="hotkey-status" class="hotkey-status">Changes save automatically.</p>
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
          <button id="minimize" aria-label="Hide settings">${icon("minus")}</button>
          <button id="close" aria-label="Hide settings">${icon("close")}</button>
        </div>
      </header>

      <div class="workspace">
        <aside class="sidebar">
          <nav>
            <button class="nav-item ${currentPage === "crosshair" ? "active" : ""}" data-page="crosshair">${icon("crosshair")}<span>Crosshair</span></button>
            <button class="nav-item ${currentPage === "presets" ? "active" : ""}" data-page="presets">${icon("palette")}<span>Presets</span></button>
            <button class="nav-item ${currentPage === "hotkeys" ? "active" : ""}" data-page="hotkeys">${icon("settings")}<span>Settings</span></button>
          </nav>
        </aside>

        <section class="content">${currentPage === "crosshair" ? renderCrosshairPage() : currentPage === "presets" ? renderPresetsPage() : renderSettingsPage()}</section>
      </div>

      <footer>
        <div class="save-state ${saveStatusError ? "error" : ""}"><span>${escapeHtml(saveStatus)}</span></div>
        <div class="footer-actions"><button id="cancel" class="button secondary" ${isDirty() ? "" : "disabled"}>Cancel</button><button id="save" class="button primary" ${isDirty() ? "" : "disabled"}>Save</button></div>
      </footer>
    </main>`;

  bindEvents();
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
    .querySelectorAll<HTMLButtonElement>("[data-shape-style]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        setShapeStyle(button.dataset.shapeStyle as ShapeStyle),
      );
    });

  document
    .querySelector("#minimize")
    ?.addEventListener("click", () => void closeSettings());
  document
    .querySelector("#close")
    ?.addEventListener("click", () => void closeSettings());
  document
    .querySelector("#cancel")
    ?.addEventListener("click", () => void cancelDraft());
  document
    .querySelector("#save")
    ?.addEventListener("click", () => void saveDraft());
  document
    .querySelectorAll<HTMLButtonElement>("[data-preset]")
    .forEach((button) => {
      button.addEventListener(
        "click",
        () => void applyPreset(button.dataset.preset as PresetId),
      );
    });
  document
    .querySelector("#create-preset")
    ?.addEventListener("click", () => void createPreset());
  document
    .querySelector("#import-preset")
    ?.addEventListener("click", () => void importPreset());
  for (const [selector, action] of [
    ["data-copy-preset", copyPreset],
    ["data-rename-preset", renamePreset],
    ["data-clone-preset", clonePreset],
    ["data-delete-preset", deletePreset],
  ] as const) {
    document
      .querySelectorAll<HTMLButtonElement>(`[${selector}]`)
      .forEach((button) => {
        button.addEventListener(
          "click",
          () => void action(button.getAttribute(selector)!),
        );
      });
  }
  document
    .querySelectorAll<HTMLButtonElement>("[data-hotkey]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        beginRecording(button.dataset.hotkey as HotkeyName),
      );
    });
  document
    .querySelectorAll<HTMLButtonElement>("[data-clear-hotkey]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        clearHotkey(button.dataset.clearHotkey as HotkeyName),
      );
    });
}

async function setNativeRecording(recording: boolean) {
  if (nativeRecording === recording) return;
  await invoke("set_hotkey_recording", { recording });
  nativeRecording = recording;
}

async function beginRecording(key: HotkeyName) {
  try {
    await setNativeRecording(true);
    recordingHotkey = key;
    recordingAttemptInFlight = false;
    hotkeyError = "";
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
  try {
    await setNativeRecording(false);
  } catch (error) {
    console.error("Could not restore hotkey dispatch", error);
  }
}

function recordFocusedKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  if (
    event.repeat ||
    isModifierCode(event.code) ||
    event.code === "Unidentified"
  )
    return;
  void handleRecordedShortcut(shortcutFromEvent(event));
}

async function handleRecordedShortcut(proposed: string) {
  if (!recordingHotkey || recordingAttemptInFlight) return;
  recordingAttemptInFlight = true;
  try {
    await applyRecordedShortcut(proposed);
  } finally {
    recordingAttemptInFlight = false;
  }
}

async function applyRecordedShortcut(proposed: string) {
  if (!recordingHotkey) return;
  if (proposed === "Escape") {
    await stopRecording();
    hotkeyError = "";
    renderShell();
    return;
  }

  const key = recordingHotkey;
  const reservedReason = reservedShortcutReason(proposed);
  if (reservedReason) {
    hotkeyError = reservedReason;
    renderShell();
    document
      .querySelector<HTMLButtonElement>(`[data-hotkey="${key}"]`)
      ?.focus();
    return;
  }
  const duplicate = Object.entries(settings.hotkeys).some(
    ([otherKey, shortcut]) =>
      otherKey !== key && shortcut.toLowerCase() === proposed.toLowerCase(),
  );
  if (duplicate) {
    hotkeyError = "That shortcut is already assigned to another action.";
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
    await stopRecording();
    hotkeyError = "";
    renderShell();
  } catch (error) {
    showHotkeyError(error);
    document
      .querySelector<HTMLButtonElement>(`[data-hotkey="${key}"]`)
      ?.focus();
  }
}

async function updateHotkeys(proposed: HotkeySettings) {
  try {
    const accepted = await invoke<HotkeySettings>("update_hotkeys", {
      hotkeys: proposed,
    });
    settings.hotkeys = accepted;
    settings.hotkeyErrors = {};
    hotkeyError = "";
    renderShell();
    return true;
  } catch (error) {
    showHotkeyError(error);
    return false;
  }
}

async function clearHotkey(key: HotkeyName) {
  await stopRecording();
  if (!settings.hotkeys[key]) return;
  await updateHotkeys({ ...settings.hotkeys, [key]: "" });
}

function showHotkeyError(error: unknown, rerender = true) {
  hotkeyError =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : String(error);
  if (rerender) renderShell();
}

type PresetState = Pick<Settings, "activePreset" | "presets" | "enabled"> &
  VisualSettings;
type ShapeStyle = "cross" | "t" | "dot";

function currentShapeStyle(): ShapeStyle {
  return settings.dotOnly ? "dot" : settings.tStyle ? "t" : "cross";
}

function setShapeStyle(style: ShapeStyle) {
  if (currentShapeStyle() === style) return;
  settings.dotOnly = style === "dot";
  settings.tStyle = style === "t";
  if (settings.dotOnly) settings.centerDot = true;
  visualRevision += 1;
  refreshDirty();
  queuePreview();
  renderShell();
}

function currentVisual(): VisualSettings {
  return Object.fromEntries(
    visualKeys.map((key) => [key, settings[key]]),
  ) as VisualSettings;
}

function isDirty() {
  const saved = settings.presets.find(
    (preset) => preset.id === settings.activePreset,
  )?.settings;
  return !!saved && visualKeys.some((key) => settings[key] !== saved[key]);
}

function refreshDirty() {
  const dirty = isDirty();
  for (const id of ["save", "cancel"]) {
    const button = document.querySelector<HTMLButtonElement>(`#${id}`);
    if (button) button.disabled = !dirty;
  }
  document
    .querySelectorAll<HTMLButtonElement>("[data-copy-preset]")
    .forEach((button) => {
      button.disabled =
        dirty && button.dataset.copyPreset === settings.activePreset;
    });
  setSaveStatus(
    dirty ? "Unsaved preset changes · previewing" : "Preset saved",
    false,
  );
}

function applyPresetState(state: PresetState, preserveDraft = false) {
  const draft = preserveDraft ? currentVisual() : null;
  settings = { ...settings, ...state, ...(draft ?? {}) };
  refreshDirty();
  renderShell();
}

function errorText(error: unknown) {
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : String(error);
}

function updateFromInput(input: HTMLInputElement) {
  const key = input.dataset.key as CrosshairKey;
  if (!key) return;
  let value: string | number | boolean = input.value;
  if (input.type === "checkbox") value = input.checked;
  if (input.type === "range" || input.type === "number") {
    value = Number(input.value);
    if (!Number.isFinite(value)) return;
    value = Math.min(Number(input.max), Math.max(Number(input.min), value));
    input.value = String(value);
  }
  if (key === "color") {
    if (!/^#[0-9a-f]{6}$/i.test(String(value))) return;
    value = String(value).toUpperCase();
  }
  if (key === "enabled") {
    const previous = settings.enabled;
    settings.enabled = Boolean(value);
    renderShell();
    void invoke<boolean>("set_crosshair_enabled", {
      enabled: settings.enabled,
    }).catch((error) => {
      settings.enabled = previous;
      setSaveStatus(
        `Could not change overlay state: ${errorText(error)}`,
        true,
      );
      renderShell();
    });
    return;
  }
  (settings as unknown as Record<string, unknown>)[key] = value;
  visualRevision += 1;
  document
    .querySelectorAll<HTMLInputElement>(`[data-key="${key}"]`)
    .forEach((peer) => {
      if (peer === input) return;
      if (peer.type === "checkbox") peer.checked = Boolean(value);
      else peer.value = String(value);
    });
  refreshDirty();
  queuePreview();
  if (key === "centerDot" || key === "outline") renderShell();
}

function queuePreview() {
  const preview = currentVisual();
  previewQueue = previewQueue
    .then(() => invoke("preview_settings", { settings: preview }))
    .catch((error) => {
      setSaveStatus(`Could not preview: ${errorText(error)}`, true);
    });
}

async function saveDraft(): Promise<boolean> {
  if (!isDirty()) return true;
  await previewQueue;
  const revision = visualRevision;
  const draft = currentVisual();
  try {
    const state = await invoke<PresetState>("save_preset_settings", {
      preset: settings.activePreset,
      settings: draft,
    });
    const changedWhileSaving = visualRevision !== revision;
    applyPresetState(state, changedWhileSaving);
    if (changedWhileSaving) queuePreview();
    return !changedWhileSaving;
  } catch (error) {
    setSaveStatus(`Could not save preset: ${errorText(error)}`, true);
    return false;
  }
}

async function cancelDraft(): Promise<boolean> {
  await previewQueue;
  const revision = visualRevision;
  try {
    await invoke("cancel_preview");
    if (visualRevision !== revision) {
      queuePreview();
      return false;
    }
    const saved = settings.presets.find(
      (preset) => preset.id === settings.activePreset,
    );
    if (saved) settings = { ...settings, ...saved.settings };
    refreshDirty();
    renderShell();
    return true;
  } catch (error) {
    setSaveStatus(`Could not cancel preview: ${errorText(error)}`, true);
    return false;
  }
}

async function guardDraft(): Promise<boolean> {
  if (!isDirty()) return true;
  const choice = await askChoice(
    "Unsaved preset changes",
    "Save your changes before switching?",
    [
      ["save", "Save", "primary"],
      ["discard", "Discard", "secondary"],
      ["stay", "Stay", "secondary"],
    ],
  );
  if (choice === "save") return saveDraft();
  if (choice === "discard") return cancelDraft();
  return false;
}

async function closeSettings() {
  if (!(await guardDraft())) return;
  await stopRecording();
  try {
    await invoke("hide_settings");
  } catch (error) {
    setSaveStatus(`Could not hide settings: ${errorText(error)}`, true);
  }
}

async function applyPreset(preset: PresetId) {
  if (preset === settings.activePreset || !(await guardDraft())) return;
  await runLibraryAction("select_preset", { preset });
}

async function runLibraryAction(
  command: string,
  args: Record<string, unknown>,
  preserveDraft = false,
) {
  await previewQueue;
  try {
    const state = await invoke<PresetState>(command, args);
    applyPresetState(state, preserveDraft);
    if (preserveDraft) queuePreview();
  } catch (error) {
    setSaveStatus(`Could not update presets: ${errorText(error)}`, true);
  }
}

async function createPreset() {
  const name = await askText(
    "New preset",
    "Name (leave empty for a random name)",
    "",
  );
  if (name === null || !(await guardDraft())) return;
  await runLibraryAction("create_preset", { name: name.trim() || null });
}

async function importPreset() {
  const code = await askText("Import preset", "Paste a preset code", "", true);
  if (!code?.trim() || !(await guardDraft())) return;
  await runLibraryAction("import_preset", { code: code.trim() });
}

async function clonePreset(preset: string) {
  if (!(await guardDraft())) return;
  await runLibraryAction("clone_preset", { preset });
}

async function renamePreset(preset: string) {
  const old = settings.presets.find((item) => item.id === preset);
  if (!old) return;
  const name = await askText("Rename preset", "Preset name", old.name);
  if (name === null || name.trim() === old.name) return;
  await runLibraryAction("rename_preset", { preset, name }, isDirty());
}

async function deletePreset(preset: string) {
  const old = settings.presets.find((item) => item.id === preset);
  if (!old || settings.presets.length === 1) return;
  const choice = await askChoice("Delete preset", `Delete “${old.name}”?`, [
    ["delete", "Delete", "danger"],
    ["stay", "Cancel", "secondary"],
  ]);
  if (
    choice !== "delete" ||
    (preset === settings.activePreset && !(await guardDraft()))
  )
    return;
  await runLibraryAction("delete_preset", { preset }, isDirty());
}

async function copyPreset(preset: string) {
  if (preset === settings.activePreset && isDirty()) return;
  try {
    const code = await invoke<string>("export_preset", { preset });
    await navigator.clipboard.writeText(code);
    setSaveStatus("Preset code copied", false);
  } catch (error) {
    setSaveStatus(`Could not copy code: ${errorText(error)}`, true);
  }
}

function askChoice(
  title: string,
  message: string,
  choices: [string, string, string][],
): Promise<string> {
  return new Promise((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "dialog-backdrop";
    backdrop.innerHTML = `<div class="app-dialog" role="dialog" aria-modal="true" aria-label="${escapeHtml(title)}">
      <h2>${escapeHtml(title)}</h2><p>${escapeHtml(message)}</p>
      <div class="dialog-actions">${choices.map(([value, label, style]) => `<button class="button ${style}" data-choice="${escapeHtml(value)}">${escapeHtml(label)}</button>`).join("")}</div>
    </div>`;
    const finish = (choice: string) => {
      window.removeEventListener("keydown", onKeyDown);
      backdrop.remove();
      resolve(choice);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish("stay");
    };
    backdrop
      .querySelectorAll<HTMLButtonElement>("[data-choice]")
      .forEach((button) => {
        button.addEventListener("click", () => finish(button.dataset.choice!));
      });
    document.body.append(backdrop);
    window.addEventListener("keydown", onKeyDown);
    backdrop.querySelector<HTMLButtonElement>("button")?.focus();
  });
}

function askText(
  title: string,
  label: string,
  initial: string,
  multiline = false,
): Promise<string | null> {
  return new Promise((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "dialog-backdrop";
    backdrop.innerHTML = `<form class="app-dialog" role="dialog" aria-modal="true" aria-label="${escapeHtml(title)}">
      <h2>${escapeHtml(title)}</h2><label>${escapeHtml(label)}
      ${multiline ? `<textarea name="value" rows="5">${escapeHtml(initial)}</textarea>` : `<input name="value" maxlength="40" value="${escapeHtml(initial)}" />`}</label>
      <div class="dialog-actions"><button type="button" class="button secondary" data-cancel>Cancel</button><button type="submit" class="button primary">Confirm</button></div>
    </form>`;
    const field = backdrop.querySelector<
      HTMLInputElement | HTMLTextAreaElement
    >("[name=value]")!;
    const finish = (value: string | null) => {
      window.removeEventListener("keydown", onKeyDown);
      backdrop.remove();
      resolve(value);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(null);
    };
    backdrop.querySelector("form")!.addEventListener("submit", (event) => {
      event.preventDefault();
      finish(field.value);
    });
    backdrop
      .querySelector("[data-cancel]")!
      .addEventListener("click", () => finish(null));
    document.body.append(backdrop);
    window.addEventListener("keydown", onKeyDown);
    field.focus();
    if (!multiline) (field as HTMLInputElement).select();
  });
}

function setSaveStatus(message: string, error: boolean) {
  saveStatus = message;
  saveStatusError = error;
  const element = document.querySelector<HTMLElement>(".save-state");
  element?.classList.toggle("error", error);
  const label = element?.querySelector("span");
  if (label) label.textContent = message;
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
  let adsErrorCleanup: (() => void) | undefined;
  let recordedCleanup: (() => void) | undefined;
  let enabledCleanup: (() => void) | undefined;
  let closeCleanup: (() => void) | undefined;
  appWindow.__periScopeCleanup = () => {
    events.abort();
    updaterCleanup?.();
    adsErrorCleanup?.();
    recordedCleanup?.();
    enabledCleanup?.();
    closeCleanup?.();
  };
  void listen<string>("hide-when-ads-error", (event) => {
    setSaveStatus(`Could not change ADS mode: ${event.payload}`, true);
  }).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else adsErrorCleanup = cleanup;
  });
  void listen<string>("hotkey-recorded", (event) => {
    void handleRecordedShortcut(event.payload);
  }).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else recordedCleanup = cleanup;
  });
  void listen<boolean>("crosshair-enabled-changed", (event) => {
    settings.enabled = event.payload;
    if (currentPage === "crosshair") renderShell();
  }).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else enabledCleanup = cleanup;
  });
  void listen("settings-close-requested", () => {
    void closeSettings();
  }).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else closeCleanup = cleanup;
  });
  void connectUpdater(
    document.querySelector<HTMLElement>("#update-status")!,
  ).then((cleanup) => {
    if (events.signal.aborted) cleanup();
    else updaterCleanup = cleanup;
  });
  window.addEventListener("keydown", recordFocusedKeyDown, {
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

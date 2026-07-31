export type HotkeySettings = {
  closeApp: string;
  showSettings: string;
};

export type HotkeyErrors = Partial<
  Record<keyof HotkeySettings | "configuration", string>
>;

export type Settings = {
  enabled: boolean;
  color: string;
  opacity: number;
  length: number;
  thickness: number;
  gap: number;
  centerDot: boolean;
  dotSize: number;
  tStyle: boolean;
  outline: boolean;
  outlineThickness: number;
  outlineColor: string;
  xOffset: number;
  yOffset: number;
  hotkeys: HotkeySettings;
  hotkeyErrors: HotkeyErrors;
};

export type CrosshairSettings = Omit<Settings, "hotkeys" | "hotkeyErrors">;
export type CrosshairKey = keyof CrosshairSettings;

export function escapeHtml(value: string) {
  return value.replace(
    /[&<>'"]/g,
    (character) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "'": "&#39;",
        '"': "&quot;",
      })[character]!,
  );
}

export function displayHotkey(value: string) {
  return value
    .split("+")
    .map((token) => {
      if (token === "Control") return "Ctrl";
      if (token === "Super") return "Win";
      if (/^Key[A-Z]$/.test(token)) return token.slice(3);
      if (/^Digit[0-9]$/.test(token)) return token.slice(5);
      return token;
    })
    .join(" + ");
}

export function isModifierCode(code: string) {
  return [
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "ShiftLeft",
    "ShiftRight",
    "MetaLeft",
    "MetaRight",
  ].includes(code);
}

export function shortcutFromEvent(event: KeyboardEvent) {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");
  parts.push(event.code);
  return normalizeShortcut(parts.join("+"));
}

export function normalizeShortcut(value: string) {
  const aliases: Record<string, string> = {
    alt: "Alt",
    control: "Control",
    ctrl: "Control",
    meta: "Super",
    shift: "Shift",
    super: "Super",
    win: "Super",
    windows: "Super",
  };
  const tokens = value
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  if (tokens.length === 0) return "";

  const keyToken = tokens.pop()!;
  const modifiers = new Set(
    tokens.map((token) => aliases[token.toLowerCase()] ?? token),
  );
  const ordered = ["Control", "Alt", "Shift", "Super"].filter((modifier) =>
    modifiers.has(modifier),
  );
  let key = keyToken;
  if (/^[a-z]$/i.test(key)) key = `Key${key.toUpperCase()}`;
  else if (/^[0-9]$/.test(key)) key = `Digit${key}`;
  return [...ordered, key].join("+");
}

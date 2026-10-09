# Settings Contract

- `get_settings` returns `hideWhenAds: boolean` in the existing settings view.
- `set_hide_when_ads({ enabled: boolean })` returns the accepted boolean, or an error string. On error, the old preference and runtime behavior stay active.
- The Hotkeys-page toggle calls this command on change and shows a save error if rejected.

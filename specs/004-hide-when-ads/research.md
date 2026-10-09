# Research

- Windows low-level mouse hooks deliver global right-button down/up events on the installing thread's message loop. Returning `CallNextHookEx` preserves the input chain.
- The existing overlay thread already runs a message loop; installing and removing the observer there avoids another worker or polling.
- An observer callback should do only transition handling and use `ShowWindow` for hide; release can use the existing redraw path.
- The preference belongs outside preset snapshots and outside shortcut bindings, since neither preset selection nor shortcut reset should change it.

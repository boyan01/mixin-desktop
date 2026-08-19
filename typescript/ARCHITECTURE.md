# TypeScript UI Architecture

The TypeScript client uses a lightweight feature-first React structure. Flutter remains the UI and interaction specification; the TypeScript folders describe ownership and dependencies rather than a separate product design.

## Directory ownership

```text
src/
├── main.tsx                 # React entry only
├── app/                     # Composition, providers, lifecycle, shell, global styles
├── state/                   # Cross-feature ephemeral desktop UI state
├── features/                # User-facing capabilities grouped by domain
│   ├── auth/
│   ├── chat/
│   ├── conversations/
│   ├── protocol/
│   ├── settings/
│   └── ...
└── shared/                  # Domain-neutral bridge, model, platform, state, and UI primitives
```

- `app` composes features and owns process-level React effects.
- `state` owns the small cross-feature Zustand store; it must not become a second durable data source.
- `features` own their UI, feature-local hooks, and pure models.
- `shared` provides reusable leaves and never imports from `app` or `features`.
- Flutter assets are referenced through the narrow `@flutter-assets` alias so feature moves do not create fragile relative paths or expose Flutter source as a TypeScript dependency.

ESLint enforces the layer direction:

```text
app ───────▶ features ───────▶ shared
 │                              ▲
 └──────────────────────────────┘
```

Features cannot import from `app`, and shared modules cannot import from either upper layer. Cross-feature dependencies should stay narrow and one-directional. If two features need to coordinate, prefer composition in `app` or a small shared model over mutual imports.

## React ownership rules

- Components own presentation and view-local state.
- Custom hooks own subscriptions or reusable stateful behavior with a concrete domain name.
- React Query owns Rust-backed and remote data.
- Zustand owns cross-tree ephemeral desktop UI state. Durable state remains in Rust.
- Pure calculations live in feature `model/` or `lib/` files and do not import React components.
- Platform event listeners must subscribe once, clean up on unmount, and use an Effect Event when the listener needs the latest render state.
- Keep component inputs explicit. Do not add pass-through wrappers or broad context objects solely to shorten prop lists.

## Chat dependency graph

```text
Chat
├──▶ ChatTimeline ──▶ MessageRow ──▶ MessageContent ──▶ model
├──▶ ChatOverlays ───▶ MessageRow
└──▶ ChatComposer ────────────────────────────────▶ model
```

`ChatTimeline` owns the virtualizer and scroll lifecycle. `ChatComposer` owns draft, focus, attachment, sticker, and voice lifecycles. These owners should stay stable when leaf renderers are extracted.

## Styling

`app/styles.css` imports ordered style slices. The import order preserves the existing cascade:

```text
foundation → shell → sidebar → shared controls → conversations → dialogs
           → chat → composer → protocol → conversation details → settings
           → startup → responsive overrides
```

Keep Flutter-derived measurements and interaction selectors intact during structural changes. Move a selector only with its complete rule and preserve stylesheet order.

## Validation

Run commands from `typescript/`:

```sh
npm run check
npm run build
npm run tauri dev
```

`check` runs ESLint, React Hooks checks, dependency-boundary rules, and TypeScript. The production build validates bundling and asset resolution. A Tauri smoke test is still required for native APIs, lifecycle, and visual parity.

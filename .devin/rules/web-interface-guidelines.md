---
description: Vercel Web Interface Guidelines for frontend code
trigger: glob
globs: "client/**/*.{tsx,ts,css,md}"
---

# Web Interface Guidelines

When writing or reviewing UI code in `client/`, follow these rules adapted from the [Vercel Web Interface Guidelines](https://github.com/vercel-labs/web-interface-guidelines).

## Accessibility

- Icon-only buttons need `aria-label`.
- Form controls need an associated `<label>` or `aria-label`.
- Interactive elements need keyboard handlers (`onKeyDown`/`onKeyUp`) where appropriate.
- Use `<button>` for actions; `<a>` / `next/link` for navigation (not `<div>` with click handlers).
- Images need `alt` text (or `alt=""` if decorative).
- Decorative icons need `aria-hidden="true"`.
- Async updates (toasts, validation) need `aria-live="polite"`.
- Prefer semantic HTML (`<nav>`, `<main>`, `<section>`, `<article>`) before adding ARIA roles.
- Keep headings hierarchical (`<h1>`–`<h6>`); include a skip link for main content on full pages.
- Add `scroll-margin-top` to heading anchors.

## Focus States

- Interactive elements need a visible focus indicator: `focus-visible:ring-*` or an equivalent.
- Never use `outline-none` / `outline: none` without a focus replacement.
- Use `:focus-visible` over `:focus` to avoid focus rings on mouse clicks.
- Use `:focus-within` for compound controls.

## Forms

- Inputs need `autocomplete` and a meaningful `name` attribute.
- Use the correct `type` (`email`, `tel`, `url`, `number`) and `inputmode` where applicable.
- Never block paste (`onPaste` + `preventDefault`).
- Labels must be clickable (`htmlFor` or wrapping the control).
- Disable spellcheck on emails, codes, and usernames (`spellCheck={false}`).
- Checkboxes/radios: label + control share a single hit target (no dead zones).
- Submit buttons stay enabled until the request starts; show a spinner during the request.
- Errors appear inline next to fields; focus the first error on submit.
- Placeholders end with `…` and show an example pattern.
- Use `autocomplete="off"` on non-auth fields to avoid password-manager triggers.
- Warn before navigation with unsaved changes (`beforeunload` or a router guard).

## Animation

- Honor `prefers-reduced-motion` (provide a reduced variant or disable animations).
- Animate `transform`/`opacity` only (compositor-friendly properties).
- Never use `transition: all` — list properties explicitly.
- Set correct `transform-origin`.
- SVG transforms should be applied to a `<g>` wrapper with `transform-box: fill-box; transform-origin: center`.
- Animations must be interruptible and respond to user input mid-animation.

## Typography

- Use `…` (ellipsis) not `...`.
- Use curly quotes `"` and `"` instead of straight quotes in user-facing copy.
- Use non-breaking spaces for pairs like `10 MB`, `⌘ K`, and brand names.
- Loading states end with `…`: `"Loading…"`, `"Saving…"`.
- Use `font-variant-numeric: tabular-nums` for number columns and comparisons.
- Use `text-wrap: balance` or `text-pretty` on headings to prevent widows.

## Content Handling

- Text containers must handle long content: `truncate`, `line-clamp-*`, or `break-words`.
- Flex children need `min-w-0` to allow text truncation.
- Handle empty states — don't render broken UI for empty strings or arrays.
- User-generated content should be tested for short, average, and very long inputs.

## Images

- `<img>` needs explicit `width` and `height` to prevent CLS.
- Below-fold images: `loading="lazy"`.
- Above-fold critical images: `priority` or `fetchpriority="high"`.

## Performance

- Virtualize large lists (`>50` items) with `virtua` or `content-visibility: auto`.
- No layout reads in render (`getBoundingClientRect`, `offsetHeight`, `offsetWidth`, `scrollTop`).
- Batch DOM reads/writes; avoid interleaving them.
- Prefer uncontrolled inputs; controlled inputs must be cheap per keystroke.
- Add `preconnect` for CDN/asset domains.
- Critical fonts: load with `font-display: swap`.

## Navigation & State

- The URL should reflect state — filters, tabs, pagination, and expanded panels belong in query params.
- Links use `<a>` / `next/link` (support Cmd/Ctrl+click and middle-click).
- Deep-link all stateful UI; if it uses `useState`, consider syncing to the URL (e.g. via `nuqs`).
- Destructive actions need a confirmation modal or an undo window — never immediate execution.

## Touch & Interaction

- Use `touch-action: manipulation` to prevent double-tap zoom delay.
- Set `-webkit-tap-highlight-color` intentionally.
- Use `overscroll-behavior: contain` in modals, drawers, and sheets.
- During drag: disable text selection and set `inert` on dragged elements.
- Use `autoFocus` sparingly — desktop only, single primary input; avoid on mobile.

## Safe Areas & Layout

- Full-bleed layouts need `env(safe-area-inset-*)` for notches.
- Avoid unwanted scrollbars: `overflow-x-hidden` on containers and fix content overflow.
- Prefer Flex/Grid over JavaScript measurement for layout.

## Dark Mode & Theming

- Add `color-scheme: dark` to `<html>` for dark themes (fixes scrollbar and native inputs).
- `<meta name="theme-color">` must match the page background.
- Native `<select>` and `<input>` need explicit `background-color` and `color` (Windows dark mode).

## Locale & Internationalization

- Dates and times: use `Intl.DateTimeFormat`, not hardcoded formats.
- Numbers and currency: use `Intl.NumberFormat`, not hardcoded formats.
- Detect language via `Accept-Language` / `navigator.languages`, not IP.
- Wrap brand names, code tokens, and identifiers with `translate="no"` to prevent garbled auto-translation.

## Hydration Safety

- Inputs with `value` need `onChange` (or use `defaultValue` for uncontrolled inputs).
- Date/time rendering must guard against hydration mismatch (server vs client).
- Use `suppressHydrationWarning` only where truly needed.

## Hover & Interactive States

- Buttons and links need a visible `hover:` state.
- Interactive states should increase contrast: hover/active/focus must be more prominent than the rest state.

## Content & Copy

- Use active voice: "Install the CLI" not "The CLI will be installed".
- Use Title Case for headings and buttons (Chicago style).
- Use numerals for counts: "8 deployments" not "eight".
- Use specific button labels: "Save API Key" not "Continue".
- Error messages must include a fix or next step, not just the problem.
- Use second person; avoid first person.
- Use `&` over "and" where space-constrained.

## Anti-patterns (flag these)

- `user-scalable=no` or `maximum-scale=1` disabling zoom.
- `onPaste` with `preventDefault`.
- `transition: all`.
- `outline-none` without a `focus-visible` replacement.
- Inline `onClick` navigation without `<a>` / `next/link`.
- `<div>` or `<span>` with click handlers that should be `<button>` or `<a>`.
- Images without dimensions.
- Large arrays `.map()` without virtualization.
- Form inputs without labels.
- Icon buttons without `aria-label`.
- Hardcoded date/number formats (use `Intl.*`).
- `autoFocus` without clear justification.

# Claresso — brand basics

> Working brand for the AI-coding governance platform (formerly codenamed CCGuard).
> Locked 2026-06-10: aperture-C mark + Sora wordmark + Azure.

## The mark
A **"C" drawn as a precise aperture / focus ring** — reads as the letter *and* as clarity/focus (never an eye). Opens to the right. Vector only; holds from 16px favicon to signage, single-color-safe, reverses on dark.

- Construction: a ~290° arc of a circle (r20 in a 64 viewBox), `stroke-linecap: round`, ~70° opening centered on the right.
- Path: `M48.38 20.53 A20 20 0 1 0 48.38 43.47`, stroke-width 7.5 (use 9–10 at favicon sizes).

## Lockups
- **Primary (integrated):** the mark IS the capital C → `[mark]laresso`. Spacing "B/close": mark height ≈ 0.9em, tucked with ~-0.14em right margin so "laresso" nestles into the C's opening. Implement in web as inline SVG mark + Sora text.
- **Horizontal alternate (side):** `[mark]  Claresso` — mark left of the full wordmark. For tight headers/footers. See `claresso-lockup.svg`.
- **App icon / favicon:** aperture-C in an Azure rounded tile (`claresso-app-icon.svg`) / standalone mark (`claresso-favicon.svg`).

## Color tokens
| token | hex | use |
|---|---|---|
| **Claresso Azure** | `#2F6BFF` | the mark, primary accent, CTAs, links |
| Azure deep | `#1E47C8` | hover / pressed |
| Azure wash | `#EAF0FF` | tints, soft fills |
| Ink | `#0F1422` | wordmark, body text on light |
| Ink-2 | `#5A6478` | secondary text |
| Night | `#0D1220` | dark backgrounds; mark shifts to `#5B87FF` on night |
| Paper | `#FFFFFF` | light background |

Single-color black = ink `#0F1422`. Reverse (on dark) = white wordmark + `#5B87FF` mark.

## Type
- **Wordmark + display:** Sora 600 (geometric sans), tracking ≈ -0.02 to -0.035em.
- **Body / UI:** Inter.
- **Mono (data/HUD):** JetBrains Mono.

## Files
`claresso-mark.svg` · `claresso-mark-mono.svg` · `claresso-favicon.svg` · `claresso-app-icon.svg` · `claresso-lockup.svg`
(Wordmark lockups use the Sora webfont via `<text>` — outline to paths in a vector tool for print/embroidery.)

## Don'ts
Don't fill the C's center (reads as an eye). Don't add a focal dot inside the ring. Don't recolor the mark outside the token set. Don't stretch or rotate. Don't set the wordmark in a different typeface.

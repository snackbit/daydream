# Daydream Design Reference

> Warm cream editorial with light headlines. A Bauhaus studio notebook: eggshell
> paper, black ink, a violet and orange spark for product moments.

**Primary theme:** light

Daydream's visual direction follows the supplied ElevenLabs style reference
closely. Keep its palette, typography, pill buttons, soft cards and hairline
dividers. Adapt the layout to a desktop recall workspace where recordings,
timestamps and source evidence need to stay readable.

This document defines the intended visual system. It is not an inventory of
implemented components. See [README.md](README.md) for current capabilities and
[the desktop stylesheet](apps/desktop/src/index.css) for the active runtime tokens.
CSS examples below are reference recipes, not a replacement stylesheet.

Daydream uses an eggshell canvas (#fdfcfc), black type and warm taupe surfaces
(#f5f3f1). Display headlines use Waldenburg at weight 300, or the bundled Inter
at weight 300, with tight -0.02em tracking. Inter at 400/500 carries body copy and
controls. Blue-violet #0447ff and orange #ff4704 appear inside product visuals.
Components stay flat or barely elevated, with 1px borders, 20px card radii and
fully rounded pill buttons. The result should feel calm, editorial and precise.

## Tokens: Colors

| Name | Value | Token | Role |
| --- | --- | --- | --- |
| Eggshell | `#fdfcfc` | `--color-eggshell` | Page canvas, button surfaces and elevated cards; the warm paper background. |
| Warm Taupe | `#f5f3f1` | `--color-warm-taupe` | Panels, cards and secondary surfaces; one step deeper than eggshell. |
| Stone | `#ebe8e4` | `--color-stone` | Hairline dividers, decorative borders and small icon plates. |
| Ink | `#000000` | `--color-ink` | Primary text, filled buttons, navigation and links. |
| Graphite | `#44403b` | `--color-graphite` | Strong secondary text and section labels. |
| Smoke | `#777169` | `--color-smoke` | Reference tone for subdued descriptions and large secondary text. |
| Ash | `#a59f97` | `--color-ash` | Decorative or disabled details; avoid for essential text. |
| Violet Spark | `#0447ff` | `--color-violet-spark` | Product illustration accent only; this reference name denotes a saturated blue. |
| Ember Orange | `#ff4704` | `--color-ember-orange` | Product illustration accent only; pair with Violet Spark inside artwork. |

### Desktop readability and state colors

Use the existing semantic tokens for working UI. Daydream uses `#6d675f` for
light-theme muted text and `#8a8379` for control borders. Stone remains appropriate
for decorative separation. Smoke and Ash should not make timestamps, source
citations, input boundaries or help text difficult to read.

| Role | Light value | Runtime token |
| --- | --- | --- |
| Readable muted text | `#6d675f` | `--muted-foreground` |
| Control border | `#8a8379` | `--control-border` |
| Recording / destructive | `#b42318` | `--status-recording` / `--destructive` |
| Success | `#26713c` | `--status-success` |
| Warning | `#8a5900` | `--status-warning` |
| Information | `#245ec0` | `--status-info` |

These semantic colors communicate capture, processing and safety states. They
are separate from the two illustration sparks. Existing annotation colors may
identify timeline data. Pair state and track colors with text, icons or patterns;
color alone must not convey recording, failure, selection or redaction.

## Tokens: Typography

### Waldenburg: Display and headings

Use weight 300 at 32px, 36px and 48px. Light headlines and tight tracking establish
the editorial character. Reserve this treatment for display and page headings.

- Token: `--font-waldenburg`.
- Substitute: bundled Inter Variable at 300; Söhne Light is an optional licensed alternative.
- Weight: 300.
- Sizes: 32px, 36px, 48px.
- Line height: 1.08–1.17.
- Letter spacing: -0.02em, equivalent to -0.64px at 32px, -0.72px at 36px and -0.96px at 48px.
- OpenType features: `"ss01" on` if the chosen font supports it.

Inter Variable is currently bundled with Daydream. Waldenburg and Söhne are
reference choices, not bundled assets. Ship fonts locally; use the Inter fallback
unless a font is licensed and packaged for the app.

### Inter: Body and controls

Use Inter for navigation, buttons, inputs, card content and captions. Weight 400
is the default; use 500 for buttons, emphasized links and compact section titles.
Body copy at 14–16px has relaxed line height and +0.01em tracking.

- Token: `--font-inter`.
- Substitute: system-ui.
- Weights: 400, 500; 300 for the display fallback.
- Sizes: 10px, 12px, 13px, 14px, 15px, 16px, 18px, 20px.
- Line height: usually 1.47–1.6 for body copy; tighter for compact controls.
- Letter spacing: +0.01em at 14px, 15px and 16px; normal elsewhere.

Use 14px body text, 13px controls and 12px metadata in dense desktop workspaces.
Use 16px body text for spacious introductions and longer explanatory copy. Keep
10px type for optional decorative captions; essential metadata starts at 12px.

### Geist Mono: Technical metadata

Use monospace sparingly for technical labels, durations or diagnostics where
alignment helps reading. Ordinary prose and navigation stay in Inter.

- Token: `--font-geist-mono`.
- Substitute: JetBrains Mono, IBM Plex Mono or the system monospace stack.
- Weight: 400.
- Size: 13px.
- Line height: 1.69.

Geist Mono is a reference choice. The system monospace fallback works without
adding a font dependency. Use tabular numerals for changing times and counters.

### Type scale

| Role | Family | Weight | Size | Line height | Letter spacing | Token |
| --- | --- | --- | --- | --- | --- | --- |
| Decorative caption | Inter | 400 | 10px | 1.6 | normal | `--text-caption` |
| Metadata | Inter | 400 | 12px | 1.5 | normal | `--text-metadata` |
| Control | Inter | 500 | 13px | 1.4 | normal | `--text-control` |
| Body small | Inter | 400 | 14px | 1.5 | 0.01em | `--text-body-sm` |
| Body | Inter | 400 | 16px | 1.5 | 0.01em | `--text-body` |
| Subheading | Inter | 400 | 18px | 1.6 | normal | `--text-subheading` |
| Body large | Inter | 400 | 20px | 1.35 | normal | `--text-body-lg` |
| Heading small | Waldenburg / Inter | 300 | 32px | 1.13 | -0.02em | `--text-heading-sm` |
| Heading | Waldenburg / Inter | 300 | 36px | 1.17 | -0.02em | `--text-heading` |
| Display | Waldenburg / Inter | 300 | 48px | 1.08 | -0.02em | `--text-display` |

The table names reference roles. The active stylesheet uses `--text-body` for
14px desktop body text; map roles deliberately rather than overwriting runtime
tokens by name.

## Tokens: Spacing and Shapes

**Base unit:** 4px. **Density:** comfortable, with compact controls where the
timeline, playback and settings need them.

### Spacing scale

| Name | Value | Token |
| --- | --- | --- |
| 4 | 4px | `--spacing-4` |
| 8 | 8px | `--spacing-8` |
| 12 | 12px | `--spacing-12` |
| 16 | 16px | `--spacing-16` |
| 20 | 20px | `--spacing-20` |
| 24 | 24px | `--spacing-24` |
| 28 | 28px | `--spacing-28` |
| 32 | 32px | `--spacing-32` |
| 36 | 36px | `--spacing-36` |
| 40 | 40px | `--spacing-40` |
| 48 | 48px | `--spacing-48` |
| 56 | 56px | `--spacing-56` |
| 64 | 64px | `--spacing-64` |
| 72 | 72px | `--spacing-72` |
| 96 | 96px | `--spacing-96` |
| 160 | 160px | `--spacing-160` |

### Border radius

| Element | Value |
| --- | --- |
| Tags | 9999px |
| Cards and panels | 20px |
| Inputs | 4px reference; 8px for current desktop controls |
| Buttons | 9999px |
| Large cards and workspaces | 24px |
| Small elements | 4–10px |

### Shadows

Prefer flat surfaces and borders. Use the following complete recipes when a
surface needs separation; avoid broad, blurred elevation.

| Name | Value | Token |
| --- | --- | --- |
| Whisper elevation | `0 0 1px 0 rgba(0, 0, 0, 0.4), 0 1px 1px 0 rgba(0, 0, 0, 0.04), 0 2px 4px 0 rgba(0, 0, 0, 0.04)` | `--shadow-subtle` |
| Soft inset | `inset 0 0 0 0.5px rgba(0, 0, 0, 0.075)` | `--shadow-subtle-2` |
| Medium inset | `inset 0 0 0 0.5px rgba(0, 0, 0, 0.1)` | `--shadow-subtle-3` |
| Strong inset | `inset 0 0 0 1px rgba(0, 0, 0, 0.1)` | `--shadow-subtle-4` |
| Compact elevation | `0 0 1px 0 rgba(0, 0, 0, 0.4), 0 2px 4px 0 rgba(0, 0, 0, 0.04)` | `--shadow-subtle-5` |
| Light inset | `inset 0 0 0 1px rgba(255, 255, 255, 0.6)` | `--shadow-subtle-6` |
| Stone inset | `inset 0 0 0 0.5px #ebe8e4` | `--shadow-subtle-7` |

Inset shadows provide surface edges. Keyboard focus needs a separate, visible
ring; a faint inset edge is insufficient.

### Layout measurements

- Editorial page max-width: 1280px.
- Editorial outer gutters: up to 64px; reduce at smaller window sizes.
- Editorial section gap: 96–125px.
- Desktop content gutters: 16–32px.
- Desktop section gap: 24–48px.
- Card padding: 32px for feature cards, 16–24px for working panels.
- Element gap: 8–16px.

## Components

### Filled pill button

Primary action. Ink fill, eggshell text, 9999px radius, 16px horizontal padding,
Inter 14px/500. Use for actions such as “Start recording”, “Save” or “Done”.
The pill shape is the default button treatment. Give each view a clear primary
action and explicit hover, pressed, focus and disabled states.

### Outline pill button

Secondary action. Eggshell fill, Ink text, 9999px radius, 14px horizontal padding,
Inter 14px/500 and a 1px border. The reference border is `#e5e5e5`; use the
semantic control border when the boundary is necessary to identify the control.
Pair beside a filled button for actions such as “Cancel” or “Open in Studio”.

### Ghost link button

Tertiary navigation or inline action. Transparent fill, Ink text, 9999px radius
and Inter 14px/500. Show a taupe or stone surface on hover. Use a border only
where grouping requires it. Keep links identifiable and keyboard accessible.

### Feature card: Taupe

Warm Taupe fill, 20px radius, 32px padding, no shadow and no border. Use for
spacious introductions, setup explanations or feature summaries. This is the
main reference card pattern.

### Eggshell card with whisper shadow

Eggshell fill, 20px radius, 16px padding and whisper elevation. Use sparingly
for a card that needs to sit above another surface. Most content stays flat.

### Large feature card

Warm Taupe fill, 24px radius and generous internal padding. Use for a prominent
overview or product demonstration. Preserve breathing room around its heading.

### Tab pill

Eggshell fill, Ink text, 9999px radius and a 1px border. Indicate the selected
tab with a clear surface or border change and the appropriate selected state.
A dot may identify a data category, but selection remains clear without color.
Violet Spark and Ember Orange stay inside artwork.

### Hairline divider

1px solid Stone. Use for section, row and panel separation where whitespace
alone does not establish the boundary. Prefer this treatment to drop shadows.

### Audio sphere visual

An optional product illustration: a soft circular gradient, roughly 200px or
larger, blending Violet Spark and Ember Orange through warm intermediate tones.
A centered play control may overlay the illustration in a demonstration.
Keep decoration clear of actual waveform, progress and recording information.

### Wordmark

Black “Daydream” text in Inter semibold. Keep the wordmark simple and consistent
across the app and project materials. Use eggshell text in dark mode.

### App navigation

Use a minimal navigation rail or header on eggshell or taupe. Set navigation
labels in Inter at 13–14px. Keep the active destination clear through surface,
weight or border treatment. Use flat grouping and restrained monochrome icons.

### Recording strip

Recording state stays visible across navigation. Show a readable status label,
elapsed time and available capture controls. A semantic recording indicator may
use the recording color; its label must distinguish recording from paused or
stopped states. Keep capture status separate from processing progress.

### Timeline and evidence panels

Use taupe panels, eggshell media surfaces and hairline separators. Keep track
labels, timestamps, playback position and source links readable at compact
sizes. Use the existing annotation palette for data identity and clear patterns
for redacted spans. Hide eligible content according to the redaction contract.

### Library and day lists

Use generous row spacing on the eggshell canvas, with taupe grouping where
needed. Dates, durations and source context carry the hierarchy. Avoid wrapping
every row in an elevated card. Use monochrome source icons by default.

## Do's and Don'ts

### Do

- Use Waldenburg or Inter at weight 300 for display headlines at 32px and above.
- Set buttons, tags and tab pills to 9999px radius.
- Use black filled actions paired with eggshell outline actions.
- Reserve Violet Spark and Ember Orange for product artwork.
- Use 1px Stone dividers for decorative separation and stronger semantic borders for controls.
- Apply -0.02em tracking to display headings and +0.01em to Inter body at 14–16px.
- Stack warm surfaces as eggshell, taupe and stone.
- Keep recording controls, timestamps and source evidence clear at every window size.

### Don't

- Do not bold Waldenburg; the display treatment uses weight 300.
- Do not use the two illustration sparks for buttons, links, focus rings or status badges.
- Do not add heavy drop shadows or blurred elevation effects.
- Do not introduce decorative accent colors beyond the reference sparks.
- Do not use corners below 8px on cards or feature panels; use 20–24px radii.
- Do not use pure white for light-theme backgrounds; use eggshell.
- Do not use light display weights for compact body copy or working controls.
- Do not let pale helper text, decoration or large whitespace obscure capture state or evidence.

Semantic status and annotation colors are the desktop exceptions to the
otherwise neutral palette. Use them for meaning, with accessible labels.

## Surfaces

| Level | Name | Value | Purpose |
| --- | --- | --- | --- |
| 1 | Eggshell canvas | `#fdfcfc` | Base page background and media framing. |
| 2 | Warm Taupe | `#f5f3f1` | Cards, panels and section bands. |
| 3 | Stone plate | `#ebe8e4` | Small grouped elements and icon plates. |

### Dark theme

Light is the primary reference and the default for new installations. Preserve
the existing user-selectable dark theme with the same hierarchy and warm tones.

| Role | Dark value | Runtime token |
| --- | --- | --- |
| Canvas | `#191715` | `--background` |
| Primary text | `#fdfcfc` | `--foreground` |
| Card | `#24211e` | `--card` |
| Muted surface | `#302b26` | `--muted` |
| Secondary text | `#ded8d1` | `--secondary-foreground` |
| Muted text | `#b7aea4` | `--muted-foreground` |
| Divider | `#443d36` | `--border` |
| Control border | `#8a8075` | `--control-border` |

Use theme-aware semantic colors for statuses and annotations. Primary buttons
use foreground fill and background text so the hierarchy carries across themes.

## Elevation and Interaction

Buttons and occasional elevated cards may use the three-layer whisper shadow.
Panels generally use surface contrast or a hairline border. Menus and dialogs
need enough separation from the workspace to establish their boundary.

Use a visible 2px focus outline with a 2px offset, based on a contrasting semantic
foreground token. Keep focus visible around pills and inside scrollable panels.
Hover uses a small warm surface change; pressed states remain restrained.
Disabled controls stay identifiable and retain readable explanations.

Essential text should meet a 4.5:1 contrast target; large text and meaningful
control boundaries should meet 3:1. Keep interactive targets at least 24px in
both dimensions and prefer 32–40px for frequent desktop controls. Respect reduced
motion and avoid decorative looping animation near playback or recording state.

## Imagery

Use product visuals rather than lifestyle photography. Soft violet-to-orange
spheres can introduce audio or recall concepts in an overview or empty state.
Keep most icons monochrome, outlined or filled, with consistent stroke weight.
Technical content takes precedence over decoration in the working app.

Use synthetic recordings, text and window titles for public screenshots and
illustrations. Captured content stays local unless the user explicitly enables
the relevant remote capability. Remote settings should state the destination
and the data sent in plain language.

## Layout

Editorial introductions use a centered column up to 1280px wide, with up to 64px
outer gutters. A 48px light headline may sit beside a short description and pill
actions. Large feature panels use taupe surfaces and 96–125px section gaps.

The desktop app uses the same visual language at a practical density. Library
and Days can use a readable central column. Studio needs flexible media,
timeline and inspector regions that use the available width. Settings groups
related controls in flat panels with 16–24px padding and clear section labels.

Reduce gutters and stack secondary panels in narrow windows. Keep primary
actions and recording state reachable. Long text and source names wrap or
truncate with a way to inspect the full value. Preserve intentional scrolling
and avoid introducing page-wide horizontal overflow.

## Agent Prompt Guide

### Quick color reference

- Primary text and action fill: `#000000`.
- Secondary text: `#44403b`; readable muted desktop text: `#6d675f`.
- Canvas: `#fdfcfc`; card surface: `#f5f3f1`; small plate: `#ebe8e4`.
- Decorative divider: `#ebe8e4`; identifiable control border: `#8a8379`.
- Illustration accents: `#0447ff` and `#ff4704`.
- Status and timeline data: existing semantic tokens, paired with labels or patterns.

### Example component prompts

1. Create a page headline: “Your day, in view” at 48px Waldenburg or bundled Inter,
   weight 300, color #000000, letter-spacing -0.02em and line-height 1.08.
   Left-align it on an eggshell canvas.
2. Create a primary button: “Start recording”, with 9999px radius, black fill,
   eggshell text, Inter 14px/500 and 16px horizontal padding. Include a visible
   keyboard focus state.
3. Create a secondary button: “Open in Studio”, with 9999px radius, eggshell fill,
   black text, Inter 14px/500, 14px horizontal padding and a semantic control border.
4. Create an overview card: taupe fill, 20px radius, 32px padding and no shadow.
   Use a 36px light heading and 16px Inter body copy in readable muted text.
5. Create an optional audio illustration: a 200px soft circle blending #0447ff
   and #ff4704 through warm intermediate tones. Keep it separate from actual
   capture status and timeline controls.

## Quick Start

### CSS custom properties

These values describe the light reference. Integrate them through the existing
semantic theme in the desktop stylesheet; keep dark-mode mappings and semantic
status colors. Fixed values belong in the reference palette, while components
should consume the theme-aware runtime roles.

```css
:root {
  /* Reference palette */
  --color-eggshell: #fdfcfc;
  --color-warm-taupe: #f5f3f1;
  --color-stone: #ebe8e4;
  --color-ink: #000000;
  --color-graphite: #44403b;
  --color-smoke: #777169;
  --color-ash: #a59f97;
  --color-violet-spark: #0447ff;
  --color-ember-orange: #ff4704;

  /* Locally packaged Inter is the default display substitute. */
  --font-waldenburg: 'Waldenburg', 'Inter Variable', 'Inter', ui-sans-serif,
    system-ui, sans-serif;
  --font-inter: 'Inter Variable', 'Inter', ui-sans-serif, system-ui, sans-serif;
  --font-geist-mono: 'Geist Mono', ui-monospace, 'SFMono-Regular', Menlo, Monaco,
    Consolas, monospace;

  /* Reference type roles; desktop body is 14px in the active theme. */
  --text-caption: 10px;
  --leading-caption: 1.6;
  --text-metadata: 12px;
  --text-control: 13px;
  --text-body-sm: 14px;
  --leading-body-sm: 1.5;
  --tracking-body-sm: 0.01em;
  --text-body: 16px;
  --leading-body: 1.5;
  --tracking-body: 0.01em;
  --text-subheading: 18px;
  --leading-subheading: 1.6;
  --text-body-lg: 20px;
  --leading-body-lg: 1.35;
  --text-heading-sm: 32px;
  --leading-heading-sm: 1.13;
  --tracking-heading-sm: -0.02em;
  --text-heading: 36px;
  --leading-heading: 1.17;
  --tracking-heading: -0.02em;
  --text-display: 48px;
  --leading-display: 1.08;
  --tracking-display: -0.02em;
  --font-weight-light: 300;
  --font-weight-regular: 400;
  --font-weight-medium: 500;

  /* Spacing */
  --spacing-unit: 4px;
  --spacing-4: 4px;
  --spacing-8: 8px;
  --spacing-12: 12px;
  --spacing-16: 16px;
  --spacing-20: 20px;
  --spacing-24: 24px;
  --spacing-28: 28px;
  --spacing-32: 32px;
  --spacing-36: 36px;
  --spacing-40: 40px;
  --spacing-48: 48px;
  --spacing-56: 56px;
  --spacing-64: 64px;
  --spacing-72: 72px;
  --spacing-96: 96px;
  --spacing-160: 160px;

  /* Layout: concrete CSS lengths, not prose ranges. */
  --page-max-width: 1280px;
  --section-gap: clamp(96px, 8vw, 125px);
  --workspace-section-gap: 32px;
  --card-padding: 32px;
  --panel-padding: 24px;
  --element-gap: 12px;

  /* Shapes */
  --radius-tags: 9999px;
  --radius-cards: 20px;
  --radius-inputs: 8px;
  --radius-buttons: 9999px;
  --radius-large-cards: 24px;
  --radius-small-elements: 8px;

  /* Elevation */
  --shadow-subtle: 0 0 1px 0 rgba(0, 0, 0, 0.4),
    0 1px 1px 0 rgba(0, 0, 0, 0.04), 0 2px 4px 0 rgba(0, 0, 0, 0.04);
  --shadow-subtle-2: inset 0 0 0 0.5px rgba(0, 0, 0, 0.075);
  --shadow-subtle-3: inset 0 0 0 0.5px rgba(0, 0, 0, 0.1);
  --shadow-subtle-4: inset 0 0 0 1px rgba(0, 0, 0, 0.1);
  --shadow-subtle-5: 0 0 1px 0 rgba(0, 0, 0, 0.4),
    0 2px 4px 0 rgba(0, 0, 0, 0.04);
  --shadow-subtle-6: inset 0 0 0 1px rgba(255, 255, 255, 0.6);
  --shadow-subtle-7: inset 0 0 0 0.5px #ebe8e4;

  /* Reference surfaces */
  --surface-eggshell-canvas: #fdfcfc;
  --surface-warm-taupe: #f5f3f1;
  --surface-stone-plate: #ebe8e4;
}
```

### Tailwind v4

The app already maps its semantic colors through `@theme inline`. Extend that
mapping rather than replacing it. Use fixed spark tokens only in artwork.
The following excerpt shows the reference typography and shape treatment with
the existing desktop color roles.

```css
@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-primary: var(--foreground);
  --color-primary-foreground: var(--background);
  --color-border: var(--border);
  --color-input: var(--control-border);
  --color-ring: var(--secondary-foreground);
  --color-violet-spark: #0447ff;
  --color-ember-orange: #ff4704;

  --font-sans: 'Inter Variable', 'Inter', ui-sans-serif, system-ui, sans-serif;
  --font-heading: 'Waldenburg', 'Inter Variable', 'Inter', ui-sans-serif,
    system-ui, sans-serif;
  --font-mono: 'Geist Mono', ui-monospace, Menlo, Monaco, Consolas, monospace;

  --text-metadata: 12px;
  --text-control: 13px;
  --text-body: 14px;
  --text-body--line-height: 1.5;
  --text-body--letter-spacing: 0.01em;
  --text-heading-sm: 32px;
  --text-heading-sm--line-height: 1.13;
  --text-heading-sm--letter-spacing: -0.02em;
  --text-heading-sm--font-weight: 300;
  --text-heading: 36px;
  --text-heading--line-height: 1.17;
  --text-heading--letter-spacing: -0.02em;
  --text-heading--font-weight: 300;
  --text-display: 48px;
  --text-display--line-height: 1.08;
  --text-display--letter-spacing: -0.02em;
  --text-display--font-weight: 300;

  --radius-input: 8px;
  --radius-panel: 20px;
  --radius-workspace: 24px;
  --radius-full: 9999px;
}
```

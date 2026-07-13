// Shared specimen-sheet building blocks: the palette ribbon (guise's real
// Mantine hues) and the captioned "plate" frame that components sit inside.

// The 14 open-color hues guise ships, at shade 6 — the palette as identity.
export const hues: { name: string; hex: string }[] = [
  { name: "Dark", hex: "#2e2e2e" },
  { name: "Gray", hex: "#868e96" },
  { name: "Red", hex: "#fa5252" },
  { name: "Pink", hex: "#e64980" },
  { name: "Grape", hex: "#be4bdb" },
  { name: "Violet", hex: "#7950f2" },
  { name: "Indigo", hex: "#4c6ef5" },
  { name: "Blue", hex: "#228be6" },
  { name: "Cyan", hex: "#15aabf" },
  { name: "Teal", hex: "#12b886" },
  { name: "Green", hex: "#40c057" },
  { name: "Lime", hex: "#82c91e" },
  { name: "Yellow", hex: "#fab005" },
  { name: "Orange", hex: "#fd7e14" },
];

// A thin band of every hue. `labelled` adds hue names beneath each swatch.
export function ribbon(labelled = false): string {
  const swatches = hues
    .map(
      (h) =>
        `<span class="swatch" style="--c:${h.hex}" title="${h.name} · ${h.hex}">${
          labelled ? `<em>${h.name}</em>` : ""
        }</span>`,
    )
    .join("");
  return `<div class="ribbon${labelled ? " ribbon--labelled" : ""}" aria-hidden="true">${swatches}</div>`;
}

// Ten shades of a single hue — used as a section rule motif.
export function shadeBar(hexes: string[]): string {
  return `<div class="shadebar" aria-hidden="true">${hexes
    .map((c) => `<span style="--c:${c}"></span>`)
    .join("")}</div>`;
}

// Inline Lucide icons (https://lucide.dev, ISC) — guise's built-in icon set,
// inlined here so the site mocks show the same glyphs the library renders.
const lucidepaths: Record<string, string> = {
  check: '<path d="M20 6 9 17l-5-5"/>',
  x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
  star: '<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>',
  play: '<polygon points="6 3 20 12 6 21 6 3"/>',
  settings:
    '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
  leaf: '<path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 4.18 2 8 0 5.5-4.78 10-10 10Z"/><path d="M2 21c0-3 1.85-5.36 5.08-6C9.5 14.52 12 13 13 12"/>',
  heart:
    '<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>',
  zap: '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>',
};

// One inline Lucide glyph, sized to the surrounding text color/box.
export function lucide(name: string, size = 16): string {
  return `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${lucidepaths[name] ?? ""}</svg>`;
}

// A captioned exhibit: a numbered, bordered frame holding a component mock, with
// a monospace caption line (name + note) and a "view docs" link.
export function plate(opts: {
  index: string;
  name: string;
  note: string;
  href: string;
  preview: string;
  wide?: boolean;
}): string {
  return `<figure class="plate${opts.wide ? " plate--wide" : ""}">
  <div class="plate-stage">${opts.preview}</div>
  <figcaption class="plate-cap">
    <span class="plate-idx">${opts.index}</span>
    <span class="plate-name">${opts.name}</span>
    <span class="plate-note">${opts.note}</span>
    <a class="plate-link" href="${opts.href}">docs &#8594;</a>
  </figcaption>
</figure>`;
}

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

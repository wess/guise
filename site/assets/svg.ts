// Generated SVG assets: the favicon mark and the Open Graph social card.
// Kept as code so the gradient/palette stay in sync with the site.

import { hues } from "../render/bits";

export function favicon(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#228be6"/>
      <stop offset="0.5" stop-color="#7950f2"/>
      <stop offset="1" stop-color="#be4bdb"/>
    </linearGradient>
  </defs>
  <rect x="6" y="6" width="52" height="52" rx="16" fill="url(#g)"/>
  <circle cx="32" cy="32" r="11" fill="#0c0c0f"/>
</svg>`;
}

export function ogImage(): string {
  const ribbon = hues
    .map(
      (h, i) =>
        `<rect x="${80 + i * 74}" y="470" width="64" height="64" rx="10" fill="${h.hex}"/>`,
    )
    .join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 630">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#0c0c0f"/>
      <stop offset="1" stop-color="#15101f"/>
    </linearGradient>
    <linearGradient id="word" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#74c0fc"/>
      <stop offset="0.5" stop-color="#b197fc"/>
      <stop offset="1" stop-color="#e599f7"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.3" cy="0.1" r="0.8">
      <stop offset="0" stop-color="#7950f2" stop-opacity="0.45"/>
      <stop offset="1" stop-color="#7950f2" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="1200" height="630" fill="url(#bg)"/>
  <rect width="1200" height="630" fill="url(#glow)"/>
  <g font-family="'Plus Jakarta Sans', sans-serif">
    <g transform="translate(80,96)">
      <rect width="34" height="34" rx="11" fill="url(#word)"/>
      <text x="50" y="26" font-size="30" font-weight="800" fill="#ededf1">guise</text>
    </g>
    <text x="80" y="280" font-size="92" font-weight="800" fill="#ededf1" letter-spacing="-3">Native UI components</text>
    <text x="80" y="380" font-size="92" font-weight="800" letter-spacing="-3">
      <tspan fill="#ededf1">for </tspan><tspan fill="url(#word)">Rust.</tspan>
    </text>
    <text x="80" y="438" font-size="30" fill="#a7a7b4">A Mantine-inspired component library for gpui · ~60 components</text>
  </g>
  ${ribbon}
</svg>`;
}

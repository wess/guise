// Pages that used to live here and now live somewhere else.
//
// Tailor's eight documentation pages were published under this domain from
// 1.1.0 to 1.6.0 and are linked to from outside it. Deleting them when Tailor
// moved out turned every one of those links into a 404, so each old slug keeps
// a stub that forwards to where the page went.
//
// A stub is a `<meta http-equiv="refresh">` plus a `rel="canonical"`: static
// hosting cannot send a real 301, and the canonical link is what stops search
// engines treating the stub as a thin duplicate of the destination.

const TAILOR = "https://github.com/wess/tailor/blob/main/docs";

export const REDIRECTS: Record<string, string> = {
  "tailor.html": `${TAILOR}/readme.md`,
  "tailortutorial.html": `${TAILOR}/tutorial.md`,
  "tailorcanvas.html": `${TAILOR}/canvas.md`,
  "tailorcomponents.html": `${TAILOR}/components.md`,
  "tailorstate.html": `${TAILOR}/state.md`,
  "tailorcodegen.html": `${TAILOR}/codegen.md`,
  "tailormcp.html": `${TAILOR}/mcp.md`,
  "tailorzed.html": `${TAILOR}/zed.md`,
};

export function renderRedirect(to: string): string {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Moved — Tailor documentation</title>
<link rel="canonical" href="${to}">
<meta http-equiv="refresh" content="0; url=${to}">
<meta name="robots" content="noindex">
<style>
  body { font: 15px/1.6 ui-sans-serif, system-ui, sans-serif; margin: 4rem auto; max-width: 34rem; padding: 0 1.5rem; color: #1b1b1f; }
  a { color: #2f5fd0; }
  @media (prefers-color-scheme: dark) { body { background: #16161a; color: #e6e6ea; } a { color: #8fb0ff; } }
</style>
</head>
<body>
<h1>This page moved</h1>
<p>Tailor is its own project now, and its documentation went with it.</p>
<p><a href="${to}">Continue to the page &rarr;</a></p>
</body>
</html>
`;
}

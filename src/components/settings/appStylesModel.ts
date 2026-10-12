import type { AppStyle, WritingStyle } from "@/bindings";

/** An id no style has yet; the backend still renames a repeat. */
export function newStyleId(styles: readonly WritingStyle[]): string {
  let n = styles.length + 1;
  while (styles.some((style) => style.id === `style-${n}`)) n += 1;
  return `style-${n}`;
}

/** The mapping with `app` on `style`: an app already mapped keeps its place. */
export function setAppStyle(
  mappings: readonly AppStyle[],
  app: string,
  style: string,
): AppStyle[] {
  const name = app.trim().toLowerCase();
  if (!name) return [...mappings];
  return mappings.some((m) => m.app === name)
    ? mappings.map((m) => (m.app === name ? { app: name, style } : m))
    : [...mappings, { app: name, style }];
}

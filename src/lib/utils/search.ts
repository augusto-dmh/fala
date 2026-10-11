/** Lowercase without accents, so "itau" finds "Itaú". */
const fold = (text: string) =>
  text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

/** Whether `text` contains `query`, ignoring case and accents; an empty query matches. */
export const matchesQuery = (text: string, query: string) =>
  fold(text).includes(fold(query.trim()));

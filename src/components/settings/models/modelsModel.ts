import type { ModelInfo } from "@/bindings";
import { supportsLanguageCode } from "@/lib/constants/languages";

/** The model transcribes `asrLanguage` ("auto" or unset: any). A model without a language
 *  list is taken as fitting, since nothing says it does not. */
export function fitsLanguage(
  model: Pick<ModelInfo, "supported_languages">,
  asrLanguage: string | null | undefined,
): boolean {
  if (!asrLanguage || asrLanguage === "auto") return true;
  if (model.supported_languages.length === 0) return true;
  return supportsLanguageCode(model.supported_languages, asrLanguage);
}

/** "Recommended" only for a model that transcribes the dictation language. */
export function recommendedFor(
  model: Pick<ModelInfo, "is_recommended" | "supported_languages">,
  asrLanguage: string | null | undefined,
): boolean {
  return model.is_recommended && fitsLanguage(model, asrLanguage);
}

/** The models to download, split by whether they transcribe the dictation language: the rest
 *  goes behind "Other models". */
export function splitByLanguage<
  M extends Pick<ModelInfo, "supported_languages">,
>(
  models: readonly M[],
  asrLanguage: string | null | undefined,
): { fit: M[]; other: M[] } {
  const fit: M[] = [];
  const other: M[] = [];
  for (const model of models) {
    (fitsLanguage(model, asrLanguage) ? fit : other).push(model);
  }
  return { fit, other };
}

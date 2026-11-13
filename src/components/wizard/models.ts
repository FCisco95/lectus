export type LanguageChoice = 'en' | 'pt' | 'both';
export const ACCURATE_MODEL = 'ggml-large-v3-turbo-q5_0.bin';
export function recommendedModel(choice: LanguageChoice): string {
  return choice === 'en' ? 'ggml-base.bin' : 'ggml-small.bin';
}
export function isFast(modelName: string, audioMs: number, elapsedMs: number): boolean {
  if (audioMs <= 0 || elapsedMs <= 0) return false;
  const threshold = modelName === 'ggml-small.bin' ? 15 : modelName === 'ggml-base.bin' ? 30 : Infinity;
  return audioMs / elapsedMs >= threshold;
}

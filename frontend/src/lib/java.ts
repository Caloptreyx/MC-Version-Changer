import type { DockerImage } from './schemas.ts';

/**
 * Picks the egg image to use for `java`: the current image when it already
 * provides that release, otherwise an exact match, otherwise (Java 17+ only)
 * the closest newer release. Returns null when no image fits.
 */
export function recommendedImage(images: DockerImage[], currentImage: string, java: number): DockerImage | null {
  const current = images.find((image) => image.image === currentImage);
  if (current?.javaVersion === java) return current;

  const exact = images.find((image) => image.javaVersion === java);
  if (exact) return exact;
  if (java < 17) return null;

  return (
    images
      .filter((image) => image.javaVersion !== null && image.javaVersion > java)
      .sort((a, b) => (a.javaVersion ?? 0) - (b.javaVersion ?? 0))[0] ?? null
  );
}

/** Whether an image providing `provided` cannot run a version that needs `required`. */
export function javaMismatch(required: number, provided: number): boolean {
  // Java 8 era servers (Forge especially) break on newer runtimes; everything later runs on newer ones.
  return required <= 8 ? provided !== required : provided < required;
}

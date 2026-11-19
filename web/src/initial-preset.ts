export const defaultWebPreset = "green-founders";
/** Preset used when no `?preset=` is given (decided 2026-11-19, Issue #11). */
export const startupWebPreset = "stage4-standard";

export function greenFounderPreset<T extends {
  plants: object;
  herbivores: object;
  toxin: object;
  carnivores: object;
}>(standard: T) {
  return {
    ...standard,
    carnivores: { ...standard.carnivores, speed: 5.25 },
    plants: {
      ...standard.plants,
      color_inheritance_enabled: true,
      color_mutation_probability: 0.20,
      color_mutation_scale: 0.25,
    },
    herbivores: { ...standard.herbivores, color_learning_enabled: true },
    toxin: {
      ...standard.toxin,
      enabled: true,
      plant_variation_enabled: true,
      initial_toxic_plant_fraction: 0,
      plant_toxic_mutation_probability: 0.05,
      experimental_plant_toxicity_colors: false,
      experimental_inherited_plant_colors: false,
    },
  };
}

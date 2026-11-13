import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { formatSettingError, settingControlKeys, settingDisplay } from "../web/src/settings-display.ts";

const config = readFileSync(new URL("../crates/sim-core/src/config.rs", import.meta.url), "utf8");
const sections = Object.fromEntries(
  [...config.matchAll(/pub struct (\w+) \{([\s\S]*?)\n\}/g)].map(([, name, fields]) => [name, fields]),
);
const schema = [...sections.SimulationConfig.matchAll(/pub (\w+): (\w+),/g)];
const scalars = /pub (\w+): (?:bool|u8|u16|u32|u64|usize|f32|f64|Option<f64>),/g;
const keys = schema.flatMap(([, key, type]) =>
  sections[type] ? [...sections[type].matchAll(scalars)].map(([, field]) => key + "." + field)
    : key === "seed" ? [key] : [],
);
for (const key of [...keys, ...Object.values(settingControlKeys)]) {
  const display = settingDisplay[key];
  assert.ok(display, key);
  assert.match(display.name, /[ぁ-んァ-ン一-龯]/, key);
  assert.match(display.description, /[ぁ-んァ-ン一-龯]/, key);
  assert.ok(display.range, key);
}
assert.match(settingDisplay["toxin.initial_type0_fraction"].range, /0\.05＝5%/);
const dependency = formatSettingError("carnivore color learning requires animal toxin");
assert.match(dependency, /肉食の色学習（animal_toxin\.color_learning_enabled）/);
assert.match(dependency, /動物毒をON/);
assert.match(formatSettingError("unmapped validator message"), /原文 unmapped validator message/);
console.log(`Display metadata verified for ${keys.length} Rust configuration fields and ${Object.keys(settingControlKeys).length} existing Web controls.`);

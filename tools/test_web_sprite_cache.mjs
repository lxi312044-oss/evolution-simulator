import assert from "node:assert/strict";
import { sprite } from "../web/src/sprites.ts";

let generated = 0;
globalThis.document = {
  createElement() {
    generated++;
    return { getContext: () => ({ fillRect() {} }) };
  },
};
const rgb = i => [(i & 255) / 255, ((i >> 8) & 255) / 255, 0];
const oldest = sprite("plant-young", rgb(0));
const retained = sprite("plant-young", rgb(1));
for (let i = 2; i < 8192; i++) sprite("plant-young", rgb(i));
assert.equal(sprite("plant-young", rgb(1)), retained);
sprite("plant-young", rgb(8192));
assert.equal(generated, 8193);
assert.equal(sprite("plant-young", rgb(1)), retained);
assert.notEqual(sprite("plant-young", rgb(0)), oldest);
for (let i = 8193; i < 20000; i++) {
  sprite("plant-young", rgb(i));
  assert.equal(sprite("plant-young", rgb(1)), retained);
}
assert.equal(generated, 20001);
console.log("Sprite cache evicts old variants while retaining frequently used images.");

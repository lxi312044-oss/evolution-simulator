/** Phase 06.1: pixel-art presentation only. No simulation state or RNG changes. */
export type SpeciesSprite = "herbivore" | "carnivore" | "plant-seedling" | "plant-young" | "plant-mature";
const palette: Record<string,string> = {
  '.':'', 'o':'#493a32', 'b':'#b78b59', 'l':'#e3c39a', 'w':'#fff2db',
  'g':'#4ecb70', 's':'#2f8751', 'h':'#a5e69b', 'e':'#171d24', 'p':'#e99c9c', 'f':'#df753e', 'd':'#a84d32', 't':'#f8d7a6'
};
// Fixed 16x16 designs. Herbivore: long ears; carnivore: pointed ears and tail.
const patterns: Record<SpeciesSprite,string[]> = {
 'plant-seedling':[
 '................','................','................','................',
 '................','................','................','................',
 '................','.....gg..gg.....','......gggg......','.......ss.......',
 '.......ss.......','.......ss.......','......ssss......','................'],
 'plant-young':[
 '................','................','................','................',
 '.....gg..gg.....','....gghgghgg....','.....gggggg.....','......ssss......',
 '..gg...ss...gg..','..gghg.ss.gghg..','...gggssgggg....','......ssss......',
 '......ssss......','.......ss.......','......ssss......','................'],
 'plant-mature':[
 '................','................','....ggg..ggg....','...gghgggghgg...',
 '..gghhggghhggg..','..gghggggghggg..','...gggsssgggg...','..gggssssgggg...',
 '.gghggsssgghgg..','.gghggsssgghgg..','..gggssssgggg...','....gssssg......',
 '......ssss......','......ssss......','.....ssssss.....','................'],
 herbivore:[
 '....oo....oo....', '....ob....bo....', '....ob....bo....',
 '....ob....bo....', '....oboooobo....',
 '....obbbbbbo....', '...obbbbbbbbo...', '...obewbbwebbo..',
 '...obbbbbbbbo...', '....obpbbbo.....', '....obbbbbo.....',
 '...obbbbbbbbo...', '..obbbbbbbbbbo..', '..obbbbbbbbbbo..',
 '...ooobbbbooo...', '.....oooooo.....'
 ],
 carnivore:[
 '..oo........oo..', '..offo....offo..', '...offo..offo...',
 '...offfffffo....', '..offfffffffo...', '..offfffffffo...',
 '..offeffefffo...', '..offfffffffo...', '..offftttfffo...',
 '...offteffo.....', '....offfffo.....', '...offfffffo.oo.',
 '..offfffffffoodo', '..offfffffffoodo', '...oooddooooodo.',
 '.....oooo..ooo..'
 ]
};
// Retain enough variants for a dense view of the default 5,000-entity world.
const maxCachedSprites = 8192;
const cache = new Map<string, HTMLCanvasElement>();
/** Color channels are sRGB floats in [0,1], matching Rust BodyColor. */
export type BodyRGB = readonly [number,number,number];
function hexToRgb(hex:string):[number,number,number] {
 return [parseInt(hex.slice(1,3),16),parseInt(hex.slice(3,5),16),parseInt(hex.slice(5,7),16)];
}
function toHex(rgb:readonly number[]):string {return `#${rgb.map(v=>Math.round(Math.max(0,Math.min(255,v))).toString(16).padStart(2,'0')).join('')}`;}
function recolor(base:string,color:BodyRGB,shade:number):string {
 const reference=hexToRgb(base);
 const luminosity=(reference[0]+reference[1]+reference[2])/3/255;
 const factor=shade*luminosity;
 return toHex(color.map(v=>v*255*factor));
}

export function sprite(species:SpeciesSprite,bodyColor?:BodyRGB):HTMLCanvasElement {
 const rgb=bodyColor?.map(v=>Math.max(0,Math.min(255,Math.round(v*255))));
 const key=species+(rgb?`:${rgb.join(',')}`:'');
 const cached=cache.get(key);if(cached){
  cache.delete(key);
  cache.set(key,cached);
  return cached;
 }
 const rows=patterns[species];
 const c=document.createElement('canvas');c.width=16;c.height=16;
 const context=c.getContext('2d')!;
 rows.forEach((row,y)=>{
  if(row.length!==16)throw new Error(`Invalid ${species} sprite row ${y}: ${row.length}`);
  for(let x=0;x<16;x++){
   const symbol=row[x];
   let color=palette[symbol];
   if(bodyColor){
    // Only fur/main body/secondary shade are recolored; eyes, outline, nose stay intact.
    if(species==='herbivore' && symbol==='b')color=toHex(bodyColor.map(v=>v*255));
    if(species==='herbivore' && symbol==='l')color=recolor('#b78b59',bodyColor,1.25);
    if(species==='carnivore' && symbol==='f')color=toHex(bodyColor.map(v=>v*255));
    if(species==='carnivore' && symbol==='d')color=recolor('#df753e',bodyColor,0.72);
    if(species.startsWith('plant-') && symbol==='g')color=toHex(bodyColor.map(v=>v*255));
    if(species.startsWith('plant-') && symbol==='s')color=recolor('#4ecb70',bodyColor,0.68);
    if(species.startsWith('plant-') && symbol==='h')color=recolor('#4ecb70',bodyColor,1.3);
   }
   if(color){context.fillStyle=color;context.fillRect(x,y,1,1);}
  }
 });
 // Evict only the least recently used image.
 if(cache.size>=maxCachedSprites){
  const oldest=cache.keys().next();
  if(!oldest.done)cache.delete(oldest.value);
 }
 cache.set(key,c);return c;
}
export function drawAnimalSprite(ctx:CanvasRenderingContext2D,species:SpeciesSprite,x:number,y:number,size:number,flip=false,bodyColor?:BodyRGB):void {
 const img=sprite(species,bodyColor);const pixels=Math.max(12,Math.round(size/2)*2);
 ctx.save();ctx.imageSmoothingEnabled=false;ctx.translate(Math.round(x),Math.round(y));
 if(flip)ctx.scale(-1,1);
 ctx.drawImage(img,-pixels/2,-pixels/2,pixels,pixels);
 ctx.restore();
}

/** Growth-stage plant sprite. Uses the visible RGB stored on the plant. */
export function drawPlantSprite(ctx:CanvasRenderingContext2D,x:number,y:number,growth:number,size:number,bodyColor?:BodyRGB):void {
 const kind:SpeciesSprite=growth<0.33?'plant-seedling':growth<0.85?'plant-young':'plant-mature';
 // Issue #70: with color mutation nearly every plant has its own color; cache sprites per
 // quantized color (16 levels per channel) so thousands of plants share a few hundred images.
 const img=sprite(kind,bodyColor&&quantize(bodyColor,15));
 ctx.save();ctx.imageSmoothingEnabled=false;
 const px=Math.round(size);ctx.drawImage(img,Math.round(x-px/2),Math.round(y-px/2),px,px);
 ctx.restore();
}

/** Issue #64: display-only creature sprites generated from genes. Predators still see color only. */
export type AnimalKind = "herbivore" | "carnivore";
export interface CreatureShape {
 /** 0..1 body size (herbivore: adult max HP). */ size: number;
 /** 0..1 speed (wider body, longer legs). */ speed: number;
 /** 0..3 back spikes from the strongest toxin production ability. */ spikes: number;
 /** Thick pale rim when toxin resistance >= 0.6. */ shell: boolean;
}
const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** `traits` is one row of `animal_shape_snapshot_json`: [max HP, speed, toxin ability, resistance]. */
export function creatureShape(kind: AnimalKind, traits?: readonly number[]): CreatureShape {
 if (!traits) return { size: kind === "carnivore" ? 1 : 0.5, speed: 0.5, spikes: 0, shell: false };
 const [hp, speed, ability, resistance] = traits;
 return {
  size: kind === "carnivore" ? 1 : clamp01((hp - 45) / 30),
  speed: kind === "carnivore" ? clamp01((speed - 4) / 3) : clamp01((speed - 1) / 3.5),
  spikes: ability < 0.35 ? 0 : ability < 0.6 ? 1 : ability < 0.85 ? 2 : 3,
  shell: resistance >= 0.6,
 };
}
const mixRgb = (rgb: readonly number[], to: number, t: number) => toHex(rgb.map(v => v * 255 * (1 - t) + to * t));
const shadeRgb = (rgb: readonly number[], f: number) => toHex(rgb.map(v => v * 255 * f));

/** 16x16 pixel grid for a creature; letters are palette keys resolved in `creatureSprite`. */
export function creaturePixels(kind: AnimalKind, shape: CreatureShape): string[][] {
 const g: string[][] = Array.from({ length: 16 }, () => Array(16).fill("."));
 const size = Math.round(shape.size * 3) / 3, speed = Math.round(shape.speed * 2) / 2;
 const legs = 1 + Math.round(speed * 2);
 const rx = Math.min(7.3, (4.4 + 1.3 * size) * (1 + 0.18 * speed));
 const ry = (4.0 + 1.0 * size) * (1 - 0.1 * speed);
 const cx = 8, bottom = 15 - legs, cy = bottom - ry + 0.3;
 const inside = (x: number, y: number) => x >= 0 && y >= 0 && x < 16 && y < 16
  && ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1;
 for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
  if (!inside(x, y)) continue;
  const edge = [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => !inside(x + dx, y + dy));
  const rim = [[2, 0], [-2, 0], [0, 2], [0, -2], [1, 1], [-1, -1], [1, -1], [-1, 1]].some(([dx, dy]) => !inside(x + dx, y + dy));
  const nx = (x + 0.5 - cx) / rx, ny = (y + 0.5 - cy) / ry;
  g[y][x] = edge ? "o" : shape.shell && rim ? "s" : nx < -0.15 && ny < -0.25 && nx * nx + ny * ny > 0.2 ? "h" : ny > 0.55 ? "d" : "b";
 }
 const top = (x: number) => { for (let y = 0; y < 16; y++) if (g[y][x] !== ".") return y; return 16; };
 // Back spikes = toxin production ability (observer aid, not seen by predators).
 const spikeXs = [[], [8], [5, 10], [4, 8, 11]][shape.spikes] ?? [];
 for (const x of spikeXs) {
  const t = top(x);
  for (const dx of [-1, 0, 1]) if (t - 1 >= 0 && g[t - 1][x + dx] === ".") g[t - 1][x + dx] = "k";
  if (t - 2 >= 0) g[t - 2][x] = "k";
  if (t - 3 >= 0) g[t - 3][x] = "k";
 }
 // Legs: longer for faster creatures.
 const lx = Math.round(cx - rx * 0.5) - 1, rxp = 15 - lx;
 for (const x of [lx, rxp]) for (let y = bottom + 1; y <= Math.min(15, bottom + legs); y++) g[y][x] = "o";
 if (legs >= 2) { g[Math.min(15, bottom + legs)][lx - 1] = "o"; g[Math.min(15, bottom + legs)][rxp + 1] = "o"; }
 const d = Math.max(2, Math.round(rx * 0.38));
 const left = 8 - d, right = 7 + d, ey = Math.round(cy - ry * 0.2);
 if (kind === "carnivore") {
  // Horns: pointed tips on both upper sides change the silhouette.
  for (const [x, dir] of [[Math.round(cx - rx * 0.62) - 1, -1], [Math.round(cx + rx * 0.62), 1]] as const) {
   const t = top(x); if (t - 1 >= 0) g[t - 1][x] = "o"; if (t - 2 >= 0 && x + dir >= 0 && x + dir < 16) g[t - 2][x + dir] = "o";
  }
  g[ey + 1][left] = "e"; g[ey + 1][right] = "e";
  g[ey][left] = "o"; g[ey - 1][left - 1] = "o"; g[ey][right] = "o"; g[ey - 1][right + 1] = "o";
  const my = Math.min(bottom - 1, ey + 3);
  for (let x = left; x <= right; x++) g[my][x] = "e";
  if (my + 1 <= bottom - 1) { g[my + 1][left] = "w"; g[my + 1][right] = "w"; }
 } else {
  g[ey][left] = "e"; g[ey + 1][left] = "e"; g[ey][right] = "e"; g[ey + 1][right] = "e";
  const my = Math.min(bottom - 1, ey + 3);
  g[my][7] = "o"; g[my][8] = "o";
 }
 return g;
}

const creatureCache = new Map<string, HTMLCanvasElement>();
export function creatureSprite(kind: AnimalKind, color: BodyRGB, shape: CreatureShape): HTMLCanvasElement {
 const bodyColor = quantize(color, 31);
 const rgb = bodyColor.map(v => Math.max(0, Math.min(255, Math.round(v * 255))));
 const key = [kind, rgb.join(","), Math.round(shape.size * 3), Math.round(shape.speed * 2), shape.spikes, shape.shell ? 1 : 0].join(":");
 const cached = creatureCache.get(key);
 if (cached) { creatureCache.delete(key); creatureCache.set(key, cached); return cached; }
 const c = document.createElement("canvas"); c.width = 16; c.height = 16;
 const ctx = c.getContext("2d")!;
 const colors: Record<string, string> = {
  b: toHex(bodyColor.map(v => v * 255)), d: shadeRgb(bodyColor, 0.78), h: mixRgb(bodyColor, 255, 0.4),
  s: mixRgb(bodyColor, 235, 0.65), o: shadeRgb(bodyColor, 0.3), k: mixRgb(bodyColor, 255, 0.55), e: "#141a20", w: "#ffffff",
 };
 creaturePixels(kind, shape).forEach((row, y) => row.forEach((p, x) => {
  if (p !== ".") { ctx.fillStyle = colors[p]; ctx.fillRect(x, y, 1, 1); }
 }));
 if (creatureCache.size >= maxCachedSprites) { const oldest = creatureCache.keys().next(); if (!oldest.done) creatureCache.delete(oldest.value); }
 creatureCache.set(key, c);
 return c;
}
export function drawCreature(ctx: CanvasRenderingContext2D, kind: AnimalKind, x: number, y: number, size: number,
 bodyColor: BodyRGB, shape: CreatureShape): void {
 const img = creatureSprite(kind, bodyColor, shape); const pixels = Math.max(16, Math.round(size / 2) * 2);
 ctx.save(); ctx.imageSmoothingEnabled = false;
 ctx.drawImage(img, Math.round(x - pixels / 2), Math.round(y - pixels / 2), pixels, pixels);
 ctx.restore();
}

/** Display-only color rounding for sprite caching (exact colors stay in the inspector). */
export function quantize(color: BodyRGB, levels: number): BodyRGB {
 return color.map(v => Math.round(Math.max(0, Math.min(1, v)) * levels) / levels) as unknown as BodyRGB;
}

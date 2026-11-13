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
 const img=sprite(kind,bodyColor);
 ctx.save();ctx.imageSmoothingEnabled=false;
 const px=Math.round(size);ctx.drawImage(img,Math.round(x-px/2),Math.round(y-px/2),px,px);
 ctx.restore();
}

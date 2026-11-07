type FieldConfig = {
 world: {width:number;height:number;land_columns:number;land_rows:number;max_living_entities:number};
 initial_population: {plants:number;herbivores:number;carnivores:number};
};
export function resizeField<T extends FieldConfig>(source:T,size:number|null,scalePopulation:boolean):T {
 const next=structuredClone(source);
 if(size===null)return next;
 if(![20,30,40].includes(size))throw new Error("フィールドの広さを選択してください");
 const ratio=size*size/(source.world.land_columns*source.world.land_rows);
 next.world.width=source.world.width/source.world.land_columns*size;
 next.world.height=source.world.height/source.world.land_rows*size;
 next.world.land_columns=size;next.world.land_rows=size;
 if(scalePopulation)for(const key of ["plants","herbivores","carnivores"] as const)next.initial_population[key]=Math.round(source.initial_population[key]*ratio);
 if(Object.values(next.initial_population).reduce((sum,n)=>sum+n,0)>next.world.max_living_entities)throw new Error("初期個体数が世界の上限を超えます。初期密度の維持をOFFにするか、個体数を減らしてください");
 return next;
}

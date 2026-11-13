type Values = Record<string, unknown>;
const groups: [string, string[], string][] = [
  ["環境・初期集団", ["seed", "world", "initial_population", "simulation"], "初期配置はseedに従うコード内の手順です。"],
  ["植物・色", ["plants", "toxin.plant_", "toxin.initial_toxic_", "toxin.initial_type", "toxin.experimental_"],
    "固定配色モードは相互排他です。植物RGBの独立変異率や自由な色配置は未実装です。"],
  ["毒・耐性", ["toxin", "animal_toxin"],
    "動物の生成上限・生成費と身体N割合に基づく移送はコード内固定です。"],
  ["学習・判断", ["herbivores.color_learning", "herbivores.hungry_color", "animal_toxin.color_learning"],
    "記憶は最大30色、一般化距離0.25、空腹時の許容式はコード内固定です。時間経過による忘却は未実装です。"],
  ["摂食・繁殖", ["herbivores", "carnivores"], ""],
];
const optional = ["animal_toxin.carnivore_damage_threshold", "animal_toxin.carnivore_hp_damage_per_excess_unit"];
const labels: Record<string, string> = {
  seed: "乱数seed", "world.width": "世界の幅", "world.height": "世界の高さ",
  "world.land_columns": "土壌の列数", "world.land_rows": "土壌の行数",
  "world.max_living_entities": "全種合計の個体数上限",
  "initial_population.plants": "初期植物数", "initial_population.herbivores": "初期草食数",
  "initial_population.carnivores": "初期肉食数",
  "herbivores.color_learning_enabled": "草食の色学習", "animal_toxin.color_learning_enabled": "肉食の色学習",
  "animal_toxin.carnivore_damage_threshold": "肉食の毒損傷閾値",
  "animal_toxin.carnivore_hp_damage_per_excess_unit": "肉食の毒損傷係数（HP／超過毒量／Tick）",
};
const matches = (path: string, parts: string[]) => parts.some(part => path === part || path.startsWith(part + ".")
  || part.includes(".") && path.startsWith(part));

export function editor(container: HTMLElement, dirty: () => void) {
  const state = {
    onApply: (_value: Values) => {},
    load(value: Values) {
      const draft = structuredClone(value);
      if (!draft.animal_toxin) draft.animal_toxin = {enabled:false,color_learning_enabled:false};
      const toxin = draft.toxin as Values;
      const herbivores = draft.herbivores as Values;
      for (const key of ["experimental_plant_toxicity_colors", "experimental_inherited_plant_colors"]) toxin[key] ??= false;
      for (const key of ["hungry_color_recheck_enabled", "hungry_color_recovery_enabled"]) herbivores[key] ??= false;
      const lists = groups.map(() => document.createElement("div"));
      lists.forEach(list => list.className = "tuning-grid");
      const entries = Object.entries(draft).flatMap(([key,val]) =>
        val && typeof val === "object" && !Array.isArray(val)
          ? Object.entries(val).map(([prop,v]) => [key+"."+prop,v] as const) : [[key,val] as const]);
      for (const path of optional) if (!entries.some(([key]) => key === path)) entries.push([path, null]);
      const updates: (() => void)[] = [];
      for (const [path,val] of entries) {
        if (typeof val !== "number" && typeof val !== "boolean" && !optional.includes(path)) continue;
        const index = matches(path, groups[3][1]) ? 3 : groups.findIndex(([,parts]) => matches(path, parts));
        if (index < 0) continue;
        const label = document.createElement("label");
        label.textContent = labels[path] ?? path;
        label.title = path;
        const input = document.createElement("input");
        input.type = typeof val === "boolean" ? "checkbox" : "number";
        if (typeof val === "boolean") input.checked = val;
        else {
          input.step = "any";
          input.value = val == null ? "" : String(val);
          input.required = !optional.includes(path);
          if (!input.required) input.placeholder = "共有設定を使用";
        }
        input.addEventListener("input", dirty);
        label.append(input);
        lists[index].append(label);
        updates.push(() => {
          if (!input.reportValidity()) throw new Error(path + " の値を確認してください");
          const [section, key] = path.split(".");
          const target = key ? draft[section] as Values : draft;
          const name = key ?? section;
          if (input.type === "checkbox") target[name] = input.checked;
          else if (!input.value && optional.includes(path)) delete target[name];
          else {
            const number = input.valueAsNumber;
            if (!Number.isFinite(number) || (path === "seed" && (!Number.isSafeInteger(number) || number < 0)))
              throw new Error(path + " の値を確認してください");
            target[name] = number;
          }
        });
      }
      const sections = groups.map(([title,,note], i) => {
        const details = document.createElement("details");
        const summary = document.createElement("summary");
        summary.textContent = title;
        details.append(summary);
        if (note) {
          const p = document.createElement("p");
          p.className = "note"; p.textContent = note; details.append(p);
        }
        details.append(lists[i]);
        return details;
      });
      const apply = document.createElement("button");
      apply.textContent = "適用して新規開始";
      apply.onclick = () => {
        try { updates.forEach(update => update()); state.onApply(draft); }
        catch (e) { document.getElementById("status")!.textContent = String(e); }
      };
      container.replaceChildren(...sections, apply);
    },
  };
  return state;
}

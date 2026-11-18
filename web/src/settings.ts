import { formatSettingError, settingControlKeys, settingDisplay } from "./settings-display";
type Values = Record<string, unknown>;
const groups: [string, string[], string][] = [
  ["環境・初期集団", ["seed", "world", "initial_population", "simulation"], "初期配置はseedに従うコード内の手順です。"],
  ["植物・色", ["plants", "toxin.plant_", "toxin.initial_toxic_", "toxin.initial_type", "toxin.experimental_"],
    "固定配色モードは相互排他です。通常の色継承・変異と実験配色は併用できません。"],
  ["毒・耐性", ["toxin", "animal_toxin"],
    "toxin.enabled は植物毒の作用と草食の被毒処理を切り替えます。植物の配色・継承と動物毒・耐性の設定は個別です。動物の生成上限・生成費と身体N割合に基づく移送はコード内固定です。"],
  ["学習・判断", ["herbivores.color_learning", "herbivores.hungry_color", "animal_toxin.color_learning"],
    "記憶は最大30色、一般化距離0.25、空腹時の許容式はコード内固定です。時間経過による忘却は未実装です。"],
  ["摂食・繁殖", ["herbivores", "carnivores"], ""],
];
const optional = ["animal_toxin.carnivore_damage_threshold", "animal_toxin.carnivore_hp_damage_per_excess_unit"];
const matches = (path: string, parts: string[]) => parts.some(part => path === part || path.startsWith(part + ".")
  || part.includes(".") && path.startsWith(part));

export function editor(container: HTMLElement, dirty: () => void) {
  const state = {
    onApply: (_value: Values, _changed: boolean) => {},
    load(value: Values) {
      const draft = structuredClone(value);
      if (!draft.animal_toxin) draft.animal_toxin = {enabled:false,color_learning_enabled:false};
      const plants = draft.plants as Values;
      plants.color_inheritance_enabled ??= false;
      plants.color_mutation_probability ??= 0;
      plants.color_mutation_scale ??= 0;
      (draft.carnivores as Values).reproduction_parent_reserve_energy ??= 0;
      (draft.carnivores as Values).speed_evolution_enabled ??= false;
      for (const key of ["initial_speed_spread", "speed_mutation_probability", "speed_mutation_scale"]) (draft.carnivores as Values)[key] ??= 0;
      const toxin = draft.toxin as Values;
      const herbivores = draft.herbivores as Values;
      for (const key of ["experimental_plant_toxicity_colors", "experimental_inherited_plant_colors"]) toxin[key] ??= false;
      for (const key of ["hungry_color_recheck_enabled", "hungry_color_recovery_enabled"]) herbivores[key] ??= false;
      for (const key of ["color_memory_safe_meal_relief", "color_memory_decay_per_tick"]) herbivores[key] ??= 0;
      const original = JSON.stringify(draft);
      const lists = [...groups, ["その他"]].map(() => document.createElement("div"));
      lists.forEach(list => list.className = "tuning-grid");
      const entries = Object.entries(draft).flatMap(([key,val]) =>
        val && typeof val === "object" && !Array.isArray(val)
          ? Object.entries(val).map(([prop,v]) => [key+"."+prop,v] as const) : [[key,val] as const]);
      for (const path of optional) if (!entries.some(([key]) => key === path)) entries.push([path, null]);
      const updates: (() => void)[] = [];
      for (const [path,val] of entries) {
        if (typeof val !== "number" && typeof val !== "boolean" && !optional.includes(path)) continue;
        const index = matches(path, groups[3][1]) ? 3 : groups.findIndex(([,parts]) => matches(path, parts));
        const display = settingDisplay[path];
        const label = document.createElement("label");
        const name = document.createElement("span");
        name.className = "setting-name";
        name.textContent = display?.name ?? path;
        const help = document.createElement("span");
        help.className = "setting-help";
        help.textContent = display?.description ?? "説明未登録";
        const range = document.createElement("span");
        range.className = "setting-range";
        range.textContent = display?.range ?? "入力範囲は設定検証に従います";
        const keyLabel = document.createElement("code");
        keyLabel.textContent = path;
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
        input.readOnly = path === "simulation.decision_interval_ticks";
        input.addEventListener("input", dirty);
        label.className = "setting-card";
        input.setAttribute("aria-label", display?.name ?? path);
        label.append(name, input, help, range, keyLabel);
        lists[index < 0 ? groups.length : index].append(label);
        updates.push(() => {
          if (!input.reportValidity()) throw new Error((display?.name ?? path) + "（" + path + "）を " + (display?.range ?? "有効な範囲") + " で入力してください。");
          const [section, key] = path.split(".");
          const target = key ? draft[section] as Values : draft;
          const name = key ?? section;
          if (input.type === "checkbox") target[name] = input.checked;
          else if (!input.value && optional.includes(path)) delete target[name];
          else {
            const number = input.valueAsNumber;
            if (!Number.isFinite(number) || (path === "seed" && (!Number.isSafeInteger(number) || number < 0)))
              throw new Error((display?.name ?? path) + "（" + path + "）を " + (display?.range ?? "有効な範囲") + " で入力してください。");
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
      if (lists[groups.length].childElementCount) {
        const extra = document.createElement("details");
        const title = document.createElement("summary");
        title.textContent = "その他の設定";
        extra.append(title, lists[groups.length]);
        sections.push(extra);
      }
      const apply = document.createElement("button");
      apply.textContent = "適用して新規開始";
      apply.onclick = () => {
        try { updates.forEach(update => update()); state.onApply(draft, JSON.stringify(draft) !== original); }
        catch (e) { document.getElementById("settings-error")!.textContent = formatSettingError(e); }
      };
      container.replaceChildren(...sections, apply);
    },
  };
  return state;
}

export function describeExistingControls() {
  for (const [id, key] of Object.entries(settingControlKeys)) {
    const control = document.getElementById(id) as HTMLInputElement | HTMLSelectElement;
    const label = control.closest("label")!;
    const display = settingDisplay[key];
    const name = document.createElement("span");
    name.className = "setting-name";
    name.textContent = display.name;
    const help = document.createElement("span");
    help.className = "setting-help";
    help.textContent = display.description;
    const range = document.createElement("span");
    range.className = "setting-range";
    const bounds = control instanceof HTMLInputElement && control.type === "number" && control.min && control.max;
    range.textContent = display.range + (bounds ? "（簡易調整では " + control.min + "〜" + control.max + "）" : "");
    const code = document.createElement("code");
    code.textContent = key;
    control.setAttribute("aria-label", display.name);
    label.className = "setting-card";
    label.replaceChildren(name, control, help, range, code);
  }
}

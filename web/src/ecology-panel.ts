// Issue #66 / #67: species ranking and event log (left panel).
type Id = [number, number];
type Kind = "plant" | "herbivore" | "carnivore";

interface SpeciesEntry {
  id: number; rank: number; label: string | null; name: string; count: number; share: number;
  rgb: [number, number, number]; age_ticks: number;
}
interface KindSummary { total: number; living_species: number; extinct_species: number; species: SpeciesEntry[] }
interface Summary { tick: number; kinds: Record<Kind, KindSummary> }
interface SimEvent { tick: number; code: string; message: string; entity: Id | null; species: number | null }

const KIND_LABELS: Record<Kind, string> = { plant: "植物", herbivore: "草食", carnivore: "肉食" };
const SHOWN = 8;
const MAX_LOG = 200;
const EVENT_ICON: Record<string, string> = {
  carnivore_from_herbivore: "🦊", new_species: "🌱", toxic_plant: "☠", toxic_animal: "☠",
  resistant_animal: "🛡", dominance: "👑", plants_declining: "📉", species_extinct: "✝",
  extinction_accelerating: "⚠", kind_gone: "⚠", kind_back: "✨",
};

export class EcologyPanel {
  private kind: Kind = "herbivore";
  private summary?: Summary;
  private events: SimEvent[] = [];
  private notice?: { element: HTMLElement; timer: number };
  private cursor = new Map<number, number>();

  constructor(private ranking: HTMLElement, private tabs: HTMLElement, private totals: HTMLElement,
    private log: HTMLElement, private focus: (id: Id) => boolean,
    private member: (species: number, n: number) => Id | null) {
    for (const button of tabs.querySelectorAll<HTMLButtonElement>("button[data-kind]")) {
      button.onclick = () => { this.kind = button.dataset.kind as Kind; this.drawRanking(); };
    }
    this.drawLog();
  }

  reset() {
    this.summary = undefined;
    this.events = [];
    this.cursor.clear();
    this.drawRanking();
    this.drawLog();
  }

  updateSpecies(json: string) {
    const next = JSON.parse(json) as Summary;
    // The summary only changes at the census; redrawing less often keeps rows clickable.
    if (this.summary && next.tick - next.tick % 100 === this.summary.tick - this.summary.tick % 100
      && next.tick >= this.summary.tick) return;
    this.summary = next;
    this.drawRanking();
  }

  addEvents(json: string) {
    const fresh = JSON.parse(json) as SimEvent[];
    if (!fresh.length) return;
    this.events.push(...fresh);
    if (this.events.length > MAX_LOG) this.events.splice(0, this.events.length - MAX_LOG);
    this.drawLog();
  }

  private drawRanking() {
    for (const button of this.tabs.querySelectorAll<HTMLButtonElement>("button[data-kind]"))
      button.setAttribute("aria-pressed", String(button.dataset.kind === this.kind));
    const data = this.summary?.kinds[this.kind];
    if (!data) {
      this.totals.textContent = "";
      this.ranking.replaceChildren(empty("Tickを進めると種の集計が表示されます（100 Tickごと）。"));
      return;
    }
    this.totals.textContent = `生存 ${data.living_species}種 · 絶滅 ${data.extinct_species}種 · ${data.total}個体`;
    if (!data.species.length) {
      this.ranking.replaceChildren(empty(KIND_LABELS[this.kind] + "は現在いません。"));
      return;
    }
    const rows = data.species.slice(0, SHOWN).map(s => {
      const row = document.createElement("li");
      row.className = "species-row" + (s.rank <= 3 ? " top" : "");
      const badge = document.createElement("span");
      badge.className = "rank-badge rank-" + s.rank;
      badge.textContent = s.label ?? s.rank + "位";
      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.style.backgroundColor = "rgb(" + s.rgb.map(v => Math.round(v * 255)).join(",") + ")";
      const name = document.createElement("span");
      name.className = "species-name";
      name.textContent = s.name;
      name.title = `${s.name}（出現から ${s.age_ticks} Tick）`;
      const count = document.createElement("span");
      count.className = "species-count";
      count.textContent = `${s.count} · ${(s.share * 100).toFixed(0)}%`;
      const bar = document.createElement("span");
      bar.className = "share-bar";
      const fill = document.createElement("span");
      fill.style.width = (s.share * 100).toFixed(1) + "%";
      fill.style.backgroundColor = swatch.style.backgroundColor;
      bar.append(fill);
      row.append(badge, swatch, name, count, bar);
      row.tabIndex = 0;
      row.title = "クリックでこの種の個体にフォーカス（押すたびに次の個体へ）";
      const go = () => this.focusSpecies(row, s.id);
      // pointerdown: the list is redrawn while playing, so a full click may span two rows.
      row.onpointerdown = e => { if (e.button === 0) go(); };
      row.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(); } };
      return row;
    });
    const rest = data.species.length - SHOWN;
    if (rest > 0) rows.push(empty(`ほか ${rest} 種`));
    this.ranking.replaceChildren(...rows);
  }

  private drawLog() {
    if (!this.events.length) {
      this.log.replaceChildren(empty("まだイベントはありません。"));
      return;
    }
    const items = this.events.slice().reverse().map(e => {
      const item = document.createElement("li");
      item.className = "event event-" + e.code;
      const tick = document.createElement("span");
      tick.className = "event-tick";
      tick.textContent = "Tick " + e.tick;
      const text = document.createElement(e.entity ? "button" : "span");
      text.className = "event-text";
      text.textContent = (EVENT_ICON[e.code] ?? "•") + " " + e.message;
      if (e.entity) {
        const id = e.entity;
        text.title = "クリックでこの個体にフォーカス";
        const go = () => { if (!this.focus(id)) this.flash(item, "この個体はもういません"); };
        text.onpointerdown = e => { if (e.button === 0) go(); };
        text.onclick = e => { if (e.detail === 0) go(); };
      }
      item.append(tick, text);
      return item;
    });
    this.log.replaceChildren(...items);
  }

  /** Focus the next living member of a species; repeated clicks cycle through members. */
  private focusSpecies(row: HTMLElement, species: number) {
    const n = this.cursor.get(species) ?? 0;
    const id = this.member(species, n);
    this.cursor.set(species, n + 1);
    if (!id || !this.focus(id)) this.flash(row, "この種の個体はもういません");
  }

  private flash(item: HTMLElement, message: string) {
    if (this.notice) { clearTimeout(this.notice.timer); this.notice.element.remove(); }
    const element = document.createElement("span");
    element.className = "event-notice";
    element.textContent = message;
    item.append(element);
    this.notice = { element, timer: window.setTimeout(() => element.remove(), 2500) };
  }
}

function empty(text: string) {
  const li = document.createElement("li");
  li.className = "empty";
  li.textContent = text;
  return li;
}

interface Mimicry {
  counts: Record<string, number>;
  avoided_by_carnivores: Record<string, number | null>;
  attacks_per_1000_individual_ticks: Record<string, number | null>;
  mimic_share_of_undefended: number | null;
}
const GROUP_LABELS: [string, string][] = [["defended", "有毒"], ["mimic", "擬態型"], ["other", "その他の無毒"]];

/** Issue #72: mimicry indicators table with a cautious one-line reading. */
export function drawMimicry(root: HTMLElement, json: string) {
  const m = JSON.parse(json) as Mimicry;
  const pct = (v: number | null | undefined) => v == null ? "—" : (v * 100).toFixed(0) + "%";
  const num = (v: number | null | undefined) => v == null ? "—" : v.toFixed(2);
  const table = document.createElement("table");
  table.className = "mimicry-table";
  const head = table.insertRow();
  for (const text of ["", "個体数", "肉食に避けられる", "襲われやすさ"]) {
    const th = document.createElement("th"); th.textContent = text; head.append(th);
  }
  for (const [key, label] of GROUP_LABELS) {
    const row = table.insertRow();
    for (const text of [label, String(m.counts[key] ?? 0), pct(m.avoided_by_carnivores[key]),
      num(m.attacks_per_1000_individual_ticks[key])]) row.insertCell().textContent = text;
  }
  const verdict = document.createElement("p");
  verdict.className = "mimicry-verdict";
  const am = m.avoided_by_carnivores.mimic, ao = m.avoided_by_carnivores.other;
  const rm = m.attacks_per_1000_individual_ticks.mimic, ro = m.attacks_per_1000_individual_ticks.other;
  verdict.textContent = am == null || ao == null || rm == null || ro == null
    ? "比べるための個体がまだそろっていません。"
    : am > ao && rm < ro ? "擬態型は、その他の無毒より避けられ、襲われにくくなっています（目安）。"
    : am > ao ? "擬態型はその他の無毒より避けられていますが、襲われにくさの差はまだ見えません（目安）。"
    : "擬態型が守られている様子はまだ見えません（目安）。";
  const share = document.createElement("p");
  share.className = "species-totals";
  share.textContent = "無毒の草食のうち擬態型の割合：" + pct(m.mimic_share_of_undefended)
    + "　／　襲われやすさ＝1000 個体Tickあたりの攻撃回数（累計）";
  root.replaceChildren(table, verdict, share);
}

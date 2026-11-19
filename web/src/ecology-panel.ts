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

  constructor(private ranking: HTMLElement, private tabs: HTMLElement, private totals: HTMLElement,
    private log: HTMLElement, private focus: (id: Id) => boolean) {
    for (const button of tabs.querySelectorAll<HTMLButtonElement>("button[data-kind]")) {
      button.onclick = () => { this.kind = button.dataset.kind as Kind; this.drawRanking(); };
    }
    this.drawLog();
  }

  reset() {
    this.summary = undefined;
    this.events = [];
    this.drawRanking();
    this.drawLog();
  }

  updateSpecies(json: string) {
    this.summary = JSON.parse(json) as Summary;
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
        text.onclick = () => {
          if (!this.focus(id)) this.flash(item, "この個体はもういません");
        };
      }
      item.append(tick, text);
      return item;
    });
    this.log.replaceChildren(...items);
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

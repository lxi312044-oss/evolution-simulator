import { creatureShape, drawCreature, drawPlantSprite } from "./sprites";

export type Kind = "plant" | "herbivore" | "carnivore";
type Id = [number, number];
type Sprite = [number, number, number, number, number, number, number];
export const species = { plant: "植物", herbivore: "草食", carnivore: "肉食" };

export class EcosystemMap {
  private zoom = 1;
  private offset = [0, 0];
  private selection?: Id;
  private removed = false;
  private hits: { id: Id; kind: Kind; x: number; y: number; radius: number }[] = [];
  private drag?: { x: number; y: number; dx: number; dy: number; moved: boolean };
  private context: CanvasRenderingContext2D;
  private world?: { width: number; height: number };

  constructor(private canvas: HTMLCanvasElement, private list: HTMLSelectElement,
    private detail: HTMLElement, private redraw: () => void) {
    this.context = canvas.getContext("2d")!;
    canvas.addEventListener("wheel", e => {
      e.preventDefault();
      this.scale(e.deltaY < 0 ? 1.25 : 0.8, ...this.point(e));
    }, { passive: false });
    canvas.addEventListener("pointerdown", e => {
      if (e.button !== 0) return;
      const [x, y] = this.point(e);
      this.drag = { x, y, dx: this.offset[0], dy: this.offset[1], moved: false };
      canvas.setPointerCapture(e.pointerId);
    });
    canvas.addEventListener("pointermove", e => {
      if (!this.drag || this.zoom === 1) return;
      const [x, y] = this.point(e);
      const drag = this.drag;
      if (!drag.moved && Math.hypot(x - drag.x, y - drag.y) < 5) return;
      drag.moved = true;
      this.offset = [drag.dx + x - drag.x, drag.dy + y - drag.y];
      this.clamp();
      redraw();
    });
    canvas.addEventListener("pointerup", e => {
      if (this.drag && !this.drag.moved) this.pick(...this.point(e));
      this.drag = undefined;
    });
    canvas.addEventListener("pointercancel", () => { this.drag = undefined; });
    list.onchange = () => {
      const [index, generation] = list.value.split(":").map(Number);
      this.selection = [index, generation];
      this.removed = false;
      redraw();
    };
  }

  private point(e: MouseEvent): [number, number] {
    const box = this.canvas.getBoundingClientRect();
    const x = (e.clientX - box.left - this.canvas.clientLeft) * this.canvas.width / this.canvas.clientWidth;
    const y = (e.clientY - box.top - this.canvas.clientTop) * this.canvas.height / this.canvas.clientHeight;
    return [x, y];
  }

  scale(factor: number, x = this.canvas.width / 2, y = this.canvas.height / 2) {
    const next = Math.max(1, Math.min(12, this.zoom * factor));
    const ratio = next / this.zoom;
    this.offset = [x - (x - this.offset[0]) * ratio, y - (y - this.offset[1]) * ratio];
    this.zoom = next;
    this.clamp();
    this.redraw();
  }

  private clamp() {
    this.offset = this.offset.map((v, i) =>
      Math.max((1 - this.zoom) * (i ? this.canvas.height : this.canvas.width), Math.min(0, v)));
  }

  center() {
    this.zoom = 1;
    this.offset = [0, 0];
    this.redraw();
  }

  /** Select an entity and zoom to it; false when it no longer exists (Issue #67). */
  focus(id: Id, inspect: (index: number, generation: number) => string): boolean {
    const data = JSON.parse(inspect(...id));
    if (!data || !this.world) return false;
    const [x, y] = data.position as [number, number];
    const { width, height } = this.canvas;
    this.zoom = Math.max(this.zoom, 4);
    this.offset = [width / 2 - x * width / this.world.width * this.zoom,
      height / 2 - y * height / this.world.height * this.zoom];
    this.clamp();
    this.selection = id;
    this.removed = false;
    this.list.hidden = true;
    this.redraw();
    return true;
  }

  reset() {
    this.selection = undefined;
    this.removed = false;
    this.list.hidden = true;
    this.zoom = 1;
    this.offset = [0, 0];
    this.detail.textContent = "マップの個体を選択してください。";
  }

  private pick(x: number, y: number) {
    const matches = this.hits.map(hit => ({ ...hit, distance: Math.hypot(hit.x - x, hit.y - y) }))
      .filter(hit => hit.distance <= hit.radius).sort((a, b) => a.distance - b.distance);
    this.list.replaceChildren(...matches.map(hit =>
      new Option(species[hit.kind] + " " + hit.id.join(":"), hit.id.join(":"))));
    this.list.hidden = matches.length < 2;
    this.selection = matches[0]?.id;
    this.removed = false;
    if (!this.selection) this.detail.textContent = "マップの個体を選択してください。";
    this.redraw();
  }

  draw(groups: Record<Kind, Sprite[]>, ids: Uint32Array,
    world: {width: number; height: number; land_columns: number; land_rows: number},
    inspect: (index: number, generation: number) => string, shapes: number[][] = []) {
    const ctx = this.context;
    this.world = world;
    const { width, height } = this.canvas;
    const sx = width / world.width * this.zoom, sy = height / world.height * this.zoom;
    const [ox, oy] = this.offset;
    ctx.fillStyle = "#0c1b18";
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = "#213a32";
    ctx.beginPath();
    for (let i = 0; i <= world.land_columns; i++) {
      const x = ox + i * width * this.zoom / world.land_columns;
      if (x >= 0 && x <= width) { ctx.moveTo(x, 0); ctx.lineTo(x, height); }
    }
    for (let i = 0; i <= world.land_rows; i++) {
      const y = oy + i * height * this.zoom / world.land_rows;
      if (y >= 0 && y <= height) { ctx.moveTo(0, y); ctx.lineTo(width, y); }
    }
    ctx.stroke();
    this.hits = [];
    let cursor = 0, animal = 0;
    for (const kind of ["plant", "herbivore", "carnivore"] as const) {
      for (const [x, y, r, state, red, green, blue] of groups[kind]) {
        const id: Id = [ids[cursor++], ids[cursor++]];
        const traits = kind === "plant" ? undefined : shapes[animal++];
        const px = x * sx + ox, py = y * sy + oy;
        const size = Math.max(kind === "plant" ? 8 : 17, r * Math.min(sx, sy) * 2);
        if (px < -size || py < -size || px > width + size || py > height + size) continue;
        if (kind === "plant") drawPlantSprite(ctx, px, py, state, size, [red, green, blue]);
        else drawCreature(ctx, kind, px, py, size, [red, green, blue], creatureShape(kind, traits));
        this.hits.push({ id, kind, x: px, y: py, radius: Math.max(9, size / 2) });
      }
    }
    if (!this.selection || this.removed) return;
    const data = JSON.parse(inspect(...this.selection));
    if (!data) {
      this.removed = true;
      this.detail.textContent = "個体 " + this.selection.join(":") + " は死亡または除去されました。";
      return;
    }
    const selected = this.hits.find(hit => hit.id[0] === this.selection![0] && hit.id[1] === this.selection![1]);
    if (selected) {
      ctx.strokeStyle = "#fce6a1";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(selected.x, selected.y, selected.radius + 3, 0, 2 * Math.PI);
      ctx.stroke();
      ctx.lineWidth = 1;
    }
    this.show(data);
  }

  private show(data: Record<string, unknown>) {
    const fmt = (value: unknown): string => value == null ? "未取得"
      : typeof value === "number" ? Number(value.toFixed(4)).toString()
      : typeof value === "boolean" ? value ? "はい" : "いいえ"
      : Array.isArray(value) ? value.map(fmt).join(" / ") : String(value);
    const labels: Record<string, string> = {
      position: "位置", rgb: "RGB（0〜1）", age: "年齢（Tick）", max_age: "寿命（Tick）",
      parent: "親ID", birth_tick: "出生Tick", hp: "HP", adult_hp_max: "成体HP上限",
      reserve_e: "予備E", reserve_e_max: "予備E上限", body_n: "身体N", reserve_n: "予備N",
      stomach_n: "胃N", stomach_n_max: "胃N上限", stomach_e: "胃E", stomach_e_max: "胃E上限",
      toxic: "有毒", toxin_type: "毒型", toxin_stock: "保有毒量", growth: "成長度",
      seed_cooldown: "種子クールダウン（Tick）", load: "被毒負荷（型順）",
      resistance: "毒耐性（型順）", feeding_active: "摂食制御が許可状態",
      target: "記録された対象ID", last_target: "直近の追跡対象ID",
      reproduction_cooldown: "繁殖クールダウン（Tick）", attack_cooldown: "攻撃クールダウン（Tick）",
      births: "直接出生数", well_fed_ticks: "満腹継続（Tick）", dose: "累積毒曝露",
      damage: "累積毒損傷（HP）", species: "種", tolerance: "現在のEでの色許容度",
      action: "実行した行動", reproduction: "実際の繁殖判定",
    };
    const kind = data.kind as Kind;
    const title = document.createElement("strong");
    title.textContent = species[kind] + "  #" + (data.id as Id).join(":");
    const color = document.createElement("span");
    color.className = "swatch";
    color.style.backgroundColor = "rgb(" + (data.rgb as number[]).map(v => Math.round(v * 255)).join(",") + ")";
    title.prepend(color);
    const primary = kind === "plant" ? ["species", "age", "toxic", "growth", "body_n"]
      : ["species", "age", "hp", "reserve_e", "stomach_n"];
    const entries = (keys: string[]) => {
      const dl = document.createElement("dl");
      for (const key of keys) {
        const dt = document.createElement("dt"), dd = document.createElement("dd");
        dt.textContent = labels[key];
        dd.textContent = key === "toxin_type" && typeof data[key] === "number"
          ? (data[key] < 2 ? ["A", "B"][data[key]] : String(data[key])) : fmt(data[key]);
        dl.append(dt, dd);
      }
      return dl;
    };
    let detail = this.detail.querySelector("details");
    const open = detail?.open ?? document.body.dataset.mode === "dev";
    detail = document.createElement("details");
    detail.open = open;
    const summary = document.createElement("summary");
    summary.textContent = "詳しい状態";
    const keys = Object.keys(labels).filter(k => !primary.includes(k) && (
      k in data || ["position", "rgb", "parent", "birth_tick"].includes(k)
      || kind !== "plant" && ["action", "reproduction"].includes(k)));
    detail.append(summary, entries(keys));
    if (kind !== "plant") {
      const memory = document.createElement("p");
      const values = data.memory as { rgb: number[]; risk_same_color: number }[] | null;
      memory.textContent = "学習した色と同色の危険度: " + (values == null ? "未取得" : values.length
        ? values.map(entry => fmt(entry.rgb) + " → " + fmt(entry.risk_same_color)).join("、") : "記憶なし");
      detail.append(memory);
    }
    this.detail.replaceChildren(title, entries(primary), detail);
  }
}

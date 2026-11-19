"""Read-only endpoint definitions for the Issue #7 fixed-horizon experiment."""
from collections import Counter
import math
import statistics


def entity(value):
    if (not isinstance(value, (list, tuple)) or len(value) != 2
            or any(type(x) is not int or x < 0 for x in value)):
        raise ValueError('entity must be [index, generation] with nonnegative integers')
    return tuple(value)


def lineage_metrics(founders, focal, births, terminals, present, horizon, focal_birth_counter):
    """All-world events up to H; terminals and end-of-H presence partition all IDs.

    Birth: {tick,parent,child}; terminal: {tick,id}; present: {id,hp}.
    Same-tick births cannot reproduce in this model. HP0 presence is not survival.
    """
    if type(horizon) is not int or horizon <= 0:
        raise ValueError('positive integer horizon required')
    ids = [entity(x) for x in founders]
    focal = entity(focal)
    if len(set(ids)) != len(ids) or focal not in ids:
        raise ValueError('duplicate founders or missing focal')
    born_at = dict.fromkeys(ids, 0)
    depths = {focal: 0}
    counts = Counter()
    direct = 0
    previous_tick = 0
    parent_events = []
    for row in births:
        tick, parent, child = row['tick'], entity(row['parent']), entity(row['child'])
        if type(tick) is not int or not 1 <= tick <= horizon or tick < previous_tick:
            raise ValueError('birth times outside horizon or unordered')
        if parent not in born_at or born_at[parent] >= tick or child in born_at:
            raise ValueError('missing/future parent, same-tick reproduction or duplicate child')
        born_at[child] = tick
        parent_events.append((parent, tick))
        if parent in depths:
            depths[child] = depths[parent] + 1
            counts[depths[child]] += 1
        direct += parent == focal
        previous_tick = tick
    removed = {}
    for row in terminals:
        ident, tick = entity(row['id']), row['tick']
        if (ident not in born_at or ident in removed or type(tick) is not int
                or not born_at[ident] < tick <= horizon):
            raise ValueError('invalid or duplicate removal')
        removed[ident] = tick
    for parent, tick in parent_events:
        if parent in removed and removed[parent] <= tick:
            raise ValueError('birth after or during parent removal')
    alive, pending = set(), set()
    seen = set()
    for row in present:
        ident, hp = entity(row['id']), row['hp']
        if ident not in born_at or ident in seen or ident in removed or not math.isfinite(hp):
            raise ValueError('invalid, duplicate or ambiguous endpoint presence')
        seen.add(ident)
        (alive if hp > 0 else pending).add(ident)
    if set(born_at) != set(removed) | seen:
        raise ValueError('missing endpoint: every individual must be removed or present')
    if type(focal_birth_counter) is not int or focal_birth_counter != direct:
        raise ValueError('focal lifetime birth counter disagrees with genealogy')
    descendants = set(depths) - {focal}
    total = len(descendants)
    return dict(horizon=horizon, direct_births=direct, lineage_births=total,
                deeper_births=total-direct, living_descendants=len(descendants & alive),
                present_hp_nonpositive_descendants=len(descendants & pending),
                removed_descendants=len(descendants & set(removed)),
                born_by_depth=dict(sorted(counts.items())),
                living_by_depth=dict(sorted(Counter(depths[x] for x in descendants & alive).items())),
                focal_living=focal in alive, focal_present_hp_nonpositive=focal in pending)


def interaction(low_on, high_on, low_off, high_off, metric):
    """Seed-level interaction; same endpoint definition and horizon in all four arms."""
    rows = [low_on, high_on, low_off, high_off]
    if len({x['horizon'] for x in rows}) != 1:
        raise ValueError('mismatched horizons')
    return (high_on[metric] - low_on[metric]) - (high_off[metric] - low_off[metric])


def primary_summary(contrasts, on_differences):
    """Fixed n=32 approximate t intervals. Secondary endpoints are not promoted here."""
    def interval(values):
        if len(values) != 32 or any(not math.isfinite(x) for x in values):
            raise ValueError('exactly 32 finite seed contrasts required')
        mean, sd = statistics.mean(values), statistics.stdev(values)
        half = 2.039513446 * sd / math.sqrt(32) if sd > 0 else None
        return dict(mean=mean, sd=sd, halfwidth=half,
                    ci=None if half is None else [mean-half, mean+half],
                    positive=sum(x > 0 for x in values), negative=sum(x < 0 for x in values),
                    zero=sum(x == 0 for x in values))
    primary, on = interval(contrasts), interval(on_differences)
    signal = (primary['ci'] is not None and on['ci'] is not None
              and primary['ci'][0] > 0 and primary['mean'] >= 0.2 and on['ci'][0] > 0)
    return dict(primary=primary, high_minus_low_on=on, positive_signal=signal,
                minimum_effect_supported=bool(signal and primary['ci'][0] >= 0.2),
                precision_target_met=primary['halfwidth'] is not None and primary['halfwidth'] <= 0.2,
                interval_assumption='approximate seed-level t; sparse/skewed counts may be unreliable')

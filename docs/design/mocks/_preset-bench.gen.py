# Generates docs/design/mocks/preset-bench.html from the AUTHORED tokens.
# The mock is not transcribed — it is driven by matrix.json + hues.json, so it
# cannot drift from what the build generates.
import json, io, os
import pathlib
R = str(pathlib.Path(__file__).resolve().parents[3])  # repo root, wherever it is cloned
M = json.load(io.open(R+'/crates/coldlight/tokens/matrix.json', encoding='utf-8'))
H = json.load(io.open(R+'/crates/coldlight/tokens/hues.json', encoding='utf-8'))

hues = {}
for g in H['groups'].values():
    for h in g.get('hues', []):
        if 'hue' not in h: continue   # white/black are component literals, not ladders
        hues[h['id']] = {'hue': h['hue'], 'chroma': h['chroma'],
                         'curve': h.get('curve', 'brand'),
                         'chromaCurve': h.get('chromaCurve', 'linear-to-500')}

# THE PRESETS ARE NOT AUTHORED HERE ANY MORE. `tokens/presets.json` (kind
# `ancestral`) is their one home, checked by build.rs against each lineage's
# own rulings. This bench RENDERS that array, so the mock and the app cannot
# disagree about what a preset is.
P = json.load(io.open(R + '/crates/coldlight/tokens/presets.json', encoding='utf-8'))
PRESETS = {}
for row in P['ancestral']:
    c = row['coordinate']
    PRESETS.setdefault(c['ancestor'], []).append((
        row['id'], row['label'], row.get('kicker', ''),
        c['structure'], c['material'], c['atmosphere'],
        row.get('tone', 'cyan'), row.get('blurb', ''),
    ))

anc = {a['id']: a for a in M['ancestors']}
SIG = lambda x: json.dumps({k: x.get(k) for k in ('fill','ink','edge_color')}, sort_keys=True)
matsig = {m['id']: SIG(m) for m in M['materials']}

# ── legality + distinctness gate. A mock that ships an illegal coordinate, or
#    two "unique" presets that resolve to the same paint, is worse than no mock
#    — so this REFUSES rather than warns.
struct = {x['id']: x for x in M['structures']}
atmo   = {x['id']: x for x in M['atmospheres']}
def paint_sig(st, mt, at):
    """Everything that reaches a pixel. Two presets differing here look different."""
    s_, m_, a_ = struct[st], matsig[mt], atmo[at]
    return json.dumps([s_['radius'], s_['edge_width'], m_,
                       a_.get('shadows'), a_.get('back_glow'), a_.get('tints')], sort_keys=True)

errs, capped = [], []
for aid, rows in PRESETS.items():
    a = anc[aid]
    legal = {ax: [a['native'][ax]] + a.get('allowed', {}).get(ax, []) for ax in ('structure','material','atmosphere')}
    seen, mats = {}, {}
    for pid, _l, _k, st, mt, at, tone, _w in rows:
        for ax, v in (('structure', st), ('material', mt), ('atmosphere', at)):
            if v not in legal[ax]:
                errs.append(f'{aid}/{pid}: {ax} "{v}" is not legal (legal: {legal[ax]})')
        if tone not in hues:
            errs.append(f'{aid}/{pid}: unknown hue "{tone}"')
        seen.setdefault(paint_sig(st, mt, at), []).append(pid)
        mats.setdefault(matsig[mt], []).append(mt)
    for _s, ids in seen.items():
        if len(ids) > 1:
            errs.append(f'{aid}: {ids} resolve to IDENTICAL paint — not unique presets')
    # How many materially-distinct looks can this ancestor even reach?
    eff = len({matsig[m] for m in ([a['native']['material']] + a.get('allowed', {}).get('material', []))})
    if len(mats) < 3:
        capped.append(f'{aid}: 3 presets share only {len(mats)} distinct material(s) '
                      f'— the lineage has {eff} effective of '
                      f'{1 + len(a.get("allowed", {}).get("material", []))} named')
if errs:
    print('REFUSED:'); [print('  -', e) for e in errs]; raise SystemExit(1)
if capped:
    print('MATERIAL CAP (caused by the matte==raw / paper==clay collision):')
    [print('  !', c) for c in capped]
n=sum(len(v) for v in PRESETS.values())
print(f'legality + distinctness: {n} coordinates OK across {len(PRESETS)} ancestors')

DATA = {'structures': M['structures'], 'materials': M['materials'],
        'atmospheres': M['atmospheres'], 'hues': hues,
        'ancestors': [{'id': a['id'], 'label': a['label'], 'blurb': a['blurb'],
                       'font': a['font'], 'native': a['native'],
                       'allowed': a.get('allowed', {}), 'forbidden': a.get('forbidden', {}),
                       'presets': [{'id': p[0], 'label': p[1], 'kicker': p[2],
                                    'c': {'structure': p[3], 'material': p[4], 'atmosphere': p[5]},
                                    'tone': p[6], 'why': p[7]} for p in PRESETS[a['id']]]}
                      for a in M['ancestors']]}

tpl = io.open(R + '/docs/design/mocks/_preset-bench.tpl.html', encoding='utf-8').read()
out = tpl.replace('/*__DATA__*/null', json.dumps(DATA, ensure_ascii=False))
io.open(R + '/docs/design/mocks/preset-bench.html', 'w', encoding='utf-8').write(out)
print('wrote docs/design/mocks/preset-bench.html  (%.0f KB)' % (len(out)/1024))

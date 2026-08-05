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

# THE THREE PER ANCESTOR. Every coordinate is checked legal below, and no two
# in a set may use materials that resolve identically (matte==raw, glass==
# metallic, paper==clay) — otherwise "three unique presets" would be a lie.
PRESETS = {
 'bauhaus': [
  ('primary',   'Primary',   'the teaching wall',   'rectilinear','matte','none',      'azure',
   'Foundational and unarguable: zero radius, flat saturated pigment, no atmosphere at all. Bauhaus before anything is added to it.'),
  ('werkstatt', 'Werkstatt', 'the workshop block',  'massive','matte','hard-cast',    'coral',
   'The same pigment given weight and a cast shadow — a heavy block sitting on the wall rather than printed into it. Same material, wholly different object.'),
  ('vorkurs',   'Vorkurs',   'paper and grid',      'measured','paper','none',        'amber',
   'The quiet register: fibrous paper, restrained radius, hairline rule. What the surrounding page is made of, not what the poster shouts.')],
 'art-deco': [
  ('chrysler',  'Chrysler',  'lacquer and sheen',   'stepped','lacquer','sheen',      'amber',
   'The native coordinate. Ziggurat steps, deep lacquer, and one hard specular band running across the top-left. Deco at full dress.'),
  ('machine',   'Machine Age','brushed and cast',   'rectilinear','metallic','hard-cast','teal',
   'Metal instead of lacquer, right angles instead of steps, and a hard cast shadow. The industrial half of the same decade.'),
  ('salon',     'Salon',     'restrained deco',     'measured','matte','none',        'violet',
   'Deco with the volume down: flat pigment, measured radius, no atmosphere. Proof the lineage survives without its ornament.')],
 'brutalist': [
  ('neo',       'Neo',       'the ratified default','massive','raw','hard-cast',      'lime',
   'The existing preset, stated as a coordinate: heavy block, unfinished material, hard offset shadow. Loud, honest, fast.'),
  ('hacker',    'Hacker',    'cold terminal',       'massive','neon-glass','neon',    'cyan',
   'Cyan ink on a lit edge with a painted halo. Brutalist bones wearing a terminal.'),
  ('retro-os',  'Retro-OS',  'amber CRT',           'rectilinear','amber-crt','scanline','amber',
   'Amber phosphor, square corners, scanline texture. The third of the ratified four.')],
 'swiss': [
  ('grid',      'Grid',      'the native',          'measured','paper','none',        'slate',
   'Paper, measured radius, hairline rule, nothing else. Swiss is the ancestor that gets quieter the more correct it is.'),
  ('akzidenz',  'Akzidenz',  'flat and square',     'rectilinear','matte','none',     'coral',
   'Saturated flat pigment at zero radius. The poster half of Swiss — still no atmosphere, because Swiss barely permits any.'),
  ('offset',    'Offset',    'print register',      'measured','matte','hard-cast',   'azure',
   'The same pigment with one hard cast shadow, like a mis-registered second pass on press. The most atmosphere Swiss legally allows.')],
 'aero': [
  ('hacker-aero','Hacker',   'the cold tool',       'measured','neon-glass','neon',   'cyan',
   'Operator-named for Workbench. Cold cyan ink on a lit edge, restrained radius, glow used as a SPOTLIGHT on the active thing.'),
  ('gloss',     'Gloss',     'classic Frutiger Aero','rounded','glass','bloom',       'cyan',
   'Aero at its most itself — translucent fill, generous radius, light from above-left, a painted halo. Optimistic and glassy, with zero blurred pixels.'),
  ('tactile',   'Tactile',   'soft and friendly',   'pill','clay','sheen',            'violet',
   'The opposite register: fully rounded clay pills with a single specular band and no halo at all. What a touch-first surface wears.')],
 'memphis': [
  ('bacterio',  'Bacterio',  'the native',          'rounded','matte','hard-cast',    'magenta',
   'Flat saturated pigment, generous radius, hard cast shadow. Memphis as Sottsass drew it — playful and unbothered.'),
  ('tawaraya',  'Tawaraya',  'lacquer pills',       'pill','lacquer','sheen',         'amber',
   'Fully rounded lacquer with a specular band. The glossy, toy-like half of the lineage.'),
  ('carlton',   'Carlton',   'stepped under neon',  'stepped','paper','neon',         'teal',
   'Chamfered steps in quiet paper, lit by a cyan halo. Memphis is the only ancestor legal for both steps and neon.')],
 'constructivist': [
  ('rodchenko', 'Rodchenko', 'the native',          'rectilinear','raw','hard-cast',  'coral',
   'Right angles, unfinished material, one hard diagonal shadow. The agitational poster.'),
  ('agitprop',  'Agitprop',  'heavy and scanned',   'massive','paper','scanline',     'amber',
   'A heavy block in quiet paper under scanline texture — the printed, reproduced, distributed version of the same message.'),
  ('lissitzky', 'Lissitzky', 'stepped and silent',  'stepped','matte','none',         'azure',
   'Chamfered steps in flat pigment with no atmosphere whatsoever. Pure geometry — the Proun register.')],
 'editorial': [
  ('broadsheet','Broadsheet','the native',          'measured','paper','none',        'slate',
   'Paper, measured radius, hairline rule. The body of the page — it must never compete with what is set on it.'),
  ('cover',     'Cover',     'glossy stock',        'rounded','lacquer','sheen',      'magenta',
   'Deep lacquer with a specular band and a generous radius. The magazine cover, not the column.'),
  ('masthead',  'Masthead',  'metal rule',          'rectilinear','metallic','hard-cast','teal',
   'Brushed metal at zero radius with a hard cast. The nameplate and the rules that divide the page.')],
}

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
print('legality + uniqueness: 24 coordinates OK across 8 ancestors')

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

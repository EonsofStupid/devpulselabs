# Real frames of the real app, labelled by the app itself: every control's name,
# role and exact rectangle comes from EguiPilotAI's semantic tree in the same
# session as the capture. No AI and no hand labelling. Scratch library only;
# runs one at a time.
import json, os, re, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
CLI = r'D:\projects\devpulselabs\target\release\eguipilotai.exe'
ROUTES = ['home/main', 'wiredfront/editor', 'wiredfront/library', 'wiredfront/overview',
          'wiredfront/viziwiz', 'wiredfront/gallery', 'wiredfront/sessions', 'workbench/library',
          'workbench/author', 'workbench/episodes', 'workbench/editor', 'workbench/lessons',
          'workbench/studio', 'workbench/experiments', 'present/shows', 'coach/main']
SIZES = ['1440x900', '1280x800', '1200x750', '1100x700', '1000x680', '1366x768']
SCENARIO = '''id = "frame"

[[steps]]
kind = "wait"
milliseconds = 1500

[[steps]]
kind = "capture"
name = "frame"

# Deliberately never true: a failing step returns the whole semantic tree of
# this same session, so the labels and the picture can't drift apart.
[[steps]]
kind = "expect"

[steps.expectation]
type = "visible"

[steps.expectation.target]
id = "root/never-present"
'''


def slug(route, size):
    return f"{route.replace('/', '-')}_{size}"


def config(combos):
    base = open(os.path.join(HERE, '.eguipilotai.toml'), encoding='utf-8').read().split('[fixtures.')[0]
    fixtures = ''.join(
        f'[fixtures.{slug(r, s)}]\narguments = ["--route", "{r}", "--window", "{s}"]\n\n' for r, s in combos)
    with open(os.path.join(HERE, '.eguipilotai.toml'), 'w', encoding='utf-8') as handle:
        handle.write(base + fixtures)
    with open(os.path.join(HERE, 'frame.toml'), 'w', encoding='utf-8') as handle:
        handle.write(SCENARIO)


def walk(value):
    if isinstance(value, dict):
        yield value
        for item in value.values():
            yield from walk(item)
    elif isinstance(value, list):
        for item in value:
            yield from walk(item)


def nodes_from(payload):
    found = {}
    for node in walk(payload):
        rect = node.get('rect')
        if isinstance(rect, dict) and 'min_x' in rect and node.get('id'):
            found[node['id']] = {k: node.get(k) for k in ('id', 'role', 'name', 'value', 'actions', 'rect')}
    return list(found.values())


def one(route, size):
    name = slug(route, size)
    out = subprocess.run([CLI, 'test', 'spike', 'frame.toml', name], cwd=HERE, capture_output=True,
                         text=True, encoding='utf-8', errors='replace', timeout=120)
    text = out.stdout
    start = text.find('{')
    payload = json.loads(text[start:text.rfind('}') + 1])
    run = payload.get('run_directory')
    pictures = [os.path.join(run, 'captures', f) for f in os.listdir(os.path.join(run, 'captures'))
                if f.endswith('.png')] if run and os.path.isdir(os.path.join(run, 'captures')) else []
    nodes = nodes_from(payload)
    if not pictures or not nodes:
        return {'route': route, 'size': size, 'ok': False, 'pictures': len(pictures), 'nodes': len(nodes),
                'tail': text[-300:], 'err': out.stderr[-300:]}
    frame = os.path.join(HERE, 'frames', name + '.png')
    shutil.copy(pictures[0], frame)
    return {'route': route, 'size': size, 'ok': True, 'frame': frame, 'nodes': nodes}


def main():
    limit = int(sys.argv[1]) if len(sys.argv) > 1 else None
    combos = [(r, s) for r in ROUTES for s in SIZES]
    config(combos)
    os.makedirs(os.path.join(HERE, 'frames'), exist_ok=True)
    os.makedirs(os.path.join(HERE, 'lib', 'content'), exist_ok=True)
    picked = [('wiredfront/editor', '1100x700')] if limit == 1 else combos
    started = time.time()
    with open(os.path.join(HERE, 'dataset.jsonl'), 'w', encoding='utf-8') as out:
        for index, (route, size) in enumerate(picked):
            try:
                row = one(route, size)
            except Exception as error:
                row = {'route': route, 'size': size, 'ok': False, 'error': repr(error)[:300]}
            out.write(json.dumps(row) + '\n')
            out.flush()
            print(index + 1, '/', len(picked), route, size, 'ok' if row.get('ok') else 'FAILED',
                  len(row.get('nodes', [])), 'nodes', round(time.time() - started), 's', flush=True)
            if not row.get('ok'):
                print('   ', {k: v for k, v in row.items() if k not in ('nodes',)}, flush=True)


main()

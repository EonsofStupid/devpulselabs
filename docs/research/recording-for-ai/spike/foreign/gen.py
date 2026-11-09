# UIs that look nothing like the app: generated pages in other themes and fonts,
# with commands the model never saw, screenshotted by headless Chrome (nothing on
# the operator's desktop). Every position is exact because code placed it.
import html, json, os, random, subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
CHROME = r'C:\Program Files\Google\Chrome\Application\chrome.exe'
THEMES = [
    {'bg': '#ffffff', 'ink': '#1a1a1a', 'button': '#e8e8e8', 'edge': '#9a9a9a', 'side': '#f3f3f3'},
    {'bg': '#f6f1e4', 'ink': '#3a2f1c', 'button': '#e6dcc2', 'edge': '#8c7a52', 'side': '#ece3cd'},
    {'bg': '#eef4fb', 'ink': '#0b2545', 'button': '#2f6fb7', 'edge': '#1d4e89', 'side': '#d9e6f5', 'label': '#ffffff'},
    {'bg': '#202124', 'ink': '#e8eaed', 'button': '#3c4043', 'edge': '#5f6368', 'side': '#292a2d'},
    {'bg': '#ffffff', 'ink': '#000000', 'button': '#ffffff', 'edge': '#000000', 'side': '#ffffff'},
]
FONTS = ['Segoe UI', 'Arial', 'Georgia', 'Consolas', 'Tahoma', 'Verdana', 'Times New Roman']
SIZES = [(1280, 800), (1100, 700), (1440, 900), (1024, 768)]
COMMANDS = ['Export report', 'Add column', 'Send invoice', 'Filter rows', 'New folder', 'Sign out', 'Merge cells',
            'Print preview', 'Share link', 'Archive', 'Rename', 'Sort by date', 'Upload file', 'Delete row',
            'Settings', 'Help', 'Undo', 'Redo', 'Zoom in', 'Zoom out', 'Open recent', 'Save as', 'Insert table',
            'Spell check', 'Find', 'Replace', 'Bold', 'Italic', 'Align left', 'Align right', 'Refresh', 'Sync now',
            'Add user', 'Remove user', 'Invite', 'Download', 'Duplicate', 'Move to trash', 'Mark as read', 'Reply']
FILLER = ('Quarterly totals are grouped by region and updated every night. Select a row to see its history, or '
          'use the toolbar to change how the table is shown. ')


def page(seed):
    rng = random.Random(seed)
    theme, font = rng.choice(THEMES), rng.choice(FONTS)
    width, height = rng.choice(SIZES)
    size = rng.choice([12, 13, 14, 15, 16])
    round_ = rng.choice([0, 3, 8, 16])
    names = rng.sample(COMMANDS, 12)
    placed, boxes = [], []
    # A toolbar along the top, a sidebar, and a few buttons in the page.
    x, y = 12, 12
    for name in names[:5]:
        w, h = len(name) * (size * 0.62) + 24, size + 18
        placed.append((name, x, y, w, h))
        x += w + rng.randint(6, 18)
    y = 80
    for name in names[5:9]:
        w, h = 150, size + 20
        placed.append((name, 12, y, w, h))
        y += h + rng.randint(8, 30)
    for name in names[9:]:
        w, h = len(name) * (size * 0.62) + 28, size + 20
        placed.append((name, rng.randint(200, width - int(w) - 20), rng.randint(height // 2, height - int(h) - 20), w, h))
    body = []
    for name, x, y, w, h in placed:
        body.append(
            f'<button style="position:absolute;left:{x:.0f}px;top:{y:.0f}px;width:{w:.0f}px;height:{h:.0f}px;'
            f'background:{theme["button"]};color:{theme.get("label", theme["ink"])};border:1px solid {theme["edge"]};'
            f'border-radius:{round_}px;font:{size}px \'{font}\';padding:0">{html.escape(name)}</button>')
        boxes.append({'name': name, 'rect': {'min_x': round(x), 'min_y': round(y), 'max_x': round(x + w), 'max_y': round(y + h)}})
    text = (f'<div style="position:absolute;left:190px;top:80px;width:{width - 220}px;color:{theme["ink"]};'
            f'font:{size}px \'{font}\';line-height:1.5">{FILLER * 3}</div>')
    side = f'<div style="position:absolute;left:0;top:64px;width:176px;bottom:0;background:{theme["side"]}"></div>'
    document = (f'<!doctype html><meta charset=utf-8><body style="margin:0;background:{theme["bg"]};overflow:hidden">'
                f'{side}{text}{"".join(body)}')
    return document, boxes, width, height


def main():
    os.makedirs(os.path.join(HERE, 'pages'), exist_ok=True)
    rows = []
    for seed in range(24):
        document, boxes, width, height = page(seed)
        source = os.path.join(HERE, 'pages', f'{seed}.html')
        frame = os.path.join(HERE, 'pages', f'{seed}.png')
        with open(source, 'w', encoding='utf-8') as out:
            out.write(document)
        try:
            subprocess.run([CHROME, '--headless=new', f'--user-data-dir={os.path.join(HERE, "chrome", str(seed))}',
                            '--force-device-scale-factor=1', '--hide-scrollbars', f'--window-size={width},{height}',
                            f'--screenshot={frame}', 'file:///' + source.replace('\\', '/')],
                           capture_output=True, timeout=45)
        except subprocess.TimeoutExpired:
            print(seed, 'chrome timed out', flush=True)
        rows.append({'frame': frame, 'boxes': boxes, 'exists': os.path.exists(frame)})
        print(seed, width, height, 'ok' if rows[-1]['exists'] else 'FAILED', flush=True)
    with open(os.path.join(HERE, 'dataset.jsonl'), 'w', encoding='utf-8') as out:
        for row in rows:
            out.write(json.dumps(row) + '\n')
    from PIL import Image, ImageDraw
    image = Image.open(rows[2]['frame']).convert('RGB')
    print('frame size', image.size)
    draw = ImageDraw.Draw(image)
    for box in rows[2]['boxes']:
        r = box['rect']
        draw.rectangle([r['min_x'], r['min_y'], r['max_x'], r['max_y']], outline=(255, 0, 0), width=2)
    image.save(os.path.join(HERE, 'check.png'))


main()

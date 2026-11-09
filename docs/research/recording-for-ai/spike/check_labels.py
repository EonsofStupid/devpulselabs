# Draw the first dataset row's labels on its frame, to see they land on the
# controls. The pixel scale is printed too: labels are in points, frames in pixels.
import json, os
from PIL import Image, ImageDraw

here = os.path.dirname(os.path.abspath(__file__))
row = json.loads(open(os.path.join(here, 'dataset.jsonl'), encoding='utf-8').readline())
image = Image.open(row['frame']).convert('RGB')
draw = ImageDraw.Draw(image)
print('frame', image.size, 'size asked', row['size'], 'nodes', len(row['nodes']))
right = max(n['rect']['max_x'] for n in row['nodes'])
bottom = max(n['rect']['max_y'] for n in row['nodes'])
print('labels reach', right, bottom)
for node in row['nodes']:
    rect = node['rect']
    draw.rectangle([rect['min_x'], rect['min_y'], rect['max_x'], rect['max_y']], outline=(255, 40, 40), width=2)
    print(f"  {node['role']:<10} {str(node['name'])[:38]:<40} {[round(rect[k]) for k in ('min_x', 'min_y', 'max_x', 'max_y')]} {node.get('actions')}")
image.save(os.path.join(here, 'check.png'))

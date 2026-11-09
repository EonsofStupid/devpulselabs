# Does training on the app's own labelled frames actually help a real vision
# model find controls? Scores the untouched model first, trains a LoRA, then
# scores it again on the SAME questions, which training never saw:
#   new_size   : screens it trained on, at a window size it never saw
#   new_screen : screens it never saw (shared chrome included)
#   new_control: on those screens, only controls whose name it never saw
# A hit is a predicted point inside the control's real rectangle.
import json, os, random, re, statistics, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
os.environ.setdefault('HF_HOME', os.path.join(HERE, 'hf'))

import torch
from PIL import Image
from peft import LoraConfig, get_peft_model
from transformers import AutoModelForImageTextToText, AutoProcessor

MODEL = 'Qwen/Qwen3.5-2B'
TEST_SIZE = '1366x768'
HELD_OUT_ROUTES = {'wiredfront/sessions', 'workbench/lessons'}
STEPS = int(os.environ.get('STEPS', 3000))
SCORE_EACH = int(os.environ.get('SCORE_EACH', 150))
LR, ACCUMULATE, RANK = 2e-4, 4, 16
random.seed(11)
torch.manual_seed(11)


def question(name):
    return (f'Where is the control named "{name}" in this screenshot? Answer with only its '
            'centre point as x,y, each a whole number from 0 to 999, as a share of the image '
            'width and height.')


def load():
    samples = []
    path = os.path.join(HERE, 'data', 'dataset.jsonl')
    for line in open(path, encoding='utf-8'):
        row = json.loads(line)
        if not row.get('ok'):
            continue
        width, height = Image.open(row['frame']).size
        for node in row['nodes']:
            name = (node.get('name') or '').replace('�', '·').strip()
            if not name or 'click' not in (node.get('actions') or []):
                continue
            r = node['rect']
            if r['max_x'] > width + 1 or r['max_y'] > height + 1:
                continue
            samples.append({
                'route': row['route'], 'size': row['size'], 'frame': row['frame'], 'name': name,
                'width': width, 'height': height, 'rect': r,
                'x': round((r['min_x'] + r['max_x']) / 2 / width * 999),
                'y': round((r['min_y'] + r['max_y']) / 2 / height * 999),
            })
    return samples


def split(samples):
    if os.environ.get('HOLD_NAMES'):
        # The harder test: some controls are never seen at all, at any size. The
        # shared menus can't be held out, since they sit on every screen.
        routes = {s['route'] for s in samples}
        on = {}
        for s in samples:
            on.setdefault(s['name'], set()).add(s['route'])
        candidates = sorted(n for n, where in on.items() if where != routes)
        held = set(random.Random(23).sample(candidates, round(len(candidates) * 0.3)))
        print(json.dumps({'held_out_controls': sorted(held), 'of': len(candidates)}, ensure_ascii=False), flush=True)
        train = [s for s in samples if s['name'] not in held and s['size'] != TEST_SIZE]
        return train, {
            'never_seen_control': [s for s in samples if s['name'] in held],
            'seen_control_new_size': [s for s in samples if s['name'] not in held and s['size'] == TEST_SIZE],
        }
    train = [s for s in samples if s['route'] not in HELD_OUT_ROUTES and s['size'] != TEST_SIZE]
    names = {s['name'] for s in train}
    new_size = [s for s in samples if s['route'] not in HELD_OUT_ROUTES and s['size'] == TEST_SIZE]
    new_screen = [s for s in samples if s['route'] in HELD_OUT_ROUTES]
    new_control = [s for s in new_screen if s['name'] not in names]
    return train, {'new_size': new_size, 'new_screen': new_screen, 'new_control': new_control}


processor = AutoProcessor.from_pretrained(MODEL, min_pixels=256 * 28 * 28, max_pixels=1024 * 28 * 28)


def prompt_text(sample):
    messages = [{'role': 'user', 'content': [{'type': 'image'}, {'type': 'text', 'text': question(sample['name'])}]}]
    try:
        return processor.apply_chat_template(messages, tokenize=False, add_generation_prompt=True, enable_thinking=False)
    except TypeError:
        return processor.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)


def picture(sample, cache={}):
    if sample['frame'] not in cache:
        cache[sample['frame']] = Image.open(sample['frame']).convert('RGB')
    return cache[sample['frame']]


def score(model, cases, label):
    model.eval()
    hits, distances, unparsed, shown = 0, [], 0, []
    started = time.time()
    with torch.inference_mode():
        for sample in cases:
            batch = processor(text=[prompt_text(sample)], images=[picture(sample)], return_tensors='pt').to(model.device)
            out = model.generate(**batch, max_new_tokens=28, do_sample=False)
            reply = processor.decode(out[0][batch['input_ids'].shape[1]:], skip_special_tokens=True).strip()
            # A tuned model can repeat its answer on new lines, so read one line.
            line = next((text for text in reply.splitlines() if re.search(r'\d', text)), '')
            numbers = [int(n) for n in re.findall(r'\d{1,4}', line)]
            if len(shown) < 3:
                shown.append(f"{sample['name'][:24]!r} -> {reply[:30]!r} (truth {sample['x']},{sample['y']})")
            if len(numbers) < 2:
                unparsed += 1
                continue
            # The untouched model often answers with a box, not a point. Score a
            # box by its centre, so the baseline isn't marked down for its format.
            if len(numbers) >= 4:
                px, py = (numbers[0] + numbers[2]) / 2, (numbers[1] + numbers[3]) / 2
            else:
                px, py = numbers[0], numbers[1]
            x = px / 999 * sample['width']
            y = py / 999 * sample['height']
            r = sample['rect']
            hits += r['min_x'] <= x <= r['max_x'] and r['min_y'] <= y <= r['max_y']
            cx, cy = (r['min_x'] + r['max_x']) / 2, (r['min_y'] + r['max_y']) / 2
            distances.append(((x - cx) ** 2 + (y - cy) ** 2) ** 0.5)
    result = {
        'label': label, 'questions': len(cases), 'hit_rate': round(hits / max(len(cases), 1), 4), 'hits': hits,
        'median_px_off': round(statistics.median(distances), 1) if distances else None,
        'unparsed': unparsed, 'seconds': round(time.time() - started), 'examples': shown,
    }
    print(json.dumps(result, ensure_ascii=False), flush=True)
    return result


def trainable_targets(model):
    # Every linear layer except the output head and the patch embedding, in both
    # the language side and the vision side: finding a control needs both.
    names = set()
    for name, module in model.named_modules():
        if isinstance(module, torch.nn.Linear) and not any(k in name for k in ('lm_head', 'patch_embed')):
            names.add(name)
    return sorted(names)


def train(model, data):
    params = [p for p in model.parameters() if p.requires_grad]
    optimizer = torch.optim.AdamW(params, lr=LR, weight_decay=0.0)
    model.train()
    model.gradient_checkpointing_enable(gradient_checkpointing_kwargs={'use_reentrant': False})
    model.enable_input_require_grads()
    order, losses, started = [], [], time.time()
    for step in range(STEPS):
        if not order:
            order = random.sample(data, len(data))
        sample = order.pop()
        prompt = prompt_text(sample)
        answer = f"{sample['x']},{sample['y']}<|im_end|>"
        batch = processor(text=[prompt + answer], images=[picture(sample)], return_tensors='pt').to(model.device)
        prompt_length = processor(text=[prompt], images=[picture(sample)], return_tensors='pt')['input_ids'].shape[1]
        labels = batch['input_ids'].clone()
        labels[:, :prompt_length] = -100
        with torch.autocast('cuda', dtype=torch.bfloat16):
            loss = model(**batch, labels=labels).loss
        (loss / ACCUMULATE).backward()
        losses.append(loss.item())
        if (step + 1) % ACCUMULATE == 0:
            torch.nn.utils.clip_grad_norm_(params, 1.0)
            optimizer.step()
            optimizer.zero_grad()
        if (step + 1) % 100 == 0:
            print(json.dumps({'step': step + 1, 'loss': round(sum(losses[-100:]) / 100, 3),
                              'seconds': round(time.time() - started),
                              'gpu_gb': round(torch.cuda.max_memory_allocated() / 1e9, 1)}), flush=True)
    model.gradient_checkpointing_disable()


def foreign():
    # Pages that look nothing like the app, with commands it never saw.
    cases = []
    for line in open(os.path.join(HERE, 'foreign', 'dataset.jsonl'), encoding='utf-8'):
        row = json.loads(line)
        if not row['exists']:
            continue
        width, height = Image.open(row['frame']).size
        for box in row['boxes']:
            r = box['rect']
            if r['max_x'] <= width and r['max_y'] <= height:
                cases.append({'frame': row['frame'], 'name': box['name'], 'width': width, 'height': height, 'rect': r,
                              'x': round((r['min_x'] + r['max_x']) / 2 / width * 999),
                              'y': round((r['min_y'] + r['max_y']) / 2 / height * 999)})
    model = AutoModelForImageTextToText.from_pretrained(MODEL, dtype=torch.bfloat16, device_map={'': 0})
    before = score(model, cases, 'before foreign')
    from peft import PeftModel
    tuned = PeftModel.from_pretrained(model, os.path.join(HERE, 'adapter-held'))
    after = score(tuned, cases, 'after foreign')
    print(f"=== foreign look: {before['hit_rate']:.1%} -> {after['hit_rate']:.1%}  median off "
          f"{before['median_px_off']} -> {after['median_px_off']} px  ({len(cases)} questions)", flush=True)
    with open(os.path.join(HERE, 'result-foreign.json'), 'w', encoding='utf-8') as out:
        json.dump({'before': before, 'after': after}, out, indent=1, ensure_ascii=False)


def main():
    if '--foreign' in sys.argv:
        return foreign()
    samples = load()
    data, tests = split(samples)
    picked = {k: random.sample(v, min(SCORE_EACH, len(v))) for k, v in tests.items()}
    print(json.dumps({'samples': len(samples), 'train': len(data), 'train_frames': len({s['frame'] for s in data}),
                      'tests': {k: len(v) for k, v in tests.items()},
                      'scored': {k: len(v) for k, v in picked.items()}}), flush=True)
    model = AutoModelForImageTextToText.from_pretrained(MODEL, torch_dtype=torch.bfloat16, device_map={'': 0})
    print('loaded', sum(p.numel() for p in model.parameters()) // 1_000_000, 'M params',
          round(torch.cuda.memory_allocated() / 1e9, 1), 'GB', flush=True)
    before = {k: score(model, v, 'before ' + k) for k, v in picked.items()}
    if '--baseline' in sys.argv:
        return
    if '--rescore' in sys.argv:
        from peft import PeftModel
        tuned = PeftModel.from_pretrained(model, os.path.join(HERE, 'adapter'))
        after = {k: score(tuned, v, 'after ' + k) for k, v in picked.items()}
        print('=== before -> after (rescored)')
        for key in picked:
            print(f"{key}: {before[key]['hit_rate']:.1%} -> {after[key]['hit_rate']:.1%}  "
                  f"median off {before[key]['median_px_off']} -> {after[key]['median_px_off']} px  "
                  f"({len(picked[key])} questions)", flush=True)
        return
    targets = trainable_targets(model)
    print('lora targets', len(targets), flush=True)
    model = get_peft_model(model, LoraConfig(r=RANK, lora_alpha=RANK * 2, lora_dropout=0.05, target_modules=targets))
    model.print_trainable_parameters()
    train(model, data)
    tag = '-held' if os.environ.get('HOLD_NAMES') else ''
    model.save_pretrained(os.path.join(HERE, 'adapter' + tag))
    after = {k: score(model, v, 'after ' + k) for k, v in picked.items()}
    with open(os.path.join(HERE, f'result{tag}.json'), 'w', encoding='utf-8') as out:
        json.dump({'model': MODEL, 'steps': STEPS, 'rank': RANK, 'before': before, 'after': after}, out, indent=1, ensure_ascii=False)
    print('=== before -> after')
    for key in picked:
        print(f"{key}: {before[key]['hit_rate']:.1%} -> {after[key]['hit_rate']:.1%}  "
              f"median off {before[key]['median_px_off']} -> {after[key]['median_px_off']} px  "
              f"({len(picked[key])} questions)", flush=True)


main()

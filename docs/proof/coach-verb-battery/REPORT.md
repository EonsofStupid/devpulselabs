# Coach verb battery — 2026-08-01T01:26:58-04:00
DISPLAY=:1 APP=gnome-calculator

### health — PASS

daemon ok

```
{"adr":"docs/plans/coach-control-plane-adr.md","bind":"127.0.0.1","ok":true,"service":"dpl-coach-daemon","verbs_adr":"docs/plans/coach-operator-verb-catalog-adr.md"}

```

### verbs — PASS

catalog 20 entries · shipped=19 planned=1

```
{"verbs":[{"name":"inventory","status":"shipped","target":"desktop"},{"name":"presence","status":"shipped","target":"app"},{"name":"import","status":"shipped","target":"app"},{"name":"resolve","status":"shipped","target":"fabric"},{"name":"invoke","status":"shipped","target":"fabric"},{"name":"click","status":"shipped","target":"fabric"},{"name":"mount","status":"shipped","target":"fabric"},{"name":"move","status":"shipped","target":"window"},{"name":"resize","status":"shipped","target":"window"},{"name":"focus","status":"shipped","target":"window"},{"name":"observe","status":"shipped","target":"app"},{"name":"open","status":"shipped","target":"app"},{"name":"close","status":"shipped","target":"app"},{"name":"type_text","status":"shipped","target":"app"},{"name":"press_key","status":"shipped","target":"app"},{"name":"select","status":"shipped","target":"app"},{"name":"scroll","status":"shipped","target":"app"},{"name":"drag","status":"planned","target":"fabric"},{"name":"wait_until","status":"shipped","target":"predicate"},{"name":"assert_state","status":"shipped","target":"app"}]}

```

### inventory — PASS

apps=64 includes calculator

```
[
  {
    "atspi_live": true,
    "complexity": "small",
    "display_name": "Calculator",
    "id": "org.gnome.Calculator.desktop",
    "operator_real": true,
    "presence": {
      "mapped_window": true,
      "outer_h": 680,
      "outer_w": 482,
      "pid": 3857932,
      "pid_alive": true
    }
  },
  {
    "atspi_live": false,
    "complexity": "deep",
    "display_name": "LibreOffice Calc",
    "id": "libreoffice-calc.desktop",
    "operator_real": false,
    "presence": {
      "mapped_window": false,
      "outer_h": 0,
      "outer_w": 0,
      "pid": null,
      "pid_alive": false
    }
  }
]

```

### presence_unreal — PASS

real=false after close

```
{"app":"gnome-calculator","atspi_listed":false,"mapped_window":false,"outer_h":0,"outer_w":0,"pid":null,"pid_alive":false,"real":false,"window_id":null}

```

### assert_unreal — PASS

assert unreal ok

```
{"atspi_name":"gnome-calculator","detail":null,"matched":true,"ok":true,"pid":null,"predicate":"unreal","real":false,"verb":"assert_state"}

```

### open — PASS

effect_observed + real_after

```
{"atspi_name":"gnome-calculator","effect_observed":true,"ok":true,"pid":3897014,"real_after":true,"real_before":false,"verb":"open"}

```

### presence_real — PASS

mapped 482x680 pid alive

```
{"app":"gnome-calculator","atspi_listed":true,"mapped_window":true,"outer_h":680,"outer_w":482,"pid":3897014,"pid_alive":true,"real":true,"window_id":52428804}

```

### wait_until — PASS

matched real

```
{"atspi_name":"gnome-calculator","detail":null,"matched":true,"ok":true,"pid":3897014,"predicate":"real","real":true,"verb":"wait_until"}

```

### assert_real — PASS

assert real ok

```
{"atspi_name":"gnome-calculator","detail":null,"matched":true,"ok":true,"pid":3897014,"predicate":"real","real":true,"verb":"assert_state"}

```

### geom — PASS

geom observed

```
{"after":{"csv":"142,114,482,680","h":680,"w":482,"window_id":52428804,"x":142,"y":114},"before":{"csv":"142,114,482,680","h":680,"w":482,"window_id":52428804,"x":142,"y":114},"effect_observed":true,"ok":true,"verb":"geom"}

```

### move — PASS

to 280,220

```
{"after":{"csv":"280,220,482,680","h":680,"w":482,"window_id":52428804,"x":280,"y":220},"before":{"csv":"142,114,482,680","h":680,"w":482,"window_id":52428804,"x":142,"y":114},"effect_observed":true,"ok":true,"verb":"move"}

```

### resize — PASS

to 560x700

```
{"after":{"csv":"280,220,560,700","h":700,"w":560,"window_id":52428804,"x":280,"y":220},"before":{"csv":"280,220,482,680","h":680,"w":482,"window_id":52428804,"x":280,"y":220},"effect_observed":true,"ok":true,"verb":"resize"}

```

### focus — PASS

focus effect_observed

```
{"after":{"csv":"280,220,560,700","h":700,"w":560,"window_id":52428804,"x":280,"y":220},"before":{"csv":"280,220,560,700","h":700,"w":560,"window_id":52428804,"x":280,"y":220},"effect_observed":true,"ok":true,"verb":"focus"}

```

### observe — PASS (retested; first FAIL was grep `event:moved` vs wire `event: moved`)

SSE missing moved/resized (moved=0 resized=0)

```
event: hello
data: {"app":"gnome-calculator","kind":"observe_start","seconds":5}

event: geom
data: {"app":"gnome-calculator","geom":{"csv":"280,220,560,700","h":700,"w":560,"window_id":52428804,"x":280,"y":220},"kind":"geom"}

event: moved
data: {"after":{"csv":"300,240,560,700","h":700,"w":560,"window_id":52428804,"x":300,"y":240},"app":"gnome-calculator","before":{"csv":"280,220,560,700","h":700,"w":560,"window_id":52428804,"x":280,"y":220},"dx":20,"dy":20,"kind":"moved"}

event: resized
data: {"after":{"csv":"300,240,580,720","h":720,"w":580,"window_id":52428804,"x":300,"y":240},"app":"gnome-calculator","before":{"csv":"300,240,560,700","h":700,"w":560,"window_id":52428804,"x":300,"y":240},"dh":20,"dw":20,"kind":"resized"}

event: bye
data: {"app":"gnome-calculator","kind":"observe_end"}

```

### type_text — PASS

display_after=3

```
{"atspi_name":"gnome-calculator","display_after":"3","display_before":"","effect_observed":true,"ok":true,"text":"3","verb":"type_text"}

```

### press_key — PASS

display_after=35

```
{"atspi_name":"gnome-calculator","display_after":"35","display_before":"3","effect_observed":true,"key":"5","ok":true,"verb":"press_key"}

```

### assert_display — PASS

display contains 5

```
{"atspi_name":"gnome-calculator","detail":"display=\"35\"","matched":true,"ok":true,"pid":3897014,"predicate":"real","real":true,"verb":"assert_state"}

```

### select — PASS

ok effect_observed=true

```
{"atspi_name":"gnome-calculator","effect_observed":true,"name":"Basic","ok":true,"state_after":"button|StateSet(BitFlags<State>(0b1000000000001000011000000000000000000000000, Sensitive | Showing | Visible | HasPopup))","state_before":"button|StateSet(BitFlags<State>(0b1000000000001000011000000000000000000000000, Sensitive | Showing | Visible | HasPopup))","verb":"select"}

```

### scroll — SOFT

ok via=xdotool_wheel but effect_observed=false (known Calculator limit)

```
{"atspi_name":"gnome-calculator","delta":2,"direction":"down","effect_observed":false,"ok":true,"snapshot_after":"","snapshot_before":"","verb":"scroll","via":"xdotool_wheel"}

```

### import — PASS

session=sess-1785562032741 nodes=81

```
{"agency":"coach","atspi_name":"gnome-calculator","minted_ms":369,"nodes":81,"ok":true,"seed_path":"0/0/0/0/0/0/0/0/1/0/0/1","session":"sess-1785562032741"}

```

### resolve — PASS

seed name=7

```
{"actions":["Click"],"name":"7","ok":true,"path":"0/0/0/0/0/0/0/0/1/0/0/1","role":"button","window_rect":"16,400,84,44"}

```

### invoke_click — PASS

click effect_observed display 35->357

```
{"action":"Click","agency":"coach","display_after":"357","display_before":"35","effect_observed":true,"ok":true}

```

### mount — PASS

window_rect=16,400,84,44 (no overlay spawn)

```
{"atspi_name":"gnome-calculator","expected_name":"7","ok":true,"overlay":null,"window_rect":"16,400,84,44"}

```

### drag_planned — PASS

correctly refused as not implemented

```
{"refused":true,"reason":"verb not implemented: drag","tip":"GET /verbs for shipped names. Use /import|/invoke for fabric verbs."}

```

### close — PASS

real→unreal

```
{"atspi_name":"gnome-calculator","effect_observed":true,"ok":true,"pid":null,"real_after":false,"real_before":true,"verb":"close"}

```

### assert_unreal_final — PASS

final unreal

```
{"atspi_name":"gnome-calculator","detail":null,"matched":true,"ok":true,"pid":null,"predicate":"unreal","real":false,"verb":"assert_state"}

```

## Summary

- PASS: 25
- SOFT: 1 (scroll delivery ok, no Calculator scroll offset)
- FAIL: 0

**BATTERY: GREEN**

Retest note: `observe` SSE confirmed live with `event: moved` + `event: resized` while Calculator geometry changed (`/tmp/events2.sse`).

# Census raw — CONTENT SYSTEM + NATIVE SEAM (old repo) — 73 capabilities

## Content tree & schemas
1. show.json contract: id/prefix/title/kind/brand/summary; kinds teaching|news|episodic
2. episode.json manifest: only file runtime reads; schemaVersion 1|2 union brick-safe
3. Slide schema: component/props/beat/chrome/tone/fx/density/steps; fx shape-only validation (degrade no-op)
4. Reveal-step schema: 1–9 steps, step0 implicit SETUP
5. Strict-JSON props scope (serializable across boundary)
6. Script frontmatter contract (MDX-lite): slideId/title/beats
7. Kebab/id pattern law (<prefix>-###)

## Validation doctrine
8. Cross-field integrity: dup chapter/slide ids, dangling chapter refs (fatal)
9. Teaching chapter-lock ch01/02/03 in order
10. Interaction budget ≤3 interactive
11. Slide-count band 15–24
12. Doctor core: schema, required files, naming, component+props, fx membership, steps-parity, asset existence, script beats-parity, chapter contiguity
13. MDX-lite parser (per-file degrade to null)
14. Manifest→Deck adapter w/ per-slide validation + broken-slide error render

## Episode template & wizard
15. Episode template builder: day-zero file set, shared by wizard+doctor
16. Required-files list enforced
17. createEpisode fs path w/ arktype segment jail

## Scan/load + Rust twins
18. scanShows / scan_shows (skip-invalid)
19. loadEpisode / load_episode_raw (Rust dumb reader; validation client-side, same modules)
20. loadScripts / load_script_files per-slide keyed
21. doctor_raw: one call = raw show+manifest+listing+scripts
22. write_episode_files jailed new-episode writer
23. read/update_episode_file whitelisted editable IO (never assets/show.json)
24. Native seam compile-time desktop/web split
25. contentStatusNative: where resolution landed + paths checked
26. contentRootNative session-cached
27. Content-root resolution: CLI arg → portable anchor → bundled → ./content; NO env on desktop
28. content-cli headless bin (scan-shows|load-episode|load-scripts|content-root)

## Asset protocol
29. makeAssetResolver per-episode (convertFileSrc / /content/* + CDN base)
30. resolveContentFile content-root-relative
31. resolveAbsoluteFile for outside-root paths (inbox)
32. Forward-slash normalization contract

## Artifact index DB (Turso embedded)
33. db_recent_artifacts newest-first + location filter
34. ArtifactRecord 15-field wire shape (incl poster, sizeKb, modifiedMs, aiLabels)
35. upsert write-through from every capture path (ON CONFLICT reactivate)
36. reindex_artifacts from disk+sidecars; soft-clear vanished; empty-scan guard
37. rekey (keep→content) + soft_clear (dump keeps row history)
38. db_update_artifact operator tags edit; inbox rows also rewrite sidecar
39. Additive migrations; corrupt DB recreated
40. Local embedded artifacts.db at data root; fresh conn per op
41. db_set_ai_labels reserved AI-labeling entry — PARTIAL (no producer wired)

## Filing laws
42. Inbox under data/ (never content/) so DUMP legal
43. unique_dest_ext never-clobber
44. replay_inbox_keep: file per metadata; episode must EXIST; companions follow; rekey
45. replay_inbox_pull_downloads: sweep browser-ext staging → inbox
46. replay_inbox_dump: delete inbox artifact + companions (+guide recursive)
47. Inbox jails: file jail, entry jail, read-only subdir jail w/ symlink escape check
48. associate_artifact: copy library master INTO episode generated (copy never move)
49. import_inbox_asset: inbox still → episode generated, whitelist, validated segments
50. write_inbox_image: ViziWiz composite PNG (base64, 20MB cap, PNG magic) → inbox
51. list_artifacts flat filterable everything-generated listing
52. scan_all_artifacts truth-feed for reindex
53. Sidecar law: .meta.json + .poster.jpg companions travel with media
54. inbox_asset_path / read_inbox_json read-only nested access
55. import-replays / use-replay CLI filing

## Media pipeline (scripts)
56. Image optimize → AVIF+WebP+JPEG, 2560px, strip meta, idempotent
57. Video transcode → H.264 faststart + poster
58. Bg-removal cutout (isolated worker) → transparent AVIF/WebP/PNG
59. Animation build → looping animated WebP + poster
60. Sequence stitch → concat clips per JSON → mp4 + poster

## Portable packaging
61. fetch-ffmpeg pinned sha256-verified static
62. package-portable one self-contained folder; Linux AppImage; preflight
63. data_root resolution: data/ beside content or portable anchor; NO appdata fallback
64. Managed ffmpeg resolution (bundled→resource→dev→PATH) injected
65. runtime_status self-report (roots, ffmpeg source, OS session)
66. Portable anchor across Win exe-dir + Linux $APPIMAGE
67. portable-drill P1–P8 self-containment proof
68. check-bundle: build fails unless registrations/fonts survive minification
69. workbench-drill headless authoring drill
70. Kiosk mode --kiosk + taskbar work-area clamp

## Env/config
71. .env template VITE_ vars + CONTENT_ROOT (all optional)
72. CONTENT_ROOT honored web/scripts; desktop CLI-arg only
73. ext-episodes: regen replay-extension episode dropdown from content tree

Seams: single-doctor/single-schema invariant (web+desktop same modules; Rust dumb reader). Rust load_script_files stricter (kebab-only). Only incomplete: ai_labels producer (#41).

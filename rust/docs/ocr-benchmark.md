# Local OCR - 0.36.5

The OCR edition includes the unmodified English PP-OCRv5 mobile recognizer
(7,872,351 bytes) and native ONNX Runtime 1.23.2. Rust performs segmentation,
resizing, CTC decoding and item validation. An independent tract CPU engine is
built into the OCR edition if the native runtime fails its known-input test.
Both engines use the same pinned model. No Python interpreter, recognition
server or GPU is used. The model is not custom-trained on PoE yet.

## Hardware target

- Windows 10 1903+ or Windows 11, x64.
- Provisional target: i3-8300-class quad-core CPU, 8 GB total system RAM, with
  memory available alongside Boosteroid and other applications.
- Dedicated GPU: not required for OCR. The cloud client has its own requirements.

This is a test target, not a measured minimum. We have not benchmarked the real
i3-8300/8 GB/GTX 1050 Ti PC with Boosteroid and Discord running. Model file size
does not describe total process memory. The GUI, capture buffers, native runtime,
catalogues and fallback readings also consume memory.

## Initial measurements

Development machine: Ryzen 9 7945HX, about 32 GB RAM. ONNX Runtime is restricted
to two compute threads; inter-op parallelism and idle spinning are disabled.
One session is loaded lazily and reused. Limiting threads does not emulate an i3.

The native diagnostic reads an already-cropped image twice: original glyphs and
colored glyphs. These timings exclude screen capture, automatic tooltip location,
Tesseract fallback, the interactive GUI and process startup. The first run includes
session creation; subsequent runs reuse it. Six repetitions were used.

| Binary / image | First pair | Warm pair | Peak process memory |
| --- | --- | --- | --- |
| Linux / advanced armour | 1.34 s | 1.20 s | 95 MiB RSS |
| Linux / compressed boots | 0.75 s | 0.54-0.61 s | 88 MiB RSS |
| Windows under Wine / compressed boots | 1.10 s | 0.60-0.64 s | 344 MiB working set |

Wine is not a Windows hardware benchmark. RSS and Windows working-set counters
are different measurements; these figures are not the full checker's memory use.
Native Windows measurements remain necessary before stating minimum hardware.

## Accuracy checks

Four supplied tooltips were placed into complete 1920x1080 test frames with
unrelated text. The Windows package located each tooltip automatically and used
PP-OCRv5. The advanced armour retained nine modifier lines, ilvl 86, roll ranges
and implicit/fractured/crafted sources. The compressed boots retained the observed
+41 Energy Shield roll and displayed 308 Energy Shield, without counting a
resistance line twice. Some other captures still have unrecognized text.
This small regression set is not an accuracy percentage for all PoE items.

The isolated GUI suite separately checks cancellation, responsiveness, five item
families, independent standard/OCR shortcuts and the absence of automatic Trade
searches. Real Boosteroid capture and overlay behavior still require Windows tests.

## 0.36.2 regression fixes

Additional local Windows reports exposed valid rolls rejected by literal fragments
of composite catalogue entries (including 25 Life and 9 Energy Shield). Variable
stat templates now take precedence without changing any observed numbers. Quality
punctuation and explanatory Cold Exposure reminders are handled separately.
Gem descriptions no longer count as unknown item modifiers or trigger extra
recognition passes. Corruption footers remain readable across the gap left by
removed descriptions. Missing searchable properties remain uncertain.

Neural recognition can use automatically positioned scout rows when projection
cannot separate text from the translucent background. Empty output is reported
explicitly, and logs identify the version, model, line counts and stage timings.
The regression set now includes seven supplied tooltip images placed in complete
frames without passing their coordinates to the app. The new cases check the
belt's seven modifiers and implicit Life, both glove implicits and all seven
other glove rolls, and Kinetic Blast's level 21, quality 20 and corruption flag.
The normal tooltip pixels do not contain ilvl, so these tests require it to stay
absent. Separate text tests cover the reported advanced roll notation and noise;
the partially covered advanced screenshots are not complete pixel fixtures.

These changes do not train or replace the model. User timing on native Windows
is still required; there is no claimed speedup for every item.

## 0.36.3 native Windows investigation

A supplied 0.36.2 native Windows log confirmed successful model loading (96 ms)
but zero decoded lines from 29 scout regions (576 ms). Capture took 143 ms;
recognition with fallback took 7.03 s. These are one reported item's timings,
not a hardware benchmark or evidence of a missing runtime.

The update tests the recognizer on an embedded synthetic line and retries basic
graph optimizations if that known input fails. Empty scout output also tries
independent projection; rare-tooltip frame edges restrict neighboring UI text.
Fallback reasons are visible and copiable in review. These checks distinguish
runtime execution from bad capture/segmentation; the native failure's cause still
requires the new self-test result rather than a claim based only on Wine tests.

The synthetic probe is a rendered "Energy Shield: 149" line in DejaVu Sans,
240x32 BGRA pixels; it contains no user capture. Recognition still runs locally
in Rust with the bundled native runtime. A deliberately wrong scout region is
included in the packaged executable's regression tests to verify projection retry.

## 0.36.4 session diagnostics

A native Windows report from a Ryzen 7 X3D machine also failed the embedded
synthetic line with both optimized and basic sessions. This isolates the failure
before tooltip recognition, but does not establish a specific CPU/runtime bug.
A third compatibility probe disables graph optimizations and prepacking and uses
one compute thread. It is accepted only if the same known line passes.

The session log records CPU brand/features, Windows build, worker floating-point
controls, ONNX Runtime build information, input tensor checksums and output score
statistics for each probe. The tensor echo checksum can be compared with the
preprocessed input checksum. Blank-class and space-class counts distinguish blank
predictions from zero-filled outputs. No user images or raw tensors are logged.
Settings > Diagnostics and Copy diagnostic share the entire current session log;
no logs are uploaded. Real Windows execution still requires the new report.

## 0.36.5 independent Rust engine

The native 0.36.4 report confirms matching input and echoed-tensor SHA256 hashes,
but all 50 output positions select the blank class with every ONNX configuration.
The same input passes on the development host and under Wine. This isolates a
native execution difference; it does not establish a specific CPU/runtime cause.
The OCR edition now tries tract if ONNX initialization or the known-input test
fails. This independent Rust engine must recognize the same synthetic line with
confidence above 0.9 before it is accepted. Missing or altered model files still
fail validation. Existing fallback OCR remains available if both engines fail.

Tract uses one compute thread and up to eight cached exact-width plans. It reads
the same preprocessed tensors as ONNX, without padding that changes this model's
predictions. Plans and weights are reused
within the process. No extra model, Python process, GPU or server is needed.
The pinned tract 0.22.1 avoids the Windows wmic cache probe introduced in newer
versions, which can open a console and cancel capture by stealing focus.
The log records the transition, self-test and plan compilation timings; the review
identifies the Rust CPU backend explicitly.

The Rust path prioritizes compatibility over latency. Timings exclude capture,
tooltip location, Tesseract and the GUI and do not establish minimum hardware.
Nine full-frame fixtures produce identical modifier kinds, references, values and
ranges with both engines. A deliberately missing native DLL forces the Rust path;
this does not reproduce the native Ryzen X3D failure itself.

The Paladin Boots regression retains all nine visible modifier lines, including
both implicits and Aspect of the Spider, without replacing their numbers. Realm
text joined to a complete requirement line is ignored as metadata; the required
level does not become an item level. This screenshot contains no visible ilvl.

## Reproduce

For a Windows ZIP, run the extracted checker from its directory:

```powershell
.\awakened-crab-trade.exe ocr-neural C:\screens\tooltip.png . 6 > ocr-benchmark.json
```

For the portable EXE, `--extract-only` prints the directory containing that checker.
The JSON contains per-pass timings, observed text/confidence and peak process
memory. The image stays local. Use a cropped tooltip for this diagnostic; the
normal shortcut still finds the tooltip automatically in the captured frame.

Linux developer comparison, from the repository root:

```sh
python3 rust/scripts/setup-neural-ocr.py --platform linux
cargo build --release --locked --features neural-ocr --manifest-path rust/Cargo.toml --bin awakened-crab-trade -j3
rust/target/release/awakened-crab-trade ocr-neural rust/assets/fixtures/ocr/boots-stream.png rust/dist/neural-runtime-linux 6
```

Python and 7-Zip are build helpers only. The shipped EXEs contain no Python
modules. [Pinned model/runtime sources](https://github.com/Shoukshai/awakened-crab-trade/blob/main/rust/packaging/windows/neural-ocr.json).

## Private fine-tuned test builds (0.36.8)

A private PoE fine-tune can replace the recognizer in a test package. The normal
build still downloads the pinned upstream model. Custom builds require both
`CRAB_OCR_MODEL_SHA256` and `CRAB_OCR_MODEL_BYTES` at compile time. The loader
checks the exact size and SHA256 before either CPU backend can use the model.
Package with `package-windows.py --ocr --ocr-model MODEL.onnx
--ocr-model-license LICENSE`, then `package-windows-portable.py --ocr`.

The review identifies these builds as "Crab OCR fine-tuned". The session log and
package license manifest record the model identity. A synthetic uppercase
"ITEM LEVEL: 84" probe must decode exactly with confidence above 0.9 before the
engine is accepted. This probe contains no user screenshot or training crop.
Screenshots, annotations and checkpoints stay outside the repository. The
portable test executable includes only the inference model and its license.

Private evaluation results measure individual cropped lines, not complete items
or end-to-end capture reliability. Real Windows testing remains necessary.


## 0.36.11 latency pass

The OCR daemon validates and loads the existing trained model on its capture
worker at startup. GTK does not wait for that initialization. Recognition remains
local; the weights, profiles and shortcuts are unchanged.

A framed rare tooltip can be reconstructed directly from the original and
color-isolated neural readings. This job runs alongside the existing verification
pipeline. A complete reconstruction cancels the unnecessary verification child;
uncertain readings leave the original pipeline in charge. Both jobs finish or
cancel before another capture can run. Changing the target still cancels the
result. Dim red Corrupted footers are included in projection.

This primarily helps clear rare tooltips. Advanced, heavily compressed or other
item families can still require the slower verification path. It is not a promise
of subsecond recognition on every item, or a measured speedup on the reported
Ryzen X3D and i3/1050 Ti Windows machines.

Private Wine checks on a Ryzen 9 7945HX compare the same complete desktop frames,
with no manually supplied tooltip coordinates. Measured recognition time excludes
process launch, screen capture and GUI display. The clear gloves and body-armour
examples drop from about 17.1/13.1 seconds to 1.4/1.3 seconds. Other cases retain
the verification path; they show much smaller gains. These timings are development
measurements under Wine, not Windows hardware requirements.

The nine pixel fixtures retain their expected modifiers, ranges, flags and item
levels. The body-armour example also corrects an old Evasion reading of 11606 to
the visible 1606. CPU usage from two brief OCR jobs and real Boosteroid latency
still need native Windows testing. Settings > Diagnostics records the winning
path, total recognition time and background warmup result; nothing is uploaded.

# DSCH

A compiler for a structured music composition DSL — parses `.dsch` source and lowers it to MIDI.

> **Status: Work in progress / research.** Active development — architecture and API subject to change.

## Overview

`DSCH` reads `.dsch` files in a custom expression-oriented DSL, compiles them to a tree of scoped musical contexts, schedules those contexts as time-stamped MIDI events, and writes a Standard MIDI File. It was previously the `compiler` crate inside the `sound-studies` workspace and has been spun out into its own self-contained crate.

## Layout

```
dsch/
├── Cargo.toml
├── grammar.pest               # Active PEG grammar
├── prototype.dsch              # Reference composition
├── test.dsch                   # Minimal smoke-test input
└── src/
    ├── main.rs                # Entry point: CLI → parse → compose → schedule → MIDI
    ├── lib.rs                 # Library crate root — re-exports compiler modules
    ├── pest_parser.rs         # Pest PEG parser → AST
    └── compiler/
        ├── mod.rs
        ├── ast.rs             # AST type definitions
        ├── functional.rs      # Monad / Functor / Combinator scaffolding
        ├── composer.rs        # Fold over AST → scoped-context arena
        ├── codegen.rs         # MIDI-domain types (PPQ, Length, Mpb, Velocity, Ctx, Pc, Register, Context, …)
        └── scheduler.rs       # Context tree → time-stamped MIDI track
```

## The `.dsch` DSL

`.dsch` is an expression-oriented language for algorithmic music composition drawing on two programming paradigms:

- **Concatenative** — at the surface level, meaning arises from juxtaposition. Placing expressions next to each other implicitly threads a musical context from left to right, with no explicit binding operator. `d4 (120 144 60 120) bpm` is three tokens in sequence: a duration that sets context, a list that supplies values, and a suffix keyword that consumes them. This is the same model used by languages like Forth and Joy.
- **Functional** — internally, AST nodes compose through the `Monad<Exp>::bind` operator in `functional.rs`, and the compiler pipeline is a left-to-right fold of those binds. Concatenative languages often have this property: Joy's formal semantics are defined entirely in terms of function composition. A planned extension will bring Haskell-style syntax to the DSL itself — allowing users to define higher-kinded types, custom data types, and functions directly in `.dsch` files. This will make the language self-extensible: composers can build and share libraries of reusable abstractions (scales, rhythmic patterns, transformations) in the same language they write music in.

Programs are nested expressions that specify duration, tempo, pitch, register, dynamics, and rhythm.

**Grouping semantics:**

| Syntax | Meaning |
|--------|---------|
| `(...)` | Sequence — expressions play in order |
| `{...}` | Stack — expressions play simultaneously |
| `[a, b, c]` | Set — comma-separated unordered collection |
| `a:b:c` | Ratio — proportional time subdivision |

**Scalars:**

| Form | Meaning |
|------|---------|
| `<n>` `<n>.<n>` `+<n>` `-<n>` | Numbers — integer, float, signed (relative) |
| `d<n>` | Fractional duration (no space) — e.g. `d4` = quarter note, `d8` = eighth note |
| `d<a>:<b>` | Tuplet duration — `a` notes in the time of `b` |
| `5'` `2"` `5'2"` | Fixed duration — minutes, seconds, or combined |
| `<n> bpm` | Tempo |
| `<n>Hz` | Frequency |
| `ppp` `pp` `p` `mp` `mf` `f` `ff` `fff` | Discrete dynamic level |

> **Note:** `d<n>` duration forms are atomic in the grammar — no whitespace is allowed between `d` and the number. `d4` parses as a duration scalar; `d 4` would parse `d` as an identifier. A planned refactor will move duration to a proper prefix so spacing is irrelevant, but this requires additional composer work.

**Prefix:**

| Token | Meaning |
|-------|---------|
| `pc` | Pitch class — bind the following value(s) as pitch classes |
| `reg` | Register (octave) |
| `d` | Duration prefix (planned — not yet dispatched in the composer) |
| `~` | Rest |

**Suffix:**

| Token | Meaning |
|-------|---------|
| `bpm` | Apply a tempo to the preceding compound |
| `Hz`  | Apply a frequency to the preceding compound |
| `A`   | Amplitude (velocity) |

**Infix:**

| Token | Meaning |
|-------|---------|
| `:`   | Ratio separator |
| `><`  | Intercalate — interleave two sequences |
| `..`  | Range — inclusive discrete enumeration between two values |
| `<`   | Interpolate upward — continuously ramp from the left operand to the right |
| `>`   | Interpolate downward — continuously ramp from the left operand to the right |
| `+` `-` `*` `/` | Arithmetic on numeric operands |

`<` and `>` are general interpolation operators rather than dynamics-specific symbols. The parameter being interpolated is determined by the values they connect and the surrounding context: `mp < f` is a crescendo, `120 < 144 bpm` is an accelerando, `220 < 440 Hz` is a glissando, `pc 0 < 12` is a pitch glide of an octave, and so on. `..` and `<`/`>` together form the language's two-axis taxonomy of change over time — discrete enumeration vs. continuous sweep. The range `..` is inclusive at both ends, matching how composers naturally think about musical ranges ("from C to G" includes G).

`...` is reserved for ellipsis semantics in a future revision of the language — continuation, repetition, or "and so on" — and is intentionally kept distinct from `..`.

**Bindings:**

`ident: exp` declares a named expression. Identifiers are alphanumeric (with underscores) and may carry `'` (prime) suffixes — useful for related variants like `theme`, `theme'`, `theme''`. Bindings are stored per-context in the composer's `bindings` arena.

### Example

```
5' (
  d4 (120 144 60 120) bpm
  2:5:7:3 (
    3:7:5:2 (
      5:3:2:7 (
        7:2:3:5 (
          {
            pc (
              (5 3 2 7)
              (7 2 3 5)
              (2 5 7 3)
              (3 7 5 2)
            )
            d (7.75 4.5 8 8.5)
            >< ~ (4.25 4 4.5)
            reg (4 5)
          }
        )
      )
    )
  )
)
```

This specifies a 5-minute composition, subdivided by nested ratios, with quarter-note durations, cycling BPM values, pitch-class sets interleaved with rests across two registers.

## Compiler pipeline

```
.dsch source
    │
    │  Pest PEG parser (grammar.pest)
    ▼
  Program AST
    │
    │  Composer — left-to-right fold over AST,
    │  producing a tree of scoped contexts:
    │  duration · pitch class · tempo · register · velocity · program · bindings
    ▼
  State (Ctx arena: parents · children · scope_types · lengths · pcs · tempos · bpms ·
                    registers · velocities · programs · bindings · stack · garbage)
    │
    │  Scheduler — walks the context tree, fills a BTreeMap<Ticks, Vec<Instruction>>,
    │  emits a single MIDI track with delta-time encoding
    ▼
  MIDI (midly::Smf, PPQ = 25200)
```

### Parser

A Pest PEG grammar (`grammar.pest`) drives a hand-written recursive-descent walker in `pest_parser.rs` that builds the typed AST defined in `compiler/ast.rs`. The AST is rooted at `Exp = Compound | Simple | Noop | EOS`, with `Simple = Prefix | Scalar | Infix | Suffix | Ident` mirroring the grammar's fixity-based taxonomy.

### Composer

The composer in `compiler/composer.rs` runs in two phases.

**Phase 1 — left-to-right fold.** The program expression list (plus a sentinel `EOS`) is pushed onto an `rhs_stack`. A `Monad::ret` seed is then driven through a loop: each iteration pops one token from the `rhs_stack` and passes it to `combine`, which pattern-matches on the `(lhs, rhs)` pair and dispatches via `Monad::bind` to a specialised composer:

- `compose_simple` → `compose_prefix` / `compose_scalar` / `compose_infix` / `compose_suffix` / `compose_ident`
- `compose_scalar` → `compose_duration` / `compose_fractional` / `compose_tempo` / `compose_dynamic` / `compose_frequency` / `compose_pure`
- `compose_prefix` handles `pc`, `reg`, `d` (dur), and `~` (rest); `pc` and `d` distribute themselves over compound arguments by reinserting a rewritten compound back onto the stack
- `compose_infix` — `Mul` is implemented (`(expr) * n` repeats a sequence `n` times); other forms remain `todo!()`
- `compose_decl` stores `ident → exp` in the per-context bindings map; `compose_ident` looks the binding up

Supporting reduction helpers extracted from `combine`: `consume_right_assoc_exps` (drains the lhs-stack for right-associative compound application), `consume_simples` (recursively folds simple tokens off the rhs-stack), `combine_subcomponents` (processes the contents of a compound once loaded onto the rhs-stack), and `consume_sequences` / `merge_sequences` (handles multiple `(...)` sequences inside a `{...}` stack). The older `drain_stack` pass has been removed.

**Phase 2 — `sequence_children` post-composition pass.** After the fold, `sequence_children` recursively restructures the context arena to handle mixed Stack+Sequence trees. For a Stack parent containing Sequence children of different lengths it runs a playhead loop (stepping by the GCD of all child lengths) and calls `take_note` at each beat to extract individual note events into freshly allocated Stack nodes, effectively unrolling the polyphonic grid. `fit` cycles a shorter sequence up to match the total length of a longer one; `expand_context` cycles a context's `pcs`/`lengths`/`velocities` arrays to fill the required count; `merge_sequences` builds a `BTreeMap<u64, Ctx>` of time-stamped Stack nodes and replaces the original children list with the flattened result.

**State arena.** `Ctx::Id(usize)` indexes into parallel `HashMap`s for `parents`, `children`, `scope_types`, `lengths`, `pcs`, `tempos`, `bpms`, `registers`, `velocities`, `programs`, and `bindings`. Three arena-mutation operations work alongside `append_child`: `empty_child` (allocates a zeroed child with no parent-field inheritance, used for structural intermediates), `move_child` (reparents a node from one context to another), and `drop` (immediately removes a context from all maps). Contexts queued for deferred removal go into a `garbage` list and are cleaned up by `collect_garbage` at the end of `compose_program`. Children inherit tempo, register, velocity, and program from their parent at `append_child` time.

**Debug infrastructure.** The composer includes a `graph` function (currently commented out in production paths) that uses `rust-sugiyama` to compute a Sugiyama-style layered layout of the context tree and renders it to stderr via `colonnade`. `print_exps` renders a 3-column lhs-stack / current-context-state / rhs-stack view; `print_state` shows per-context fields. These are wired to toggle via the commented `out(...)` / `execute!(...)` calls throughout the file.

### Codegen types

`compiler/codegen.rs` defines the MIDI-domain types used downstream of the composer: `PPQ` (25200 ticks per quarter — chosen for high divisibility), `MicroSeconds`, `Length`, `Mpb` (microseconds per beat — the internal tempo representation), `Velocity`, `Pc` (`Class(i8) | None`), `Prog`, `Register` (`Reg(i8) | None`), `Context` (a flat per-`Ctx` record of all parallel-vec fields), and an `Instruction` enum with `Midi(MidiMessage)` / `Meta(MetaMessage)` variants. Helpers convert between time domains: `to_length` (fractional → microseconds), `duration_to_micros` (fixed → microseconds), `length_to_ticks` (microseconds → PPQ ticks at a given tempo).

### Scheduler

`compiler/scheduler.rs` consumes the composed `State` and emits a `midly::Smf`. The earlier playhead-loop + `get_counters` approach has been replaced by a recursive `schedule_context` that dispatches cleanly per `ScopeType`:

- `ScopeType::None` — folds over children, accumulating total length without advancing the clock
- `ScopeType::Sequence` — iterates children (or leaf note events) left-to-right, advancing the clock by `length_to_ticks(length, tempo)` after each
- `ScopeType::Stack` — iterates children without advancing the clock between them (all play simultaneously), returning the minimum child length

For leaf contexts, `schedule_note` inserts a `NoteOn`/`NoteOff` pair (zero-velocity `NoteOn` for note-off) into a `BTreeMap<Ticks, Vec<Instruction>>` keyed by absolute tick time. Tempo changes emit as `MetaMessage::Tempo`; an initial `ProgramChange` is inserted at tick 0. `render_tracks` iterates the map in ascending time order and converts absolute ticks to delta-time `TrackEvent`s for a single-track SMF. A `visited: HashSet<Ctx>` field on `Scheduler` is reserved for future cycle detection.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete |
| Grammar — arithmetic infix (`+`, `-`, `*`, `/`) | In grammar; composer dispatch WIP |
| Composer — fractional durations (`d<n>`), tuplets | Working |
| Composer — `pc` (absolute & relative), `reg` | Working |
| Composer — scope types (Sequence, Stack, Set) | Working |
| Composer — tempo (`<n> bpm`) | Working |
| Composer — declarations (`ident: exp`) | Initial dispatch in place; semantics partial |
| Composer — fixed durations (`5'`, `5'2"`) | Grammar defined; composer integration pending |
| Composer — prefix `d` (duration as prefix keyword) | Planned refactor; grammar still uses atomic scalar form |
| Composer — dynamics, frequency, amplitude (`A`), bare suffix forms | Stubbed |
| Composer — infix (`:`, `><`, `..`, `<`, `>`, `+`, `-`, `*`, `/`) | Stubbed |
| Scheduler → MIDI | Working — emits a single-track SMF with delta-time events |

## Running

```bash
# Read <name>.dsch from the working directory, write <name>.mid alongside it
cargo run -- --input <name>
```

For example, `cargo run -- --input test` parses `test.dsch`, composes it, schedules MIDI events, and writes `test.mid`.

## Built with

- [`pest`](https://github.com/pest-parser/pest) — PEG parser generator
- [`pest_derive`](https://github.com/pest-parser/pest) — derive macro for typed Pest grammars
- [`midly`](https://github.com/kovaxis/midly) — MIDI file I/O
- [`clap`](https://github.com/clap-rs/clap) — CLI argument parsing (`--input <name>`)
- [`num-rational`](https://github.com/rust-num/num-rational) — exact rational arithmetic for ratios and tuplets
- [`crossterm`](https://github.com/crossterm-rs/crossterm) — cross-platform terminal control (used for debug/progress output)
- [`colonnade`](https://github.com/dfhoughton/colonnade) — aligned terminal column formatting for state inspection
- [`colprint`](https://crates.io/crates/colprint) — coloured terminal output helpers
- [`rust-sugiyama`](https://github.com/paddison/rust-sugiyama) — Sugiyama-style layered graph layout (context-tree visualisation)

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

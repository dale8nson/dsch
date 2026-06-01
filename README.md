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
| `d<n>` | Fractional duration (e.g. `d4` = quarter note, `d8` = eighth note) |
| `d<a>:<b>` | Tuplet duration — `a` notes in the time of `b` |
| `5'` `2"` `5'2"` | Fixed duration — minutes, seconds, or combined |
| `<n> bpm` | Tempo |
| `<n>Hz` | Frequency |
| `ppp` `pp` `p` `mp` `mf` `f` `ff` `fff` | Discrete dynamic level |

**Prefix:**

| Token | Meaning |
|-------|---------|
| `pc` | Pitch class — bind the following value(s) as pitch classes |
| `reg` | Register (octave) |
| `d` | Distribute a duration across the following compound |
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

The composer in `compiler/composer.rs` is structured as a left-to-right fold over expressions. Adjacent expressions are reduced through `combine`, which dispatches via `Monad::bind` to specialised composers per AST shape (`compose_simple`, `compose_scalar`, `compose_duration`, `compose_fractional`, `compose_prefix`, `compose_suffix`, `compose_infix`, `compose_tempo`, `compose_dynamic`, `compose_decl`, `compose_ratio`, `compose_range`, `compose_pure`, `compose_frequency`, `compose_ident`, …). Helper passes (`drain_stack`, `consume_prefixes`, `consume_compound`, `merge_sequences`) manage the pending-prefix stack and assemble compound expressions before their context is finalised.

State is an arena rather than a recursive structure: `Ctx::Id(usize)` indexes into parallel `Vec`s for `parents`, `children`, `scope_types`, `lengths` (per-context `Vec<Length>` for sequences), `pcs`, `tempos`, `bpms`, `registers`, `velocities` (also per-context `Vec<Velocity>`), `programs`, and `bindings` (per-context `BTreeMap<Ident, Exp>`). A `stack` of pending `(Exp, Ctx)` pairs holds prefixes waiting for an operand, and a `garbage` list tracks discarded contexts. Children inherit tempo from their parent at creation time, and scope type (`Sequence` / `Stack` / `Set`) records how each compound's contents will be flattened into MIDI events. Fixed durations (`5'2"`) become absolute microsecond lengths; fractional durations (`d4`, `d3:2`) are resolved against the parent's remaining length and current tempo.

### Codegen types

`compiler/codegen.rs` defines the MIDI-domain types used downstream of the composer: `PPQ` (25200 ticks per quarter — chosen for high divisibility), `MicroSeconds`, `Length`, `Mpb` (microseconds per beat — the internal tempo representation), `Velocity`, `Pc` (`Class(i8) | None`), `Prog`, `Register` (`Reg(i8) | None`), `Context` (a flat per-`Ctx` record of all parallel-vec fields), and an `Instruction` enum with `Midi(MidiMessage)` / `Meta(MetaMessage)` variants. Helpers convert between time domains: `to_length` (fractional → microseconds), `duration_to_micros` (fixed → microseconds), `length_to_ticks` (microseconds → PPQ ticks at a given tempo).

### Scheduler

`compiler/scheduler.rs` consumes the composed `State` and emits a `midly::Smf`. `schedule_context` walks the context tree depth-first, threading a global `clock` (in PPQ ticks). For each note it inserts a `NoteOn`/`NoteOff` pair (encoded as a zero-velocity `NoteOn`) into a `BTreeMap<Ticks, Vec<Instruction>>` keyed by absolute time. Tempo changes and program changes are dropped into the same map as `MetaMessage::Tempo` and `MidiMessage::ProgramChange`. `render_tracks` then iterates the map in time order, converting absolute ticks into delta-time `TrackEvent`s for a single-track SMF. The pipeline runs end-to-end: `cargo run -- --input <name>` reads `<name>.dsch` and writes `<name>.mid`.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete |
| Composer — fixed/fractional durations, tuplets | Working |
| Composer — `pc` (absolute & relative), `reg` | Working |
| Composer — scope types (Sequence, Stack, Set) | Working |
| Composer — tempo (`<n> bpm`) | Working (scalar form) |
| Composer — declarations (`ident: exp`) | Initial dispatch in place; semantics partial |
| Composer — dynamics, frequency, amplitude (`A`), bare suffix forms | Stubbed |
| Composer — infix (`:`, `><`, `..`, `<`, `>`) | Stubbed |
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

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

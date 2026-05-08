# DSCH

A compiler for an experimental music composition DSL — parses `.dsch` source and lowers it to MIDI.

> **Status: Work in progress / research.** Active exploration — architecture and API subject to change.

## Overview

`DSCH` reads `.dsch` files in a custom expression-oriented DSL and compiles them to a tree of scoped musical contexts which is then rendered to a Standard MIDI File. It was previously the `compiler` crate inside the `sound-studies` workspace and has been spun out into its own self-contained crate.

## Layout

```
dsch/
├── Cargo.toml
├── grammar.pest               # Active PEG grammar
├── prototype.dsch              # Reference composition
├── test.dsch                   # Minimal smoke-test input
└── src/
    ├── main.rs                # Entry point: read .dsch, parse, compose
    ├── pest_parser.rs         # Pest PEG parser → AST
    └── compiler/
        ├── mod.rs
        ├── ast.rs             # AST type definitions
        ├── functional.rs      # Monad / Functor / Combinator scaffolding
        ├── composer.rs        # Tree-walking compiler → scoped contexts
        ├── codegen.rs         # MIDI-domain primitives (PPQ, Length, Mpb, Velocity, Ctx, …)
        └── renderer.rs        # Context tree → MIDI tracks (in progress)
```

## The `.dsch` DSL

`.dsch` is an expression-oriented language for algorithmic music composition drawing on two programming paradigms:

- **Concatenative** — at the surface level, meaning arises from juxtaposition. Placing expressions next to each other implicitly threads a musical context from left to right, with no explicit binding operator. `d4 (120 144 60 120) bpm` is three tokens in sequence: a duration that sets context, a list that supplies values, and a suffix keyword that consumes them. This is the same model used by languages like Forth and Joy.
- **Functional** — internally, AST nodes compose through the `Monad<Exp>::bind` operator in `functional.rs`, and the compiler pipeline is a left-to-right fold of those binds. Concatenative languages often have this property: Joy's formal semantics are defined entirely in terms of function composition. A planned extension will bring Haskell-style syntax to the DSL itself — allowing users to define higher-kinded types, custom data types, and functions directly in `.dsch` files. This would make the language self-extensible: composers could build and share libraries of reusable abstractions (scales, rhythmic patterns, transformations) in the same language they write music in.

Programs are nested expressions that specify duration, tempo, pitch, register, dynamics, and rhythm.

**Grouping semantics:**

| Syntax | Meaning |
|--------|---------|
| `(...)` | Sequence — expressions play in order |
| `{...}` | Stack — expressions play simultaneously |
| `[a, b, c]` | Set — comma-separated unordered collection |
| `a:b:c` | Ratio — proportional time subdivision |

**Primitives (prefix keywords):**

| Token | Meaning |
|-------|---------|
| `d<n>` | Fractional duration (e.g. `d4` = quarter note, `d8` = eighth note) |
| `d<a>:<b>` | Tuplet duration — `a` notes in the time of `b` |
| `5'` `2"` `5'2"` | Fixed duration — minutes, seconds, or combined |
| `pc` | Pitch class |
| `reg` | Register (octave) |
| `~` | Rest |

**Primitives (suffix keywords):**

| Token | Meaning |
|-------|---------|
| `bpm` | Tempo in beats per minute |
| `A` | Amplitude (velocity) |
| `Hz` | Frequency suffix (e.g. `440Hz`) |

**Dynamics:**

| Token | Meaning |
|-------|---------|
| `ppp` `pp` `p` `mp` `mf` `f` `ff` `fff` | Discrete dynamic levels |
| `<` | Crescendo (continuous increase) |
| `>` | Decrescendo (continuous decrease) |

**Operators:**

| Token | Meaning |
|-------|---------|
| `:`   | Ratio separator |
| `><`  | Intercalate — interleave two sequences |
| `...` | Range |

**Bindings:**

`ident: exps` declares a named expression. Identifiers are alphanumeric (with underscores) and may carry `'` (prime) suffixes — useful for related variants like `theme`, `theme'`, `theme''`.

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
    │  Composer — monadic fold over AST,
    │  producing a tree of scoped contexts:
    │  duration · pitch class · tempo · register · velocity · program
    ▼
  State (Ctx arena: parents · scope_types · lengths · pcs · tempos · bpms · registers · velocities · programs · children)
    │
    │  Renderer (in progress)
    ▼
  MIDI (midly::Smf, PPQ = 25200)
```

### Parser

A Pest PEG grammar (`grammar.pest`) drives a hand-written recursive-descent walker in `pest_parser.rs` that builds the typed AST defined in `compiler/ast.rs`.

### Composer

The composer in `compiler/composer.rs` is structured as a left-to-right fold over expressions. Each pair of adjacent expressions is reduced through `compose_exps`, which dispatches via `Monad::bind` to specialised composers per AST shape (`compose_simple`, `compose_compound`, `compose_scalar`, `compose_primitive`, `compose_duration`, `compose_fractional`, `compose_prefix`, `compose_suffix`, …).

State is an arena rather than a recursive structure: `Ctx::Id(usize)` indexes into parallel `Vec`s for `parents`, `scope_types`, `lengths`, `pcs`, `tempos`, `bpms`, `registers`, `velocities`, `programs`, and `children`. Children inherit tempo from their parent at creation time, and scope type (`Sequence` / `Stack` / `Set`) records how each compound's contents will be flattened into MIDI tracks. Fixed durations (`5'2"`) become absolute microsecond lengths; fractional durations (`d4`, `d3:2`) are resolved against the parent's remaining length and current tempo.

### Codegen primitives

`compiler/codegen.rs` defines the MIDI-domain types used downstream of the composer: `PPQ` (25200 ticks per quarter — chosen for high divisibility), `MicroSeconds`, `Length`, `Mpb` (microseconds per beat — the internal tempo representation), `Velocity`, `Pc`, `Prog`, and an `Instruction` enum that wraps `midly::MidiMessage` and `MetaMessage`. Helpers convert fractional/fixed durations to microseconds (`to_length`, `duration_to_micros`).

### Renderer

`compiler/renderer.rs` consumes the composed `State` and emits a `midly::Smf`. It is the current focus of work — the scaffolding (`Renderer::render`, instruction/event buffers per track) is in place, and the next step is walking the context tree to emit time-stamped `TrackEvent`s.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete |
| Composer — fixed/fractional durations, tuplets | Working |
| Composer — `pc` (absolute & relative), `reg` | Working |
| Composer — scope types (Sequence, Stack, Set) | Working |
| Composer — `bpm`, `A`, `Hz`, `dur`, `rest` | Stubbed (`todo!()`) |
| Composer — operators (`:`, `><`, `...`) | Stubbed |
| Composer — identifier bindings | Stubbed |
| Renderer → MIDI | In progress |
| Dynamics (`ppp` … `fff`, `<`, `>`) | Parsed; not yet composed |

## Running

```bash
# Parse test.dsch and run it through the composer
cargo run
```

The default entry reads `test.dsch` from the working directory.

## Built with

- [`pest`](https://github.com/pest-parser/pest) — PEG parser generator
- [`midly`](https://github.com/kovaxis/midly) — MIDI file I/O
- [`primal`](https://github.com/huonw/primal) — prime-number utilities (used by the ratio/tuplet logic)
- [`num-integer`](https://github.com/rust-num/num-integer) — gcd / integer utilities

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

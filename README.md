# DSCH

A compiler for a structured music composition DSL — parses `.dsch` source and lowers it to MIDI.

> **Status: Work in progress / research.** Active development — architecture and API subject to change. The composer and scheduler were rewritten from scratch in this revision. `1.dsch` is the current smoke test and does compile end-to-end, though a timing bug in the output is still being tracked down; see [Known issues](#known-issues) for the rest.

## Overview

`DSCH` reads `.dsch` files in a custom expression-oriented DSL, compiles them to a tree of scoped musical contexts, schedules those contexts as time-stamped MIDI events, and writes a Standard MIDI File. It was previously the `compiler` crate inside the `sound-studies` workspace and has been spun out into its own self-contained crate.

## Layout

```
dsch/
├── Cargo.toml
├── grammar.pest               # Active PEG grammar
├── test.dsch, 0.dsch, 1.dsch,  # .dsch inputs (mixed grammar generations — see Known issues)
│   2.dsch, 3.dsch, merge.dsch,
│   prototype.dsch
└── src/
    ├── main.rs                # Entry point: CLI → parse → compose → schedule → MIDI
    ├── lib.rs                 # Library crate root — re-exports compiler modules
    ├── pest_parser.rs         # Pest PEG parser → AST
    ├── track.rs               # Unused scaffolding (`Seq<T>` layer builder) — not wired into lib.rs
    └── compiler/
        ├── mod.rs
        ├── ast.rs             # AST type definitions
        ├── functional.rs      # Monad / Functor / Combinator scaffolding
        ├── composer.rs        # Fold over AST → scoped-context arena (rewritten, see below)
        ├── codegen/
        │   ├── mod.rs         # MIDI-domain types (PPQ, Length, Mpb, Velocity, Ctx, Pc, Register, Prog, Instruction, …)
        │   ├── state.rs       # `State` — context arena + the new post-composition `sequence` merge pass
        │   └── utils.rs       # `length_to_ticks`, `gcd`, terminal debug-print helpers (`out`, `graph`, `print_state`)
        └── scheduler.rs       # Timeline → time-stamped MIDI track
```

`codegen.rs` (a single 618-line file) has been split into a `codegen/` module. The most significant change is that `State` — the context arena that used to live in `composer.rs` — moved to `codegen::state::State`, and it now owns the post-composition sequencing pass (`State::sequence`) that used to be composer-side (`sequence_children`). Composer and scheduler both consume this shared `State`.

## The `.dsch` DSL

`.dsch` is an expression-oriented language for algorithmic music composition drawing on two programming paradigms:

- **Concatenative** — at the surface level, meaning arises from juxtaposition. Placing expressions next to each other implicitly threads a musical context from left to right, with no explicit binding operator. This is the same model used by languages like Forth and Joy.
- **Functional** — internally, AST nodes compose through the `Monad<Exp>::bind` operator in `functional.rs`. A planned extension will bring Haskell-style syntax to the DSL itself — allowing users to define higher-kinded types, custom data types, and functions directly in `.dsch` files.

Programs are nested expressions that specify duration, tempo, pitch, register, dynamics, program (instrument), and rhythm.

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
| `d<a>/<b>` | Rational duration — new this revision, e.g. `d3/16` |
| `5'` `2"` `5'2"` | Fixed duration — minutes, seconds, or combined |
| `<n> bpm` | Tempo |
| `<n>Hz` | Frequency |
| `#<n>` | MIDI program (instrument) change — new this revision, e.g. `#48` for strings |
| `~` | Rest — moved from a prefix token to a scalar this revision |
| `ppp` `pp` `p` `mp` `mf` `f` `ff` `fff` | Discrete dynamic level |

> **Note:** `d<n>` duration forms are atomic in the grammar — no whitespace is allowed between `d` and the number. `d4` parses as a duration scalar; `d 4` would parse `d` as an identifier.

**Prefix:**

| Token | Meaning |
|-------|---------|
| `pc` | Pitch class — bind the following value(s) as pitch classes |
| `@` | Register (octave) — **changed this revision**, was the word `reg` |
| `d` | Duration prefix (planned — not yet dispatched in the composer) |

**Suffix:**

| Token | Meaning |
|-------|---------|
| `bpm` | Apply a tempo to the preceding compound |
| `Hz`  | Apply a frequency to the preceding compound |
| `^`   | Amplitude (velocity) — **changed this revision**, was `A` |

**Infix:**

| Token | Meaning |
|-------|---------|
| `:`   | Ratio separator |
| `><`  | Intercalate — interleave two sequences |
| `..`  | Range — inclusive discrete enumeration between two values |
| `<`   | Interpolate upward |
| `>`   | Interpolate downward |
| `+` `-` `*` `/` | Arithmetic on numeric operands |

**Bindings:**

`ident: exp` declares a named expression. Identifiers are alphanumeric (with underscores) and may carry `'` (prime) suffixes. Declarations moved this revision from being a `Compound` variant to a first-class `Simple::Decl`, matching how the grammar already treated `decl` as an alternative of `simple` rather than `compound`.

**Comments:** `-- comment text` runs to end of line. Comments are enabled this revision (`COMMENT` was previously commented out in the grammar).

### Example (new `@` / `#` syntax, from `1.dsch`)

```
-- WORDS --
Strings: #48
Violin: #40
Cello: #42
C#: 1 D: 2 E: 4 F#: 6 G: 7 A: 9 B: 11
Piano: #0
ost: (d4 @3 D @2 A B F# G D G A )
ln1: (d4 @5 F# E D C# @4 B A B @5 C#)

-- COMPOSITION --
50 bpm

mf ost

{
  mf
  ost
  ln1
}
```

This binds named voices (`ost`, `ln1`, …) built from duration + register + pitch-class tokens, then layers them into `{...}` stacks — a chorale-style texture growing one voice at a time. Note this file uses bindings (`ident: exp`) as short, single-letter-style pitch names (`C#: 1`) rather than defining a scale DSL; `C#` here is just an identifier bound to the pitch class `1`.

## Compiler pipeline

```
.dsch source
    │
    │  Pest PEG parser (grammar.pest)
    ▼
  Program AST
    │
    │  Composer — recursive descent over AST, driven by `State` as an iterator
    │  (State::next() pulls the next Exp), building a tree of scoped contexts:
    │  duration · pitch class · tempo · register · velocity · program · bindings
    ▼
  State (Ctx arena: parents · children · scopes · pcs · lengths · velocities ·
                    registers · tempos · programs · bindings)
    │
    │  State::sequence — post-composition pass. Walks Stack/Sequence structure
    │  and merges concurrent Sequence children into synthetic Stack nodes
    │  (`combine_sequences`), producing a flat `timeline: Vec<(u64, Ctx)>`
    │  of time-stamped contexts.
    ▼
  Scheduler — walks the timeline in order, forwarding the clock between
    entries and emitting NoteOn/NoteOff, ProgramChange, and Tempo events
    into a BTreeMap<Ticks, Vec<Instruction>>
    ▼
  MIDI (midly::Smf, PPQ = 25200)
```

### Parser

A Pest PEG grammar (`grammar.pest`) drives a hand-written recursive-descent walker in `pest_parser.rs` that builds the typed AST defined in `compiler/ast.rs`. The AST is rooted at `Exp = Compound | Simple | Noop | EOI` (renamed from `EOS` this revision), with `Simple = Prefix | Scalar | Infix | Suffix | Decl | Ident`. `Compound` is no longer boxed (`Exp::Compound(Compound)`, not `Exp::Compound(Box<Compound>)`).

### Composer — rewritten this revision

`compiler/composer.rs` shrank from ~2,900 lines to 353. The old design pushed the whole program onto explicit `rhs_stack`/`lhs_stack` vectors and drained them through a large `combine` dispatcher with dedicated stack-merging helpers (`consume_right_assoc_exps`, `consume_simples`, `merge_sequences`, `fit`, `expand_context`, …). That machinery has been replaced:

- `State` itself now implements `Iterator<Item = Exp>` over the current expression list (`state.exps`, `state.index`); `state.next()` pulls the next token, and `state.set_exps` swaps in a new list (e.g. when recursing into a compound), returning the old one so it can be restored on the way back out (`compose_compound`).
- `compose_exp` dispatches on `Exp` (`Compound` / `Simple` / `Noop` / `EOI`); `compose_simple` dispatches on `Simple` (`Prefix` / `Infix` / `Suffix` / `Decl` / `Scalar` / `Ident`) — a direct match instead of a stack-driven combine.
- Polyphonic merging (what used to be `sequence_children`/`merge_sequences` in composer.rs) has moved to `State::sequence` / `State::combine_sequences` in `codegen/state.rs`, run once at the end of `compose_program`, after the full tree is built.

**Composer coverage is currently partial** — several branches are `todo!()` stubs left mid-refactor:

| Composer path | Status |
|---|---|
| Bare pitch-class numbers (`0 3 7 10`), tempo (`bpm`), tuplet/rational/absolute durations, dynamics, program change (`#n`), register scalar (`@n`), declarations/bindings | Working |
| `pc` prefix (any argument form) | `todo!()` — unimplemented this revision |
| `reg`/`@` prefix with a compound argument (e.g. `@ (4 5)`) | `todo!()` |
| `compose_infix` (`:`, `><`, `..`, `<`, `>`, `+`, `-`, `*`, `/`) | `todo!()` |
| `compose_suffix` (`^`, `bpm`, `Hz` as bare suffixes) | `todo!()` |
| Fixed durations (`5'`, `5'2"`) | `todo!()` |
| Relative numbers (`+n`, `-n`) | `todo!()` |

### Codegen types

`codegen/mod.rs` defines the MIDI-domain types: `PPQ` (25200 ticks per quarter), `Length`, `Mpb` (microseconds per beat), `Velocity`, `Pc` (`Class(u8) | None`), `Prog` (MIDI program number, new this revision), `Register` (`Reg(i8) | None`), `Ctx` (arena index; `Ctx::Root` now compares equal to `Ctx::Id(0)`), and `Instruction` (`Midi` / `Meta`). A `Data`/`Type` pair of enums (`Pc`/`Length`/`Velocity`/`Program`/`Tempo`/`Register`) backs a small type-erased `State::take_last::<T>()` accessor via `TypeId`.

### `State` (`codegen/state.rs`) — new this revision

`State` owns the context arena (`parents`, `children`, `scopes`, and parallel per-field vectors for `pcs`/`lengths`/`velocities`/`registers`/`tempos`/`programs`, plus per-context `bindings`) and now also the `timeline: Vec<(u64, Ctx)>` and `playhead: Length` used by the merge pass. Fields are appended (not overwritten) as the composer walks the tree — `add_pc`, `add_length`, etc. — with `pad`/`cycle_fill`/`resize` helpers to align differently-sized parallel arrays before merging.

`State::sequence(ctx)` recurses down `Scope::Sequence` nodes and, on reaching a `Scope::Stack` node whose children are `Scope::Sequence`, calls `combine_sequences` on that set of children. `combine_sequences` cycle-fills each sequence to a common total length, then walks all of them in lock-step by their per-event lengths (a manual playhead loop, replacing the old GCD-stepped loop), allocating a fresh `Scope::Stack` child per playhead tick and appending a `(tick, ctx)` pair to `state.timeline()` for each one.

### Scheduler — rewritten this revision

`compiler/scheduler.rs` no longer recurses the full context tree at schedule time. `schedule()` now iterates `state.timeline()` directly: for each `(t, ctx)` entry it converts `t` (microseconds) to ticks at the context's current tempo, computes `delta_ticks` from the scheduler's current position, calls `scheduler.forward(delta_ticks)`, and dispatches `schedule_context(ctx, ...)` — which now only has two live branches, `Scope::Sequence` (walk pcs/lengths/velocities/registers/tempos/programs in lock-step, emitting a note and advancing the clock after each) and `Scope::Stack` (emit all pcs simultaneously, using the minimum length). `Scheduler::set_program`/`set_tempo` emit a `ProgramChange`/`Tempo` meta event only when the value actually changes.

## Known issues

- **Timing bug in scheduled output.** `1.dsch` — the current smoke test, using the new `@`/`#` syntax — composes and schedules to a `.mid` file successfully, but the author is still tracking down a timing discrepancy in the result. Likely areas: the `delta_ticks`/`forward` bookkeeping in `schedule()` ([scheduler.rs](src/compiler/scheduler.rs)) that steps the clock between `state.timeline()` entries, or the playhead loop in `State::combine_sequences` ([state.rs](src/compiler/codegen/state.rs)) that decides where each merged Stack node lands on the timeline.
- **The debug graph visualizer is fragile and shouldn't be relied on.** `codegen/state.rs:718` calls `graph(self, self.parent(ctx))` **unconditionally** inside the `combine_sequences` merge loop — every other call site for `graph`/`print_state` in the codebase is commented out, but this one is live. `graph()` sizes its ASCII layout off `crossterm::terminal::size()` and feeds it through `rust-sugiyama`; in a non-interactive or narrow-terminal environment (piped output, CI, some sandboxes) this reliably panics — I hit `WouldBlock` from `size()`, a `rust-sugiyama` cyclic-graph assertion, and a `column_width` integer underflow/capacity overflow across different runs and build profiles here, none of which reproduced for the author running interactively. Since it's wired into the merge hot path rather than gated behind a flag, anyone running `.dsch` files non-interactively (CI, scripts, narrow terminals) should expect it to crash. Worth removing or feature-gating.
- **`test.dsch`, `merge.dsch`, `3.dsch`** (older `reg <n> pc (...)` syntax) stack-overflow. `reg` is no longer a grammar keyword — the `reg` prefix token now requires `@` — so bare `reg` text falls through to the generic `ident` rule and composes as an unbound, silently-swallowed identifier, while `pc` still parses as `Prefix::Pc`, which is `todo!()` (see composer table above). These files need to be ported to `@`-syntax and rewritten to avoid `pc` prefix usage, or `pc` needs an implementation, before they're useful smoke tests again.
- **`prototype.dsch`** no longer parses at all — it predates this revision's grammar (still uses the old `reg` keyword and `5'` fixed-duration usage in a position the current grammar doesn't accept at top level). It documents the target long-term DSL shape but is not currently runnable.
- `src/track.rs` (`Seq<T>` layer builder) is not referenced from `lib.rs`/`mod.rs` and appears to be disconnected scaffolding.
- `num-rational` is listed as a dependency but is unused — the new `Rational` duration form (`d<a>/<b>`) is computed directly as `f64` in `compose_fractional_duration` rather than through the crate.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete for the syntax exercised in `0.dsch`/`1.dsch`/`2.dsch`; `prototype.dsch`'s older syntax no longer parses |
| Composer — bare pitch classes, tuplet/rational/absolute durations, tempo, program change, `@` register, bindings | Working |
| Composer — `pc` prefix | Not implemented (`todo!()`) |
| Composer — infix (`:`, `><`, `..`, `<`, `>`, `+`, `-`, `*`, `/`) | Not implemented (`todo!()`) |
| Composer — suffix (`^`, bare `bpm`/`Hz`), fixed durations, relative numbers | Not implemented (`todo!()`) |
| `State::sequence` polyphonic merge → timeline | Working (`1.dsch`) — the live debug `graph()` call in the merge loop is a separate reliability risk, see Known issues |
| Scheduler → MIDI | Working (`1.dsch` produces a `.mid`), but a timing bug in the scheduled output is still being tracked down — see Known issues |

## Running

```bash
# Read <name>.dsch from the working directory, write <name>.mid alongside it
cargo run -- --input <name>
```

For example, `cargo run -- --input 1` parses `1.dsch`, composes it, sequences it, schedules MIDI events, and writes `1.mid`. `0.dsch`/`2.dsch`/`3.dsch` are older or in-progress fixtures — see [Known issues](#known-issues) for which composer paths they still need.

## Built with

- [`pest`](https://github.com/pest-parser/pest) — PEG parser generator
- [`pest_derive`](https://github.com/pest-parser/pest) — derive macro for typed Pest grammars
- [`midly`](https://github.com/kovaxis/midly) — MIDI file I/O
- [`clap`](https://github.com/clap-rs/clap) — CLI argument parsing (`--input <name>`)
- [`num-rational`](https://github.com/rust-num/num-rational) — declared dependency; not currently used (see Known issues)
- [`crossterm`](https://github.com/crossterm-rs/crossterm) — cross-platform terminal control (used for debug/progress output; see Known issues for a case where this is load-bearing for a crash)
- [`colonnade`](https://github.com/dfhoughton/colonnade) — aligned terminal column formatting for state inspection
- [`colprint`](https://crates.io/crates/colprint) — coloured terminal output helpers
- [`rust-sugiyama`](https://github.com/paddison/rust-sugiyama) — Sugiyama-style layered graph layout (context-tree visualisation)

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

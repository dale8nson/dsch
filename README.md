# DSCH

A compiler for a structured music composition DSL — parses `.dsch` source and lowers it to MIDI.

> **Status: Work in progress / research.** Active development — architecture and API subject to change. The composer and scheduler were rewritten from scratch in this revision; most checked-in `.dsch` fixtures compile and schedule to MIDI, including the timing fix for stacked sequences of differing lengths. Expressions built on relative numbers (`+n`, `-n`) still don't compile (`compose_relative` is `todo!()`), and listening back to the more complex pieces has turned up rhythmic discrepancies the GCD-step fix doesn't fully account for — see [Known issues](#known-issues) for what's still incomplete.

## Overview

`DSCH` reads `.dsch` files in a custom expression-oriented DSL, compiles them to a tree of scoped musical contexts, schedules those contexts as time-stamped MIDI events, and writes a Standard MIDI File. It was previously the `compiler` crate inside the `sound-studies` workspace and has been spun out into its own self-contained crate.

## Layout

```
dsch/
├── Cargo.toml
├── grammar.pest               # Active PEG grammar
├── 0.dsch, 1.dsch, 2.dsch,     # .dsch inputs on the current `@`-based grammar
│   3.dsch, test.dsch,
│   merge.dsch, timing.dsch    # timing.dsch — regression fixture for the stacked-sequence timing fix
├── prototype.dsch              # predates this revision's grammar — see Known issues
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

Pitch classes are bare numbers with no prefix (`0 3 7 10`) — the `pc` prefix keyword has been **removed from the grammar** this revision. It worked in the composer prior to this revision's rewrite, but was left as a `todo!()` stub when `composer.rs` was rewritten from scratch (see below); rather than re-implementing it, it's been dropped from the grammar and every fixture now expresses pitch classes as plain numbers instead.

**Prefix:**

| Token | Meaning |
|-------|---------|
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

### Example (from `1.dsch`)

```
-- WORDS --
Strings: #48
Violin I: #40
Cello: #42
C#: 1 D: 2 E: 4 F#: 6 G: 7 A: 9 B: 11
Piano: #0
Marimba: #13
ost: (d4 @3 D @2 A B F# G D G A )
ost8ve: (d4 @2 D @1 A B F# G D G A )
ln1: (d4 @5 F# E D C# @4 B A B @5 C#)

-- COMPOSITION --
50 bpm

Marimba

{
  ppp ost
  ppp ost8ve
}

{
  ppp ost
  ppp ost8ve
  pp ln1
}
```

This binds named voices (`ost`, `ost8ve`, `ln1`, …) built from duration + register + pitch-class tokens, then layers them into `{...}` stacks — a chorale-style texture growing one voice at a time, doubled an octave down by `ost8ve`. Note this file uses bindings (`ident: exp`) as short, single-letter-style pitch names (`C#: 1`) rather than defining a scale DSL; `C#` here is just an identifier bound to the pitch class `1`. A bare `Marimba` (a `Prog` binding) sets the instrument for everything that follows.

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
| Bare pitch-class numbers (`0 3 7 10`), tempo (`bpm`), tuplet/rational/absolute durations, dynamics, program change (`#n`), register scalar (`@n`), declarations/bindings | Working — exercised by every current fixture |
| `@` (register) prefix with a compound argument (e.g. `@ (4 5)`) | `todo!()` |
| `compose_infix` (`:`, `><`, `..`, `<`, `>`, `+`, `-`, `*`, `/`) | `todo!()` |
| `compose_suffix` (`^`, `bpm`, `Hz` as bare suffixes) | `todo!()` |
| Fixed durations (`5'`, `5'2"`) | `todo!()` |
| Relative numbers (`+n`, `-n`) | `todo!()` |

`compose_pc`/`Prefix::Pc` in `ast.rs` are now unreachable dead code — the grammar no longer produces a `pc` token (see above), so nothing constructs `Prefix::Pc` any more.

### Codegen types

`codegen/mod.rs` defines the MIDI-domain types: `PPQ` (25200 ticks per quarter), `Length`, `Mpb` (microseconds per beat), `Velocity`, `Pc` (`Class(u8) | None`), `Prog` (MIDI program number, new this revision), `Register` (`Reg(i8) | None`), `Ctx` (arena index; `Ctx::Root` now compares equal to `Ctx::Id(0)`), and `Instruction` (`Midi` / `Meta`). A `Data`/`Type` pair of enums (`Pc`/`Length`/`Velocity`/`Program`/`Tempo`/`Register`) backs a small type-erased `State::take_last::<T>()` accessor via `TypeId`.

### `State` (`codegen/state.rs`) — new this revision

`State` owns the context arena (`parents`, `children`, `scopes`, and parallel per-field vectors for `pcs`/`lengths`/`velocities`/`registers`/`tempos`/`programs`, plus per-context `bindings`) and now also the `timeline: Vec<(u64, Ctx)>` and `playhead: Length` used by the merge pass. Fields are appended (not overwritten) as the composer walks the tree — `add_pc`, `add_length`, etc. — with `pad`/`cycle_fill`/`resize` helpers to align differently-sized parallel arrays before merging.

`State::sequence(ctx)` recurses down `Scope::Sequence` nodes and, on reaching a `Scope::Stack` node whose children are `Scope::Sequence`, calls `combine_sequences` on that set of children. `combine_sequences` cycle-fills each sequence to a common total length, then walks all of them in lock-step, allocating a fresh `Scope::Stack` child each time any input sequence has an event due and appending a `(tick, ctx)` pair to `state.timeline()` for it. The playhead itself now advances by a fixed `step` (the GCD of all the input sequences' event lengths) once per outer-loop iteration, rather than by the length of whatever was last merged — this was the fix for the timing bug where stacked sequences of differing lengths picked up an extra gap between events. Once a `Scope::Sequence` node has been folded into the timeline this way, `sequence` also prunes its arena entry (`bindings`/`tempos`/`velocities`/`lengths`/`pcs`/`registers`/`programs`/`children`) and unlinks it from its parent's children list, so the tree doesn't accumulate now-redundant intermediate nodes.

### Scheduler — rewritten this revision

`compiler/scheduler.rs` no longer recurses the full context tree at schedule time. `schedule()` now iterates `state.timeline()` directly: for each `(t, ctx)` entry it converts `t` (microseconds) to ticks at the context's current tempo, computes `delta_ticks` from the scheduler's current position, calls `scheduler.forward(delta_ticks)`, and dispatches `schedule_context(ctx, ...)` — which now only has two live branches, `Scope::Sequence` (walk pcs/lengths/velocities/registers/tempos/programs in lock-step, emitting a note and advancing the clock after each) and `Scope::Stack` (emit all pcs simultaneously, using the minimum length). `Scheduler::set_program`/`set_tempo` emit a `ProgramChange`/`Tempo` meta event only when the value actually changes.

## Known issues

- **Rhythmic discrepancies in more complex pieces.** The `combine_sequences` GCD-step fix (see below) resolves the extra-gap bug for the simple stacked-sequence cases it targeted, but listening back to more elaborate pieces — deeper nesting, more voices, mixed sequence/stack shapes — has turned up timing that doesn't sound right. Root cause not yet isolated; likely another edge case in the merge/cycle-fill logic rather than a fault in the scheduler itself, since the scheduler just walks whatever timeline `State::sequence` hands it.
- **The debug graph visualizer is fragile and shouldn't be relied on.** `codegen/state.rs` calls `graph(self, Ctx::Root)` **unconditionally**, twice per invocation of the `combine_sequences` merge loop — every other call site for `graph`/`print_state` in the codebase is commented out, but these are live. `graph()` sizes its ASCII layout off `crossterm::terminal::size()` and feeds it through `rust-sugiyama`; in a non-interactive or narrow-terminal environment (piped output, CI, some sandboxes) this can panic — I hit `WouldBlock` from `size()`, a `rust-sugiyama` cyclic-graph assertion, and a `column_width` integer underflow/capacity overflow across different runs here, none of which reproduce when running interactively in a normal terminal. Since it's wired into the merge hot path rather than gated behind a flag, running `.dsch` files non-interactively (CI, scripts, some terminal emulators) is at risk of hitting this. Worth removing or feature-gating rather than relying on terminal geometry.
- **`prototype.dsch`** doesn't parse. `scalar = { dynamic | frequency | tempo | pure | duration | rest | prog }` tries `pure` (a bare number) before `duration` (which is where `fixed`/minutes-seconds durations live), so on input like `5'` the parser commits to reading `5` as a plain number via `pure` and never backtracks to try `fixed` — the trailing `'` then has nowhere to go and the whole parse fails at that point. `5'`/`2"` fixed durations are effectively unreachable in the current grammar whenever they start with digits, independent of the file's other issue (it also still uses the pre-`@` `reg` keyword further in, and `composed_fixed_duration` in the composer is `todo!()` regardless). It documents the target long-term DSL shape but is not currently runnable.
- **`@` (register) prefix with a compound argument** (e.g. `@ (4 5)`, cycling the register per note) is still `todo!()` in the composer — every current fixture only uses `@n` with a single scalar register.
- `src/track.rs` (`Seq<T>` layer builder) is not referenced from `lib.rs`/`mod.rs` and appears to be disconnected scaffolding.
- `num-rational` is listed as a dependency but is unused — the `Rational` duration form (`d<a>/<b>`) is computed directly as `f64` in `compose_fractional_duration` rather than through the crate.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete for the current `@`-based grammar; `prototype.dsch`'s older syntax no longer parses |
| Composer — bare pitch classes, tempo, tuplet/rational/absolute durations, dynamics, program change, `@n` register, bindings | Working |
| Composer — `@` register with a compound argument | Not implemented (`todo!()`) |
| Composer — infix (`:`, `><`, `..`, `<`, `>`, `+`, `-`, `*`, `/`) | Not implemented (`todo!()`) |
| Composer — suffix (`^`, bare `bpm`/`Hz`), fixed durations, relative numbers | Not implemented (`todo!()`) |
| `State::sequence` polyphonic merge → timeline | Working for the simple cases the GCD-step fix targeted; rhythmic discrepancies remain in more complex pieces (deeper nesting, more voices, mixed sequence/stack shapes) — see Known issues. The live debug `graph()` call in the merge loop is a separate reliability risk, also see Known issues |
| Scheduler → MIDI | Working — most checked-in fixtures compose, sequence, and schedule to a `.mid` file, modulo the rhythmic discrepancies above |

## Running

```bash
# Read <name>.dsch from the working directory, write <name>.mid alongside it
cargo run -- --input <name>
```

For example, `cargo run -- --input 1` parses `1.dsch`, composes it, sequences it, schedules MIDI events, and writes `1.mid`. All of `0`/`1`/`2`/`3`/`test`/`merge`/`timing` are current fixtures on the `@`-based grammar; `prototype` is not (see [Known issues](#known-issues)).

## Built with

- [`pest`](https://github.com/pest-parser/pest) — PEG parser generator
- [`pest_derive`](https://github.com/pest-parser/pest) — derive macro for typed Pest grammars
- [`midly`](https://github.com/kovaxis/midly) — MIDI file I/O
- [`clap`](https://github.com/clap-rs/clap) — CLI argument parsing (`--input <name>`)
- [`num-rational`](https://github.com/rust-num/num-rational) — declared dependency; not currently used (see Known issues)
- [`crossterm`](https://github.com/crossterm-rs/crossterm) — cross-platform terminal control (used for debug/progress output; see Known issues — the debug graph visualizer's dependency on terminal size is a reliability risk outside a normal interactive terminal)
- [`colonnade`](https://github.com/dfhoughton/colonnade) — aligned terminal column formatting for state inspection
- [`colprint`](https://crates.io/crates/colprint) — coloured terminal output helpers
- [`rust-sugiyama`](https://github.com/paddison/rust-sugiyama) — Sugiyama-style layered graph layout (context-tree visualisation)

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

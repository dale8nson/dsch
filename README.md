# DSCH

A compiler for a structured music composition DSL — parses `.dsch` source and lowers it to MIDI.

> **Status: Work in progress / research.** Active development — architecture and API subject to change. This revision is a substantial follow-on to the composer/scheduler rewrite: the grammar gained repetition, relative register offsets, and multi-point dynamic/duration ramps, and the codegen layer gained a generic `Interpolant`/`Normalize` trait system plus a `Thunk`-based deferred-effect mechanism. Fifteen of the sixteen checked-in `.dsch` fixtures (`0`–`10`, `test`, `merge`, `timing`) compile and schedule to MIDI; `prototype.dsch` does not, since its `5'`/`2"` fixed-duration syntax isn't implemented yet under the current grammar (see [Known issues](#known-issues)). The author's own testing has also turned up rhythmic discrepancies in more elaborate pieces that the earlier timing fix doesn't fully cover, and relative-number expressions in some contexts (e.g. `+n bpm`) are still unimplemented — see below.

## Overview

`DSCH` reads `.dsch` files in a custom expression-oriented DSL, compiles them to a tree of scoped musical contexts, schedules those contexts as time-stamped MIDI events, and writes a Standard MIDI File. It was previously the `compiler` crate inside the `sound-studies` workspace and has been spun out into its own self-contained crate.

## Layout

```
dsch/
├── Cargo.toml
├── grammar.pest                # Active PEG grammar
├── 0.dsch, 0.5.dsch, 1.dsch,    # .dsch inputs on the current grammar
│   2.dsch, 3.dsch, 4.dsch,
│   5.dsch, 6.dsch, 7.dsch,
│   8.dsch, 9.dsch, 10.dsch,
│   test.dsch, merge.dsch,
│   timing.dsch                 # timing.dsch — regression fixture for the stacked-sequence timing fix
├── prototype.dsch               # predates this revision's grammar — see Known issues
└── src/
    ├── main.rs                 # Entry point: CLI → parse → compose → schedule → MIDI
    ├── lib.rs                  # Library crate root — re-exports compiler modules
    ├── pest_parser.rs          # Pest PEG parser → AST
    ├── track.rs                # Unused scaffolding (`Seq<T>` layer builder) — not wired into lib.rs
    └── compiler/
        ├── mod.rs
        ├── ast.rs               # AST type definitions
        ├── functional.rs        # Monad / Functor / Combinator scaffolding
        ├── composer.rs          # Fold over AST → scoped-context arena
        ├── lexer/mod.rs         # Empty stub, not declared as a module anywhere — dead scaffolding
        └── codegen/
            ├── mod.rs           # MIDI-domain types, `Interpolant`/`Normalize`/`ToString` traits, `Thunk`
            ├── state.rs         # `State` — context arena + the `sequence`/`combine_sequences` merge pass
            └── utils.rs         # `length_to_ticks`, `gcd`, terminal debug-print helpers (`out`, `graph`, `print_state`)
```

`src/compiler/lexer/` is new scaffolding this revision — it's an empty file and isn't declared as a module in `compiler/mod.rs`, so it doesn't compile into the binary at all, the same as the pre-existing `track.rs`.

## The `.dsch` DSL

`.dsch` is an expression-oriented language for algorithmic music composition drawing on two programming paradigms:

- **Concatenative** — at the surface level, meaning arises from juxtaposition. Placing expressions next to each other implicitly threads a musical context from left to right, with no explicit binding operator. This is the same model used by languages like Forth and Joy.
- **Functional** — internally, AST nodes compose through the `Monad<Exp>::bind` operator in `functional.rs`. The grammar now has `import` and function declarations (`f x = ...`) as a first step toward bringing this to the DSL surface itself, though neither is implemented in the composer yet (see below).

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
| `<n>` `<n>.<n>` `+<n>` `-<n>` `<a>/<b>` | Numbers — integer, float, signed (relative), rational |
| `d<n>` | Fractional duration — e.g. `d4` = quarter note, `d8` = eighth note |
| `d<a>/<b>` | Rational duration — e.g. `d3/16` |
| `<n> bpm` | Tempo |
| `<n>Hz` | Frequency |
| `#<n>` | MIDI program (instrument) change, e.g. `#48` for strings |
| `~` | Rest |
| `ppp` `pp` `p` `mp` `mf` `f` `ff` `fff` | Discrete dynamic level |

> **Note:** whitespace between a prefix token (`d`, `@`, `#`) and its argument is no longer restricted — `d 4` and `d4` both parse the same way now that duration is composed as a generic prefix + scalar, rather than through its own atomic grammar rule.

Fixed-duration forms (`5'`, `2"`, `5'2"` — minutes/seconds) **aren't implemented yet** this revision — the `duration`/`fixed`/`minutes`/`seconds`/`fractional`/`tuplet` rules are commented out in `grammar.pest`, not removed as a design decision, pending being reworked into the new prefix-based duration scheme. This is why `prototype.dsch`, which still uses `5'`-style durations, doesn't parse — see [Known issues](#known-issues).

Pitch classes are bare numbers with no prefix (`0 3 7 10`) — the `pc` prefix keyword remains removed from the grammar (dropped in the previous revision, when `composer.rs` was rewritten from scratch and `Prefix::Pc` was left unimplemented).

**Prefix:**

| Token | Meaning |
|-------|---------|
| `@` | Register (octave) |
| `@+<n>` `@-<n>` | Relative register offset — **new this revision**, e.g. `@+1` shifts up an octave from the current register |
| `d` | Duration prefix |
| `#` | Bare program-change prefix — **new this revision**, grammar-only. Only single-program use works, and only via the atomic `#<n>` scalar setting an ambient program for whatever compound follows (e.g. `#48 (...)`); the `#` prefix taking a compound argument directly (e.g. `# (48)`, `# (48 13)`) has no arm in `compose_prefix` and panics (`todo!()`) regardless of how many programs the compound holds — that path depends on "homogeneous layers" (`Layer::Homogenous`/`HomogenousLayer` in `codegen/mod.rs`), which are defined but not wired into anything yet |

**Suffix:**

| Token | Meaning |
|-------|---------|
| `bpm` | Apply a tempo to the preceding compound |
| `Hz`  | Apply a frequency to the preceding compound |
| `^`   | Amplitude (velocity) |

**Infix:**

| Token | Meaning |
|-------|---------|
| `:`   | Ratio separator (not implemented — `todo!()`) |
| `><`  | Intercalate — interleave two sequences (not implemented — `todo!()`) |
| `..`  | Range — inclusive discrete enumeration (not implemented — `todo!()`) |
| `<` / `>` | Interpolate upward/downward — ramps a duration, register, or dynamic level between two values, and chains across further `<`/`>` for multi-point ramps (e.g. `mp < ff > pp`) — **implemented this revision** for `Duration`/`Register`/`Dynamic` pairs; `Frequency`, bare `Pure` numbers, `Tuplet`, `Prog`, and `Rest` pairs are still `todo!()` |
| `*`   | Repeat — repeats a parenthesized or braced group by an integer count (e.g. `(im7 * 16)`) — **implemented this revision** for `Parens`/`Braces` groups multiplied by a plain integer; other combinations (compound × compound, duration/dynamic/prog counts) are `todo!()` |
| `->`  | Arrow — grammar-only, added for a planned function-type syntax; no `Infix::Arrow` AST variant exists yet, so it's currently inert |
| `+` `-` `/` | Arithmetic on numeric operands (not implemented — `todo!()`) |

**Bindings and declarations:**

`ident: exp` declares a named expression (`Decl::ExpDecl`), unchanged from the previous revision. The grammar now also has `import ident` (`Decl::ImportDecl`) and `ident param+ = exps` function declarations (`Decl::FuncDecl`), but the composer doesn't implement either yet (both are `todo!()` in `compose_decl`) — no current fixture uses them.

**Comments:** `-- comment text` runs to end of line.

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

For a denser example exercising this revision's new features, see `7.dsch`/`8.dsch` (`(d16 im7 * 16)` — repetition) and `6.dsch` (`@+1`/`@-1` relative register shifts, `mp < ff > pp` dynamic ramps).

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
    │  (`combine_sequences`), producing a `timeline: BTreeMap<Length, Vec<Slice>>`
    │  of time-stamped contexts.
    ▼
  Scheduler — walks the timeline in order, forwarding the clock between
    entries and emitting NoteOn/NoteOff, ProgramChange, and Tempo events
    into a BTreeMap<Ticks, Vec<Instruction>>
    ▼
  MIDI (midly::Smf, PPQ = 25200)
```

### Parser

A Pest PEG grammar (`grammar.pest`) drives a hand-written recursive-descent walker in `pest_parser.rs` that builds the typed AST defined in `compiler/ast.rs`. `decl` (`import`/`expdecl`/`funcdecl`) is now tried before `simple`/`compound` at the top of `exp`, rather than living inside `simple` as before.

### Composer

`compiler/composer.rs` (now ~1,600 lines, up from 353 after the previous from-scratch rewrite) still drives itself the same way: `State` implements `Iterator<Item = Exp>` over `state.exps`/`state.index`, `compose_exp` dispatches on `Exp`, and `compose_simple` dispatches on `Simple`. This revision's growth is mostly new dispatch arms rather than a structural change — repetition (`Infix::Mul`), relative register offsets (`compose_relative::<Register>`), and multi-point interpolation/ramps (`compose_interpolation`, now handling chained `<`/`>` sequences instead of a single pair) are all new.

**Composer coverage is currently partial** — a large number of branches (about 95 call sites) are `todo!()` stubs, mostly combinations that no current fixture exercises:

| Composer path | Status |
|---|---|
| Bare pitch-class numbers, tempo, `d<n>`/`d<a>/<b>` durations, dynamics, `#n` program change, `@n`/`@+n`/`@-n` register, bindings (`ident: exp`) | Working — exercised by current fixtures |
| `*` repetition of a `(...)`/`{...}` group by a plain integer | Working (new this revision) |
| `<`/`>` interpolation/ramps between two or more `Duration`, `Register`, or `Dynamic` values | Working (new this revision) |
| `import`/function declarations (`Decl::ImportDecl`, `Decl::FuncDecl`) | Grammar + AST only — `todo!()` in the composer |
| `#` (program) prefix with a compound argument | `todo!()` regardless of a single vs. multiple programs — depends on the not-yet-wired-up "homogeneous layers" mechanism. The atomic `#<n>` scalar (single program, e.g. `#48 (...)`) works fine on its own |
| `@` (register) prefix with a compound argument (e.g. `@ (4 5)`) | `todo!()` |
| `compose_infix` for `:`, `><`, `..`, `+`, `-`, `/` | `todo!()` |
| `compose_suffix` (`^`, bare `bpm`/`Hz` as suffixes), relative numbers as a tempo suffix (`+n bpm`) | `todo!()` |
| Interpolation/ramps involving `Frequency`, bare `Pure` numbers, `Tuplet`, `Prog`, or `Rest` | `todo!()` |
| `*` repetition of anything other than `(...)`/`{...}` × integer (compound × compound, duration/dynamic/prog counts) | `todo!()` |

### Codegen types

`codegen/mod.rs` (now ~1,700 lines, up from 431) defines the MIDI-domain types — `Length` (backed by `num_rational::Ratio<u64>` microseconds, not a plain integer), `Mpb`, `Velocity`, `Pc`, `Prog`, `Register`, `Ctx`, `Instruction` — plus a new generic layer this revision: `Normalize` and `Interpolant` traits give a common `get_vec`/`get_vec_mut`/interpolation interface across those types, and `Thunk` (paired with `LifeCycleEvent`) lets the composer register a closure to run against `State` at a later composition phase instead of writing directly — used, for example, so a bare `Prog` binding is applied when its scope actually finalizes rather than immediately.

### `State` (`codegen/state.rs`)

`State` (now ~1,800 lines, up from 729) still owns the context arena the same way, but the merge output changed shape: `state.timeline()` is now a `BTreeMap<Length, Vec<Slice>>` (was `Vec<(u64, Ctx)>`), where `Slice` groups per-tick data into `Layer`/`HomogenousLayer` collections rather than pointing back at a single `Ctx`. `State::sequence`/`combine_sequences` still recurse `Scope::Sequence` nodes and fold sibling sequences into the timeline via a GCD-derived step per outer-loop iteration, the same approach that fixed the previous revision's extra-gap bug — but see [Known issues](#known-issues): more complex pieces are still producing rhythms that don't sound right, so this isn't fully solved.

### Scheduler

`compiler/scheduler.rs` (now ~600 lines, up from 450) iterates the timeline the same way as before — converting each entry's time to ticks at the current tempo, forwarding the clock, and dispatching on scope — adapted to read from the new `BTreeMap<Length, Vec<Slice>>`/`Slice` shape instead of `Vec<(u64, Ctx)>`.

## Known issues

- **Rhythmic discrepancies in more complex pieces.** The GCD-step fix in `combine_sequences` resolves the extra-gap bug for the simple stacked-sequence cases it targeted, but listening back to more elaborate pieces — deeper nesting, more voices, mixed sequence/stack shapes, the new repetition (`*`) and ramp (`<`/`>`) operators layered together — has turned up timing that doesn't sound right. Root cause not yet isolated.
- **Relative-number expressions are only partly implemented.** `@+n`/`@-n` (relative register) work via `compose_relative::<Register>`, but `Pure::Relative` is still `todo!()` in several other contexts — notably as a tempo suffix (`+n bpm`) and inside ramps/interpolation.
- **The debug graph visualizer and other debug scaffolding are noisy and still live in the hot path.** `codegen/state.rs` calls `graph()` unconditionally in `State::sequence` (previously this panicked on `crossterm::terminal::size()` `WouldBlock`/layout errors in non-interactive environments; that particular crash wasn't reproduced in this pass, but the call is still unconditional and terminal-geometry-dependent). Separately, `composer.rs` is full of live `eprintln!`/`dbg!` calls in hot paths (`compose`, `compose_infix`, `compose_interpolation`, …) that produce very large volumes of ANSI-formatted debug output on every run — several megabytes of text even for small fixtures. Worth gating behind a verbosity flag rather than leaving unconditional.
- **`prototype.dsch`** doesn't parse, for a more fundamental reason than before: the fixed-duration grammar rules (`duration`, `fixed`, `minutes`, `seconds`, `fractional`, `tuplet`) are commented out of `grammar.pest` this revision — not removed for good, just not yet reimplemented under the new prefix-based duration scheme — so `5'`/`2"`-style durations have nowhere to go regardless of rule ordering. It also still uses the pre-`@` `reg` keyword. It documents an older/target DSL shape but is not currently runnable.
- **`import`/function declarations** (`Decl::ImportDecl`, `Decl::FuncDecl`) exist in the grammar and AST but are `todo!()` in the composer — no current fixture uses them.
- **The `->` arrow infix and multi-scalar `controls` interpolation rule** are grammar-only additions with no corresponding AST variant (`Infix` has no `Arrow`, `Interpolation` has no `Controls`) — they're effectively inert until wired up.
- `src/track.rs` (`Seq<T>` layer builder) and the new `src/compiler/lexer/mod.rs` (currently empty) are not referenced from `lib.rs`/`compiler/mod.rs` and appear to be disconnected scaffolding.

## Implementation status

| Stage | Status |
|-------|--------|
| Parser (grammar + AST) | Complete for the current grammar; `prototype.dsch`'s fixed-duration syntax doesn't parse yet — not yet reimplemented (see Known issues) |
| Composer — pitch classes, tempo, durations, dynamics, program change, register (incl. relative `@+n`/`@-n`), bindings, `*` repetition, `<`/`>` ramps | Working |
| Composer — `import`/function declarations | Grammar + AST only, not implemented (`todo!()`) |
| Composer — infix (`:`, `><`, `..`, `+`, `-`, `/`), suffix forms, `#`/`@` prefix with a compound argument | Not implemented (`todo!()`) |
| `State::sequence` polyphonic merge → timeline | Working for the cases the GCD-step fix targeted; rhythmic discrepancies remain in more complex pieces — see Known issues |
| Scheduler → MIDI | Working — 15 of 16 checked-in fixtures (all but `prototype.dsch`) compose, sequence, and schedule to a `.mid` file, modulo the rhythmic discrepancies above |

## Running

```bash
# Read <name>.dsch from the working directory, write <name>.mid alongside it
cargo run -- --input <name>
```

For example, `cargo run -- --input 1` parses `1.dsch`, composes it, sequences it, schedules MIDI events, and writes `1.mid`. `0`/`0.5`/`1`–`10`/`test`/`merge`/`timing` are current fixtures on the current grammar; `prototype` is not (see [Known issues](#known-issues)).

## Built with

- [`pest`](https://github.com/pest-parser/pest) — PEG parser generator
- [`pest_derive`](https://github.com/pest-parser/pest) — derive macro for typed Pest grammars
- [`midly`](https://github.com/kovaxis/midly) — MIDI file I/O
- [`clap`](https://github.com/clap-rs/clap) — CLI argument parsing (`--input <name>`)
- [`num-rational`](https://github.com/rust-num/num-rational) — backs `Length`'s microsecond representation (`Ratio<u64>`) throughout `codegen`; no longer an unused dependency
- [`num-bigint`](https://github.com/rust-num/num-bigint) / [`num-traits`](https://github.com/rust-num/num-traits) — new this revision, alongside `num-rational`'s `num-bigint` feature
- [`derive_more`](https://github.com/JelteF/derive_more) — new this revision (`from`/`into` derives), used across the new `Interpolant`/`Normalize` codegen types
- [`crossterm`](https://github.com/crossterm-rs/crossterm) — cross-platform terminal control (used for debug/progress output; see Known issues — the debug graph visualizer's dependency on terminal size is a reliability risk outside a normal interactive terminal)
- [`colonnade`](https://github.com/dfhoughton/colonnade) — aligned terminal column formatting for state inspection
- [`colprint`](https://crates.io/crates/colprint) — coloured terminal output helpers
- [`rust-sugiyama`](https://github.com/paddison/rust-sugiyama) — Sugiyama-style layered graph layout (context-tree visualisation)

Audio synthesis dependencies (`cpal`, `ndarray`, `ringbuf`, `bit-set`) will be reintroduced in a later phase when DSCH grows custom-instrument support.

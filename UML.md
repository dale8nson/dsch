```mermaid
classDiagram
    class src_compiler_codegen_Normalize {
        +norm() Output
        +norm() Output
        +norm() Output
        +norm() Output
        +norm() Output
        +norm() Output
    }
    class src_compiler_codegen_ToString {
        +to_string() String
        +to_string() String
        +to_string() String
        +to_string() String
        +to_string() String
        +to_string() String
    }
    class src_compiler_codegen_Interpolant {
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
        +get_vec(state: &State, ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Self::T>>~
        +get_vec_mut(state: &mut State, ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Self::T>>~
    }
    class src_compiler_codegen_From {
        +from(value: ~Length as Interpolant~) Self
        +from(value: Data) Self
        +from(value: ~Length as Interpolant~) Self
        +from(value: u64) Self
        +from(value: src_compiler_Absolute) Self
        +from(value: src_compiler_Fixed) Self
        +from(value: usize) Self
        +from(value: f64) Self
        +from(value: src_compiler_codegen_Velocity) Self
        +from(value: src_compiler_codegen_Velocity) Self
        +from(value: f64) Self
        +from(value: Data) Self
        +from(value: src_compiler_Absolute) Self
        +from(value: usize) Self
        +from(value: src_compiler_Dynamic) Self
        +from(value: usize) Self
        +from(value: src_compiler_codegen_Pc) Self
        +from(value: src_compiler_codegen_Pc) Self
        +from(value: usize) Self
        +from(value: Data) Self
        +from(value: src_compiler_Absolute) Self
        +from(value: src_compiler_Absolute) Self
        +from(value: usize) Self
        +from(value: src_compiler_codegen_Mpb) Self
        +from(value: src_compiler_codegen_Length) Self
        +from(value: src_compiler_codegen_Mpb) Self
        +from(value: f64) Self
        +from(value: Data) Self
        +from(value: ~Prog as Interpolant~) Self
        +from(value: Data) Self
        +from(value: src_compiler_Absolute) Self
        +from(value: usize) Self
        +from(value: f64) Self
        +from(value: src_compiler_codegen_Prog) Self
        +from(value: src_compiler_codegen_Register) Self
        +from(value: f64) Self
        +from(value: Data) Self
        +from(value: usize) Self
        +from(value: src_compiler_codegen_Register) Self
        +from(value: src_compiler_Absolute) Self
    }
    class src_compiler_codegen_Length {
        +as_u64() u64
        +as_u32() u32
        +as_f64() f64
        +as_usize() usize
        +default_max() Self
        +to_note(self, tempo: &Tempo) String
        +to_rest(self, tempo: &Tempo) String
        +as_ratio() Ratio~u64~
    }
    class src_compiler_codegen_Default {
        +default() Self
        +default() Self
        +default() Self
        +default() Self
        +default() Self
    }
    class src_compiler_codegen_Mul {
        +mul(self, rhs: src_compiler_codegen_Length) Output
        +mul(self, rhs: Self) Output
        +mul(self, rhs: src_compiler_codegen_Velocity) Output
        +mul(self, rhs: src_compiler_codegen_Pc) Output
        +mul(self, rhs: src_compiler_codegen_Mpb) Output
        +mul(self, rhs: src_compiler_codegen_Mpb) Output
        +mul(self, rhs: Self) Output
        +mul(self, rhs: src_compiler_codegen_Register) Output
    }
    class src_compiler_codegen_Div {
        +div(self, rhs: usize) Output
        +div(self, rhs: Self) Output
        +div(self, rhs: Self) Output
        +div(self, rhs: usize) Output
        +div(self, rhs: src_compiler_codegen_Pc) Output
        +div(self, rhs: usize) Output
        +div(self, rhs: Self) Output
        +div(self, rhs: Self) Output
        +div(self, rhs: Self) Output
    }
    class src_compiler_codegen_PartialEq {
        +eq(other: &Self) bool
        +eq(other: &Self) bool
        +eq(other: &Self) bool
        +eq(other: &Self) bool
    }
    class src_compiler_codegen_PartialOrd {
        +partial_cmp(other: &Self) Option~std::cmp::Ordering~
        +partial_cmp(other: &Self) Option~std::cmp::Ordering~
        +partial_cmp(other: &Self) Option~std::cmp::Ordering~
    }
    class src_compiler_codegen_Add {
        +add(self, rhs: Self) Output
        +add(self, rhs: Self) Output
        +add(self, rhs: Self) Output
        +add(self, rhs: src_compiler_Absolute) Output
        +add(self, rhs: Self) Output
        +add(self, rhs: Self) Output
        +add(self, rhs: Self) Output
        +add(self, rhs: i8) Output
    }
    class src_compiler_codegen_Sum {
        +sum(iter: I) Self
    }
    class src_compiler_codegen_Rem {
        +rem(self, rhs: Self) Output
    }
    class src_compiler_codegen_Sub {
        +sub(self, rhs: Self) Output
        +sub(self, rhs: Self) Output
        +sub(self, rhs: src_compiler_Absolute) Output
        +sub(self, rhs: Self) Output
        +sub(self, rhs: Self) Output
        +sub(self, rhs: Self) Output
        +sub(self, rhs: Self) Output
    }
    class src_compiler_codegen_AddAssign {
        +add_assign(rhs: Self) void
        +add_assign(rhs: Self) void
        +add_assign(rhs: Self) void
        +add_assign(rhs: Self) void
        +add_assign(rhs: Self) void
        +add_assign(rhs: Self) void
    }
    class src_compiler_codegen_SubAssign {
        +sub_assign(rhs: Self) void
        +sub_assign(rhs: Self) void
        +sub_assign(rhs: Self) void
        +sub_assign(rhs: Self) void
        +sub_assign(rhs: Self) void
        +sub_assign(rhs: Self) void
    }
    class src_compiler_codegen_Velocity {
    }
    class src_compiler_codegen_Ord {
        +cmp(other: &Self) Ordering
        +cmp(other: &Self) Ordering
    }
    class src_compiler_codegen_BuildHasher {
        +build_hasher() Hasher
        +build_hasher() Hasher
        +build_hasher() Hasher
        +build_hasher() Hasher
        +build_hasher() Hasher
        +build_hasher() Hasher
    }
    class src_compiler_codegen_Ctx {
        +to_usize() usize
        +to_u32() u32
    }
    class src_compiler_codegen_Into {
        +into(self) usize
    }
    class src_compiler_codegen_Eq {
    }
    class src_compiler_codegen_Pc {
        +as_f64() f64
    }
    class src_compiler_codegen_Mpb {
    }
    class src_compiler_codegen_Prog {
    }
    class src_compiler_codegen_Register {
        +as_i8() i8
    }
    class src_compiler_codegen_Thunk {
        +call(ctx: src_compiler_codegen_Ctx, state: &mut State) void
    }
    class src_compiler_codegen_Debug {
        +fmt(f: Formatter~'_~) Result
    }
    class src_compiler_codegen_Slice {
        -pcs: Vec~src_compiler_codegen_Pc~
        -registers: Vec~src_compiler_codegen_Register~
        -lengths: Vec~src_compiler_codegen_Length~
        -velocities: Vec~src_compiler_codegen_Velocity~
        -tempos: Vec~Tempo~
        -programs: Vec~src_compiler_codegen_Prog~
        +new(pcs: Vec~src_compiler_codegen_Pc~, registers: Vec~src_compiler_codegen_Register~, lengths: Vec~src_compiler_codegen_Length~, velocities: Vec~src_compiler_codegen_Velocity~, tempos: Vec~Tempo~, programs: Vec~src_compiler_codegen_Prog~) Self
        +data() (
        &[Pc],
        &[Register],
        &[Length],
        &[Velocity],
        &[Tempo],
        &[Prog]
    )
    }
    class src_compiler_codegen_State {
        -index_: usize
        -stack_indexes: Vec~usize~
        -peeked: Vec~src_compiler_Exp~
        -exps: Vec~src_compiler_Exp~
        -ctx_count: usize
        -ctxs: HashSet~src_compiler_codegen_Ctx~
        -parents: HashMap~src_compiler_codegen_Ctx, src_compiler_codegen_Ctx~
        -children: HashMap~src_compiler_codegen_Ctx, Vec<Ctx>~
        -scopes: HashMap~src_compiler_codegen_Ctx, Scope~
        -layers: HashMap~src_compiler_codegen_Ctx, Layer~
        -program: Option~Vec<Prog>~
        -programs_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Prog>>~
        -register: Option~Vec<Register>~
        -registers_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Register>>~
        -pcs_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Pc>>~
        -length: Option~Vec<Length>~
        -lengths_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Length>>~
        -velocity: Option~Vec<Velocity>~
        -velocities_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Velocity>>~
        -tempo: src_compiler_codegen_Mpb
        -tempos_: HashMap~src_compiler_codegen_Ctx, Vec<Vec<Mpb>>~
        -bindings: HashMap~src_compiler_codegen_Ctx, HashMap<Ident, Exp>~
        -instruments: HashMap~src_compiler_codegen_Prog, Vec<u8>~
        -timeline: Vec~(u64, Ctx)~
        -timeline_: BTreeMap~src_compiler_codegen_Length, Vec<Slice>~
        -playhead: src_compiler_codegen_Length
        -stack: Vec~src_compiler_Exp~
        -thunks: HashMap~src_compiler_codegen_Ctx, HashMap<LifeCycleEvent, Vec<Thunk>>~
        -thunk_stack: Vec~(LifeCycleEvent, Thunk)~
        -lead_counter: src_compiler_codegen_Length
        +append_child(parent: src_compiler_codegen_Ctx) src_compiler_codegen_Ctx
        +new_ctx() src_compiler_codegen_Ctx
        +index() usize
        +exps() &Vec~src_compiler_Exp~
        +set_exps(exps: Vec~src_compiler_Exp~, index: usize) (Vec~src_compiler_Exp~
        +set_stack(stack: Vec~src_compiler_Exp~) Vec~src_compiler_Exp~
        +scope(ctx: src_compiler_codegen_Ctx) Scope
        +set_scope(ctx: src_compiler_codegen_Ctx, scope: Scope) void
        +layer(ctx: src_compiler_codegen_Ctx) Layer
        +set_layer(ctx: src_compiler_codegen_Ctx, layer: Layer) void
        +parent(ctx: src_compiler_codegen_Ctx) src_compiler_codegen_Ctx
        +set_parent(ctx: src_compiler_codegen_Ctx, parent: src_compiler_codegen_Ctx) void
        +children(ctx: src_compiler_codegen_Ctx) Vec~src_compiler_codegen_Ctx~
        +children_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Ctx>~
        +move_child(child: src_compiler_codegen_Ctx, parent: src_compiler_codegen_Ctx) void
        +move_children(src: src_compiler_codegen_Ctx, dest: src_compiler_codegen_Ctx) void
        +add_child(parent: src_compiler_codegen_Ctx, child: src_compiler_codegen_Ctx) void
        +programs(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Prog>>~
        +programs_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Prog>>~
        +add_program(ctx: src_compiler_codegen_Ctx, program: src_compiler_codegen_Prog) void
        +add_programs(ctx: src_compiler_codegen_Ctx, programs: Vec~src_compiler_codegen_Prog~) void
        +instruments() &HashMap~src_compiler_codegen_Prog, Vec<u8>~
        +instruments_mut() &mut HashMap~src_compiler_codegen_Prog, Vec<u8>~
        +registers(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Register>>~
        +registers_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Register>>~
        +add_register(ctx: src_compiler_codegen_Ctx, register: src_compiler_codegen_Register) void
        +add_registers(ctx: src_compiler_codegen_Ctx, registers: Vec~src_compiler_codegen_Register~) void
        +pcs(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Pc>>~
        +pcs_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Pc>>~
        +add_pc(ctx: src_compiler_codegen_Ctx, pc: src_compiler_codegen_Pc) void
        +add_pcs(ctx: src_compiler_codegen_Ctx, pcs: Vec~src_compiler_codegen_Pc~) void
        +velocities(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Velocity>>~
        +velocities_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Velocity>>~
        +velocity() Option~Vec<Velocity>~
        +set_velocity(velocity: src_compiler_codegen_Velocity) void
        +add_velocities(ctx: src_compiler_codegen_Ctx, velocities: Vec~src_compiler_codegen_Velocity~) void
        +add(ctx: src_compiler_codegen_Ctx, ts: Vec~T~) void
        +extend(ctx: src_compiler_codegen_Ctx, ts: Vec~Vec<T>~) void
        +set_velocities(ctx: src_compiler_codegen_Ctx, velocities: Vec~Vec<Velocity>~) void
        +lengths(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Length>>~
        +lengths_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Length>>~
        +add_length(ctx: src_compiler_codegen_Ctx, length: src_compiler_codegen_Length) void
        +add_lengths(ctx: src_compiler_codegen_Ctx, lengths: Vec~src_compiler_codegen_Length~) void
        +set_lengths(ctx: src_compiler_codegen_Ctx, lengths: Vec~Vec<Length>~) void
        +tempos(ctx: src_compiler_codegen_Ctx) Option~Vec<Vec<Mpb>>~
        +tempos_mut(ctx: src_compiler_codegen_Ctx) Option~&mut Vec<Vec<Tempo>>~
        +set_bpm(bpm: src_compiler_Absolute) void
        +tempo() Tempo
        +set_tempo(tempo: Tempo) void
        +add_tempo(ctx: src_compiler_codegen_Ctx, tempo: src_compiler_codegen_Mpb) void
        +add_tempos(ctx: src_compiler_codegen_Ctx, tempos: Vec~Tempo~) void
        +bindings(ctx: src_compiler_codegen_Ctx) HashMap~src_compiler_Ident, src_compiler_Exp~
        +binding(ctx: src_compiler_codegen_Ctx, ident: &Ident) Option~src_compiler_Exp~
        +add_binding(ctx: src_compiler_codegen_Ctx, ident: src_compiler_Ident, exp: src_compiler_Exp) void
        +add_bindings(ctx: src_compiler_codegen_Ctx, bindings: HashMap~src_compiler_Ident, src_compiler_Exp~) void
        +thunks(ctx: src_compiler_codegen_Ctx) HashMap~LifeCycleEvent, Vec<Thunk>~
        +push_thunk(event: LifeCycleEvent, thunk: src_compiler_codegen_Thunk) void
        +add_thunk(ctx: src_compiler_codegen_Ctx, life_cycle_event: LifeCycleEvent, thunk: src_compiler_codegen_Thunk) void
        +call_thunks(ctx: src_compiler_codegen_Ctx, life_cycle_event: LifeCycleEvent) void
        +push_back(exp: src_compiler_Exp) void
        +store(exp: src_compiler_Exp) void
        +load() Option~src_compiler_Exp~
        +push() void
        +pop() Option~src_compiler_Exp~
        +stack() &Vec~src_compiler_Exp~
        +timeline() &BTreeMap~src_compiler_codegen_Length, Vec<Slice>~
        +timeline_mut() &mut BTreeMap~src_compiler_codegen_Length, Vec<Slice>~
        +playhead() &mut Length
        +max_len(ctx: src_compiler_codegen_Ctx) usize
        +cycle_fill(ctx: src_compiler_codegen_Ctx, len: src_compiler_codegen_Length) void
        +get_last_mut(ctx: src_compiler_codegen_Ctx) Option~&'a mut T~
        +get(ctx: src_compiler_codegen_Ctx) Vec~T~
        +get_(ctx: src_compiler_codegen_Ctx) Vec~Data~
        +get_last(ctx: src_compiler_codegen_Ctx) Vec~T~
        +get_leaves(ctx: src_compiler_codegen_Ctx) Vec~src_compiler_codegen_Ctx~
        +pad(ctx: src_compiler_codegen_Ctx, len: usize) void
        +pad_(ctx: src_compiler_codegen_Ctx, f: F, g: G, len: usize) void
        +hoist(from: src_compiler_codegen_Ctx, to: src_compiler_codegen_Ctx) void
        +clean_context(ctx: src_compiler_codegen_Ctx) void
        +flatten_stack(ctx: src_compiler_codegen_Ctx) void
        +drop(ctx: src_compiler_codegen_Ctx) void
        +sequence(ctx: src_compiler_codegen_Ctx) void
        +get_ctx_length(ctx: src_compiler_codegen_Ctx) src_compiler_codegen_Length
        +stack_len() usize
        +length_sum(ctx: src_compiler_codegen_Ctx) src_compiler_codegen_Length
        +min_length(ctx: src_compiler_codegen_Ctx) src_compiler_codegen_Length
        +max_length(ctx: src_compiler_codegen_Ctx) src_compiler_codegen_Length
        +ctx_pc_count(ctx: src_compiler_codegen_Ctx) usize
        +combine_sequences(ctxs: Vec~src_compiler_codegen_Ctx~) void
        +emplace(ts: &Vec~Vec<T>~, ctx: src_compiler_codegen_Ctx, col: usize, row: usize, into_col: usize, f: fn(&mut Self, Ctx) -> Option~&mut Vec<Vec<T>>~) void
        +take_slice(from: src_compiler_codegen_Ctx, row: usize, col: usize) src_compiler_codegen_Slice
    }
    class src_compiler_codegen_Default {
        +default() Self
    }
    class src_compiler_codegen_Iterator {
        +next() Option~Self::Item~
    }
    class src_compiler_codegen_ExactSizeIterator {
        +len() usize
    }
    class src_compiler_codegen_Display {
        +fmt(f: Formatter~'_~) Result
        +fmt(f: Formatter~'_~) Result
        +fmt(f: Formatter~'_~) Result
    }
    class src_compiler_codegen_LineMap {
        +map: Option~HashMap<u32, TextStyle>~
    }
    class src_compiler_codegen_ColorIter {
        +get(line: u32) Self
    }
    class src_compiler_codegen_From {
        +from(value: src_compiler_codegen_ColorIter) Self
    }
    class src_compiler_Scheduler {
        -prog: u8
        -tempo: src_compiler_codegen_Mpb
        -clock: Ticks
        -times: BTreeSet~Ticks~
        -schedule: BTreeMap~Ticks, Vec<Instruction>~
        -instruments: HashMap~src_compiler_codegen_Prog, Vec<u8>~
        -visited: HashSet~src_compiler_codegen_Ctx~
        -event_idx: usize
        +forward(ticks: u64) void
        +rewind(ticks: u64) void
        +to(ticks: u64) void
        +ticks() u64
        +reset() void
        +event_index() &mut usize
        +add_instruction(time: u64, instruction: Instruction) void
        +tempo() src_compiler_codegen_Mpb
        +set_tempo(mpb: src_compiler_codegen_Mpb) void
        +set_program(program: src_compiler_codegen_Prog, state: &State) void
        +set_instrument(name: Vec~u8~) void
        +schedule_mut() &mut BTreeMap~Ticks, Vec<Instruction>~
        +schedule() BTreeMap~Ticks, Vec<Instruction>~
        +instruments() &HashMap~src_compiler_codegen_Prog, Vec<u8>~
    }
    class src_compiler_Default {
        +default() Self
    }
    class src_compiler_Program {
        +exps: Vec~src_compiler_Exp~
    }
    class src_compiler_Exp {
        +to_string() String
    }
    class src_compiler_Display {
        +fmt(f: Formatter~'_~) Result
        +fmt(f: Formatter~'_~) Result
    }
    class src_compiler_Bpm {
    }
    class src_compiler_ImportDecl {
    }
    class src_compiler_FuncDecl {
        +ident: src_compiler_Ident
        +params: Vec~src_compiler_Ident~
        +funcdef: Vec~src_compiler_Exp~
    }
    class src_compiler_ExpDecl {
        +ident: src_compiler_Ident
        +binding: Box~src_compiler_Exp~
    }
    class src_compiler_From {
        +from(value: src_compiler_Compound) Self
        +from(value: Data) Self
        +from(value: Duration) Self
    }
    class src_compiler_Compound {
        +to_vec() Vec~src_compiler_Exp~
        +to_exp(self) src_compiler_Exp
        +scope() Scope
    }
    class src_compiler_IntoIterator {
        +into_iter(self) IntoIter
    }
    class src_compiler_Dynamic {
    }
    class src_compiler_Frequency {
    }
    class src_compiler_Range {
        +start: src_compiler_Exp
        +end: src_compiler_Exp
    }
    class src_compiler_Ident {
    }
    class src_compiler_BuildHasher {
        +build_hasher() Hasher
    }
    class src_compiler_Prefix {
        +unwrap(exp: src_compiler_Exp) Option~Self~
    }
    class src_compiler_Fixed {
        +minutes: src_compiler_Absolute
        +seconds: src_compiler_Absolute
    }
    class src_compiler_Rational {
        +num: src_compiler_Absolute
        +den: src_compiler_Absolute
    }
    class src_compiler_Tuplet {
        +lhs: src_compiler_Absolute
        +rhs: src_compiler_Absolute
    }
    class src_compiler_Minutes {
    }
    class src_compiler_Seconds {
    }
    class src_compiler_Relative {
        +sign: Sign
        +val: src_compiler_Absolute
    }
    class src_compiler_Default {
        +default() Self
    }
    class src_compiler_Mul {
        +mul(self, rhs: u64) Output
        +mul(self, rhs: src_compiler_codegen_Length) Output
    }
    class src_compiler_Div {
        +div(self, rhs: src_compiler_Absolute) Output
        +div(self, rhs: Self) Output
    }
    class src_compiler_Absolute {
        +as_u64() u64
        +as_f64() f64
        +as_usize() usize
    }
    class src_compiler_Category~T~ {
        -objs: Vec~T~
        -morphs: Vec~Box<dyn Fn(T) -> T>~
    }
    class src_compiler_Monad~Clone~ {
        +ret(a: A) src_compiler_Monad~A~
        +bind(self, f: F) src_compiler_Monad~B~
    }
    class src_compiler_Add~Clone, Clone~ {
        +add(self, rhs: Box~dyn FnOnce(A) -> Monad<B>~) Output
    }
    class src_compiler_AddAssign~Clone~ {
        +add_assign(rhs: Box~dyn FnOnce(A) -> Monad<A>~) void
    }
    class src_compiler_Functor~T~ {
        +default() Self
        +shl(self, rhs: Self) Output
        +shr(self, rhs: Self) Output
        +new(f: fn (T) -> T) Self
        +call(input: T) T
    }
    class src_compiler_Combinator~A, B, C, D~ {
        +compose(self, f: Box~dyn Fn(A) -> B~, g: Box~dyn Fn(C) -> D~) impl Fn(A) -> D
    }
    class src_Seq~T~ {
        -layers: Vec~Transformer<T>~
        +new() Self
        +layer(self, l: Transformer~T~) Self
        +add_layer(l: Transformer~T~) void
        +build() void
    }
    class src_CLParser {
    }
    class src_Args {
        -input: String
    }
    src_compiler_codegen_Slice --> src_compiler_codegen_Pc
    src_compiler_codegen_Slice --> src_compiler_codegen_Register
    src_compiler_codegen_Slice --> src_compiler_codegen_Length
    src_compiler_codegen_Slice --> src_compiler_codegen_Velocity
    src_compiler_codegen_Slice --> src_compiler_codegen_Prog
    src_compiler_codegen_State --> src_compiler_Exp
    src_compiler_codegen_State --> src_compiler_Exp
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Mpb
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Prog
    src_compiler_codegen_State --> src_compiler_codegen_Length
    src_compiler_codegen_State --> src_compiler_codegen_Length
    src_compiler_codegen_State --> src_compiler_Exp
    src_compiler_codegen_State --> src_compiler_codegen_Ctx
    src_compiler_codegen_State --> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Ctx
    src_compiler_codegen_State ..> src_compiler_codegen_Ctx
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_codegen_Ctx
    src_compiler_codegen_State ..> src_compiler_codegen_Ctx
    src_compiler_codegen_State ..> src_compiler_codegen_Prog
    src_compiler_codegen_State ..> src_compiler_codegen_Prog
    src_compiler_codegen_State ..> src_compiler_Ident
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_Exp
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Ctx
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Length
    src_compiler_codegen_State ..> src_compiler_codegen_Slice
    src_compiler_Scheduler --> src_compiler_codegen_Mpb
    src_compiler_Scheduler --> src_compiler_codegen_Prog
    src_compiler_Scheduler --> src_compiler_codegen_Ctx
    src_compiler_Scheduler ..> src_compiler_codegen_Mpb
    src_compiler_Scheduler ..> src_compiler_codegen_Prog
    src_compiler_Program --> src_compiler_Exp
    src_compiler_FuncDecl --> src_compiler_Ident
    src_compiler_FuncDecl --> src_compiler_Ident
    src_compiler_FuncDecl --> src_compiler_Exp
    src_compiler_ExpDecl --> src_compiler_Ident
    src_compiler_ExpDecl --> src_compiler_Exp
    src_compiler_Compound ..> src_compiler_Exp
    src_compiler_Compound ..> src_compiler_Exp
    src_compiler_Range --> src_compiler_Exp
    src_compiler_Range --> src_compiler_Exp
    src_compiler_Fixed --> src_compiler_Absolute
    src_compiler_Fixed --> src_compiler_Absolute
    src_compiler_Rational --> src_compiler_Absolute
    src_compiler_Rational --> src_compiler_Absolute
    src_compiler_Tuplet --> src_compiler_Absolute
    src_compiler_Tuplet --> src_compiler_Absolute
    src_compiler_Relative --> src_compiler_Absolute

```
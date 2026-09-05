use std::{
    array::IntoIter,
    collections::{BTreeMap, HashMap},
    fmt::Display,
    io::stderr,
    iter::{Cloned, Cycle, from_fn, repeat_n, zip},
    ops::{Deref, Div, Mul, Rem},
    slice::Iter,
    sync::LazyLock,
};

use crate::{
    color,
    compiler::{
        ast::{utils::abs_to_f64, *},
        codegen::{state::State, *},
    },
};

use colonnade::{Alignment, Colonnade};
use crossterm::{cursor::*, execute, style::*, terminal::*};
use rust_sugiyama::{
    configure::{Config, RankingType},
    from_edges,
};

#[derive(Clone, Copy)]
pub enum Color {
    Black(Intensity),
    Red(Intensity),
    Green(Intensity),
    Yellow(Intensity),
    Blue(Intensity),
    Purple(Intensity),
    Cyan(Intensity),
    White(Intensity),
    Reset,
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::Black(intensity) => write!(f, "\x1b{intensity}0m"),
            Color::Red(intensity) => write!(f, "\x1b{intensity}1m"),
            Color::Green(intensity) => write!(f, "\x1b{intensity}2m"),
            Color::Yellow(intensity) => write!(f, "\x1b{intensity}3m"),
            Color::Blue(intensity) => write!(f, "\x1b{intensity}4m"),
            Color::Purple(intensity) => write!(f, "\x1b{intensity}5m"),
            Color::Cyan(intensity) => write!(f, "\x1b{intensity}6m"),
            Color::White(intensity) => write!(f, "\x1b{intensity}7m"),
            Color::Reset => write!(f, "\x1b[0m"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Intensity {
    Normal,
    Intense,
    Bold,
    IntenseBold,
}

impl Display for Intensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Intensity::Normal => write!(f, "[0;3"),
            Intensity::Intense => write!(f, "[0;9"),
            Intensity::Bold => write!(f, "[1;3"),
            Intensity::IntenseBold => write!(f, "[1;9"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextStyle {
    Black,
    IntenseBlack,
    BoldBlack,
    IntenseBoldBlack,
    Red,
    IntenseRed,
    BoldRed,
    IntenseBoldRed,
    Green,
    IntenseGreen,
    BoldGreen,
    IntenseBoldGreen,
    Yellow,
    IntenseYellow,
    BoldYellow,
    IntenseBoldYellow,
    Blue,
    IntenseBlue,
    BoldBlue,
    IntenseBoldBlue,
    Purple,
    IntensePurple,
    BoldPurple,
    IntenseBoldPurple,
    Cyan,
    IntenseCyan,
    BoldCyan,
    IntenseBoldCyan,
    White,
    IntenseWhite,
    BoldWhite,
    IntenseBoldWhite,
    ResetColor,
}

impl Display for TextStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Intensity::*;
        use TextStyle::*;
        match self {
            Black => write!(f, "{}", Color::Black(Normal)),
            IntenseBlack => write!(f, "{}", Color::Black(Intense)),
            BoldBlack => write!(f, "{}", Color::Black(Bold)),
            IntenseBoldBlack => write!(f, "{}", Color::Black(IntenseBold)),
            Red => write!(f, "{}", Color::Red(Normal)),
            IntenseRed => write!(f, "{}", Color::Red(Intense)),
            BoldRed => write!(f, "{}", Color::Red(Bold)),
            IntenseBoldRed => write!(f, "{}", Color::Red(IntenseBold)),
            Green => write!(f, "{}", Color::Green(Normal)),
            IntenseGreen => write!(f, "{}", Color::Green(Intense)),
            BoldGreen => write!(f, "{}", Color::Green(Bold)),
            IntenseBoldGreen => write!(f, "{}", Color::Green(IntenseBold)),
            Yellow => write!(f, "{}", Color::Yellow(Normal)),
            IntenseYellow => write!(f, "{}", Color::Yellow(Intense)),
            BoldYellow => write!(f, "{}", Color::Yellow(Bold)),
            IntenseBoldYellow => write!(f, "{}", Color::Yellow(IntenseBold)),
            Blue => write!(f, "{}", Color::Blue(Normal)),
            IntenseBlue => write!(f, "{}", Color::Blue(Intense)),
            BoldBlue => write!(f, "{}", Color::Blue(Bold)),
            IntenseBoldBlue => write!(f, "{}", Color::Blue(IntenseBold)),
            Purple => write!(f, "{}", Color::Purple(Normal)),
            IntensePurple => write!(f, "{}", Color::Purple(Intense)),
            BoldPurple => write!(f, "{}", Color::Purple(Bold)),
            IntenseBoldPurple => write!(f, "{}", Color::Purple(IntenseBold)),
            Cyan => write!(f, "{}", Color::Cyan(Normal)),
            IntenseCyan => write!(f, "{}", Color::Cyan(Intense)),
            BoldCyan => write!(f, "{}", Color::Cyan(Bold)),
            IntenseBoldCyan => write!(f, "{}", Color::Cyan(IntenseBold)),
            White => write!(f, "{}", Color::White(Normal)),
            IntenseWhite => write!(f, "{}", Color::White(Intense)),
            BoldWhite => write!(f, "{}", Color::White(Bold)),
            IntenseBoldWhite => write!(f, "{}", Color::White(IntenseBold)),
            ResetColor => write!(f, "{}", Color::Reset),
        }
    }
}

struct LineMap {
    pub map: Option<HashMap<u32, TextStyle>>,
}

static mut LINES: LineMap = LineMap { map: None };
static mut COLOR: [TextStyle; 5] = [
    TextStyle::IntensePurple,
    TextStyle::IntenseYellow,
    TextStyle::IntenseCyan,
    TextStyle::IntenseGreen,
    TextStyle::IntenseRed,
];
static mut COLOR_INDEX: usize = 0;

pub struct ColorIter(TextStyle);

impl From<ColorIter> for TextStyle {
    fn from(value: ColorIter) -> Self {
        value.0
    }
}

impl ColorIter {
    pub fn get(line: u32) -> Self {
        unsafe {
            if let Some(ref mut map) = LINES.map {
                if map.contains_key(&line) {
                    Self(map.get(&line).cloned().unwrap())
                } else {
                    let idx = COLOR_INDEX;
                    let color = COLOR[idx].clone();
                    map.insert(line, color);
                    COLOR_INDEX = (COLOR_INDEX + 1) % 5;
                    Self(color)
                }
            } else {
                LINES.map = Some(HashMap::<u32, TextStyle>::new());
                let idx = COLOR_INDEX;
                let color = COLOR[idx];
                COLOR_INDEX = (COLOR_INDEX + 1) % 5;
                Self(color)
            }
        }
    }
}

// impl Iterator for ColorIter {
//     type Item = TextStyle;

//     fn next(&mut self) -> Option<Self::Item> {
//         use TextStyle::*;

//         let color = unsafe { CURRENT_COLOR };
//         unsafe {
//             CURRENT_COLOR = match CURRENT_COLOR {
//                 IntensePurple => IntenseYellow,
//                 IntenseYellow => IntenseCyan,
//                 IntenseCyan => IntenseGreen,
//                 IntenseGreen => IntenseBoldRed,
//                 IntenseBoldRed => IntensePurple,
//                 _ => todo!(),
//             };
//         }

//         Some(color)
//     }
// }
#[macro_export]
macro_rules! color (
    () => {
        TextStyle::from(ColorIter::get(line!()))
    }

);

pub fn length_to_ticks(length: Length, tempo: Mpb) -> u64 {
    let ticks = match length {
        Length::None => 0,
        Length::MicroSeconds(ratio) => (ratio / tempo.0
            * Ratio::<u64>::from_u16(PPQ.as_int()).unwrap())
        .to_u64()
        .unwrap(),
    };
    // dbg!(ticks);
    // pause();
    ticks
}

pub fn compose_thunks(from: Ctx, into: Ctx, state: &mut State) {
    info(format!(
        "{}COMPOSE THUNKS FROM {from:?} INTO {into:?}",
        color!()
    ));
    misc(format!("{:?}", state.thunks(from)));
    // pause();
    misc(state_str(state, into));
    // pause();
    if let Some(thunks) = state.thunks_mut(from) {
        if let Some(thunks) = thunks.get_mut(&LifeCycleEvent::Composing) {
            thunks.reverse();
            if let Some(mut thunk) = thunks.pop() {
                thunk.call(into, state);
                compose_thunks(from, into, state);
            }
        }
    }
    misc(state_str(state, into));
    // pause();
}

pub fn gcf<T: Div + Rem<Output = T> + From<u64> + PartialOrd + Ord + PartialEq + Eq + Copy>(
    mut n1: T,
    mut n2: T,
) -> T {
    let mut rem = n1 % n2;

    while rem != Into::<T>::into(0) {
        n1 = n2;
        n2 = rem;
        rem = n1 % n2;
    }
    n2
}

pub fn align(expr: impl std::fmt::Debug, indents: usize, width: usize) -> String {
    let expr = format!("{:?}", expr).replace('\n', "");
    let step: usize = isize::min(
        (i32::min(
            width as i32 - (indents) as i32,
            i32::max(expr.len() as i32 - (indents) as i32, expr.len() as i32),
        )) as isize,
        expr.len() as isize,
    ) as usize;

    // eprintln!("step: {step}\nindents: {indents}");

    let mut prev: usize = 0;
    let mut string = format!("{}", " ".repeat(indents));
    string.push_str(
        format!(
            "{}",
            expr.chars()
                .enumerate()
                .step_by(step)
                .scan(format!("{:?}", expr), |string, (idx, _)| {
                    let slice = &string[prev..idx];
                    prev = idx;
                    Some(format!("{}{}", " ".repeat(indents), slice.to_string()))
                })
                .collect::<Vec<String>>()
                .join(format!("\n{0:<1$}", ' ', indents).as_str())
        )
        .as_str(),
    );
    let slice = &expr[prev..expr.len()];
    string.push_str(format!("\n{0:<1$}{2}", ' ', indents * 2, slice).as_str());
    // println!("{}", string);
    string.replace('"', "")
}

pub fn gcd<T: Div<Output = T> + Rem<Output = T> + Ord + PartialEq + Default + Clone + Debug>(
    a: T,
    b: T,
) -> T {
    let max = a.clone().max(b.clone());
    let min = a.clone().min(b.clone());

    misc(format!("{}a: {a:?} b: {b:?}", color!()));

    if min == T::default() {
        max
    } else {
        gcd(min.clone(), max % min)
    }
}

// pub fn lerp<T: Div + Mul>(value: T, )

pub fn progress<T: Div + Display + Copy + Into<f64> + Into<u32>>(
    dividend: T,
    divisor: T,
    row: u16,
) {
    let quotient = Into::<f64>::into(dividend) / Into::<f64>::into(divisor);
    let rem: u32 = Into::<u32>::into(dividend) % Into::<u32>::into(divisor);
    let subtrahend = f64::floor(rem as f64 / Into::<f64>::into(divisor) * 8.) as u32;

    let rem_block = format!("{}", char::from_u32(0x258F - subtrahend).unwrap());
    let pc: f64 = quotient * 100.0;
    let text = format!("{:.2}%", pc);
    let (c, r) = size().unwrap();
    let width = c as usize - 7;
    // execute!(stderr(), cursor::Hide, cursor::MoveTo(0, r.min(row)));
    eprint!(
        "{}{}{}{}{text:>7}",
        TextStyle::Cyan,
        repeat_n(
            "\u{2588}",
            (f64::floor(pc as f64 / width as f64 * width as f64)) as usize,
        )
        .collect::<String>(),
        rem_block,
        TextStyle::ResetColor,
    );
}

pub fn out(col: u16, row: u16, s: String) {
    let (c, r) = size().unwrap();

    // execute!(
    //     stderr(),
    //     // cursor::SavePosition,
    //     cursor::MoveTo(col.min(c), row.min(r))
    // );
    eprintln!("{}", s);
    // execute!(stderr(), terminal::Clear(ClearType::UntilNewLine));
}

#[inline(always)]
pub fn print_state(state: &State, ctx: Ctx) {
    let mut text = state_str(state, ctx);

    let mut lines: Vec<String> = text.split("\n").map(str::to_string).collect();

    lines.into_iter().for_each(|line| {
        execute!(
            stderr(),
            SavePosition,
            Print(format!("{line}")),
            RestorePosition,
            MoveDown(1)
        );
    });
    // eprint!("\n");
}

#[inline(always)]
pub fn print_states(state: &State, ctx: Ctx) {
    // execute!(
    //     stderr(),
    //     SavePosition,
    //     MoveTo(0, 4),
    //     Clear(ClearType::CurrentLine)
    // );
    eprint!("{}", color!());
    misc(state_str(state, ctx));
    state
        .children(ctx)
        .iter()
        .cloned()
        .for_each(|ctx| print_states(state, ctx));
    // execute!(stderr(), Clear(ClearType::UntilNewLine), RestorePosition);
}

pub fn print_slice(ctx: Ctx, col: usize, state: &mut State) {
    let pcs = state
        .pcs(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();
    let lengths = state
        .lengths(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();
    let velocities = state
        .velocities(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();
    let registers = state
        .registers(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();
    let tempos = state
        .tempos(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();
    let programs = state
        .programs(ctx)
        .and_then(|ts| ts.get(col).cloned())
        .unwrap_or_default();

    execute!(
        stderr(),
        SavePosition,
        Print(format!("{ctx:?}")),
        RestorePosition,
        MoveDown(1),
        SavePosition,
        Print(format!("\t\tCOL: {col}")),
        RestorePosition,
        MoveDown(1)
    );

    zip(
        pcs.into_iter(),
        zip(
            lengths.into_iter(),
            zip(
                velocities.into_iter(),
                zip(
                    registers.into_iter(),
                    zip(tempos.into_iter(), programs.into_iter()),
                ),
            ),
        ),
    )
    .enumerate()
    .for_each(
        |(index, (pc, (length, (velocity, (register, (tempo, program))))))| {
            let row_str = format!("ROW {index} ");
            execute!(
                stderr(),
                SavePosition,
                // Print(row_str.clone()),
                Print(format!("PC:  {}", pc.as_f64())),
                RestorePosition,
                MoveDown(1),
                SavePosition,
                // MoveRight(row_str.len() as u16),
                Print(format!("LEN: {}", length.as_u64())),
                RestorePosition,
                MoveDown(1),
                SavePosition,
                // MoveRight(row_str.len() as u16),
                Print(format!("VEL: {velocity:?}")),
                RestorePosition,
                MoveDown(1),
                SavePosition,
                // MoveRight(row_str.len() as u16),
                Print(format!("REG: {}", register.as_i8())),
                RestorePosition,
                MoveDown(1),
                SavePosition,
                // MoveRight(row_str.len() as u16),
                Print(format!("TMP: {}", tempo.clone().0.to_u64().unwrap())),
                RestorePosition,
                MoveDown(1),
                SavePosition,
                // MoveRight(row_str.len() as u16),
                Print(format!("PRG: {}", program.0)),
                RestorePosition,
                MoveUp(5)
            );
        },
    );
}

pub fn print_slices(ctx: Ctx, from: u16, to: u16, state: &mut State) {
    let (cols, rows) = size().unwrap();
    let col_width = cols / 10;

    execute!(
        stderr(),
        SavePosition,
        Clear(ClearType::FromCursorDown),
        MoveTo(0, 24),
        // RestorePosition,
        // Print(row_str.clone()),
        Print(format!("PC:  ")),
        MoveToColumn(0),
        MoveDown(1),
        // MoveRight(row_str.len() as u16),
        Print(format!("LEN: ")),
        MoveToColumn(0),
        MoveDown(1),
        // MoveRight(row_str.len() as u16),
        Print(format!("VEL: ")),
        MoveToColumn(0),
        MoveDown(1),
        // MoveRight(row_str.len() as u16),
        Print(format!("REG: ")),
        MoveToColumn(0),
        MoveDown(1),
        // MoveRight(row_str.len() as u16),
        Print(format!("TMP: ")),
        MoveToColumn(0),
        MoveDown(1),
        // MoveRight(row_str.len() as u16),
        Print(format!("PRG: ")),
        // RestorePosition,
        // MoveUp(5),
        // MoveRight(5)
    );
    let range = to - from;
    for idx in 0..range {
        let col = from + idx;
        let x = (idx * col_width);

        execute!(stderr(), SavePosition, MoveTo(5 + x, 24 + (idx / 10) * 6));
        print_slice(ctx, col as usize, state);
        // execute!(stderr(), Clear(ClearType::FromCursorDown));
    }

    execute!(stderr(), Clear(ClearType::FromCursorDown), RestorePosition);
}

pub fn print_stacks(state: &mut State) {
    misc(format!(
        "{}EXPS:\n{}\nSTACK:\n{}\n",
        color!(),
        state
            .exps()
            .iter()
            .map(Exp::to_string)
            .collect::<Vec<String>>()
            .join(", "),
        state
            .stack()
            .iter()
            .map(Exp::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    ));
}

pub fn print_notes(ctx: Ctx, state: &mut State) {
    // eprintln!("{}PRINT NOTES {ctx:?}", color!());
    let notes = note_str(ctx, state);
    // dbg!(&notes);
    misc(format!("\n{}\n", notes));
}

pub fn note_str(ctx: Ctx, state: &mut State) -> String {
    let rows = state
        .lengths(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .max_by(|a, b| a.len().cmp(&b.len()))
        .unwrap_or_default()
        .len();
    let cols = state.lengths(ctx).unwrap_or_default().len();
    let mut lengths = state
        .lengths(ctx)
        .and_then(|lengths| {
            if lengths.is_empty() {
                None
            } else {
                Some(lengths)
            }
        })
        .unwrap_or(vec![
            from_fn(|| Some(state.get::<Length>(ctx)))
                .take(cols)
                .flatten()
                .collect(),
        ]);

    let mut tempos: Vec<Vec<Tempo>> = state
        .tempos(ctx)
        .and_then(|tempos| {
            if tempos.is_empty() {
                None
            } else {
                Some(tempos)
            }
        })
        .unwrap_or(vec![
            from_fn(|| Some(state.get::<Tempo>(ctx)))
                .take(cols)
                .flatten()
                .collect(),
        ])
        .iter()
        .cloned()
        .cycle()
        .take(cols)
        .collect();

    // dbg!(rows, cols, &lengths, &tempos);
    // dbg!(rows, cols, vec![state.get::<Length>(ctx)], vec![state.get::<Tempo>(ctx)], state.pcs(ctx).unwrap_or_default());
    let mut lines = Vec::<Vec<String>>::new();
    lines.resize(
        rows,
        repeat_n("\u{0020}\u{0020}\u{0020}".to_string(), cols).collect::<Vec<String>>(),
    );

    let mut beam = false;

    lengths
        .iter()
        .cloned()
        .enumerate()
        .zip(
            tempos.iter().cloned().chain(
                vec![state.get::<Tempo>(ctx)]
                    .iter()
                    .rev()
                    .cloned()
                    .take(1)
                    .cycle(),
            ),
        )
        .zip(
            state
                .pcs(ctx)
                .and_then(|pcs| {
                    if pcs.is_empty() {
                        Some(vec![
                            vec![Pc::None]
                                .iter()
                                .cloned()
                                .cycle()
                                .take(rows)
                                .collect::<Vec<Pc>>(),
                        ])
                    } else {
                        Some(pcs)
                    }
                })
                .unwrap_or_default()
                .iter()
                .cycle()
                .take(cols),
        )
        .for_each(|(((col, lengths), tempos), pcs)| {
            // dbg!(col, &lengths, &tempos, &pcs);
            lengths
                .iter()
                .cloned()
                .enumerate()
                .zip(tempos.iter().cycle())
                .zip(pcs.iter())
                .for_each(|(((row, length), tempo), pc)| {
                    let beam_ = if length.as_u64() < tempo.0.to_u64().unwrap() {
                        if !beam {
                            beam = !beam;
                            SB
                        } else {
                            '\u{200b}'
                        }
                    } else {
                        if beam { EB } else { '\u{200b}' }
                    };
                    // dbg!(row, &length, tempo, pc);
                    if matches!(*pc, Pc::None) {
                        lines[row][col] = format!("{:<3}", length.to_rest(tempo));
                    } else {
                        lines[row][col] = format!("{:<3}", length.to_note(tempo));
                    }
                });
        });

    // dbg!(&lines);

    // let lines = lines
    //     .into_iter()
    //     .map(|row| (row.join("\u{200B}")))
    //     .collect::<Vec<String>>();

    // dbg!(&lines);

    let sanitised: Vec<Vec<String>> = lines
        .iter()
        .map(|lines| {
            lines
                .iter()
                .map(|line| {
                    line.chars()
                        .filter(|ch| {
                            matches!(*ch, '\u{1d13a}'..'\u{1d164}' | '\u{0020}' | '\u{200b}')
                        })
                        .collect::<String>()
                })
                .collect()
        })
        .collect::<Vec<Vec<String>>>();
    // dbg!(&sanitised);

    // let sanitised = sanitised.iter().map(|s| s.chars().count()).collect::<Vec<usize>>();
    // dbg!(sanitised.iter().all(|ln| *ln == 3));

    let cols = size().unwrap().0 as usize;

    let notes = lines
        .into_iter()
        .map(|lines| lines.join(""))
        .collect::<Vec<String>>()
        .join("\n");

    // .join("\n");
    // dbg!(&notes);
    notes
}

#[inline(always)]
pub fn state_str(state: &State, ctx: Ctx) -> String {
    let parent = state.parent(ctx);

    let parent_str = match parent {
        Ctx::Id(_) => format!("{:?} {:?}", parent, state.scope(parent)).to_uppercase(),
        Ctx::Root => format!("ROOT {:?}", state.scope(parent)).to_uppercase(),
        Ctx::None => " ".to_string(),
    };

    let child_str = match ctx {
        Ctx::Id(_) => format!("{:?} {:?}", ctx, state.scope(ctx)).to_uppercase(),
        Ctx::None => format!(" "),
        Ctx::Root => format!("ROOT {:?}", state.scope(ctx)).to_uppercase(),
    };

    // let tempo: Tempo = state.get(ctx);

    let len: usize = 1;

    let pcs: String = state
        .pcs(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|pcs| {
            format!(
                "[{}]",
                pcs.into_iter()
                    .map(|pc| format!(
                        "{0:^1$}",
                        if matches!(pc, Pc::None) {
                            String::from("None")
                        } else {
                            pc.as_f64().to_string()
                        },
                        len
                    ))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let velocities: String = state
        .velocities(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|velocities| {
            format!(
                "[{}]",
                velocities
                    .into_iter()
                    .map(|velocity| format!("{0:^1$}", velocity.0, len))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let registers: String = state
        .registers(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|registers| {
            format!(
                "[{}]",
                registers
                    .into_iter()
                    .map(|register| format!("{0:^1$}", register.as_i8(), len))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let lengths: String = state
        .lengths(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|lengths| {
            format!(
                "[{}]",
                lengths
                    .into_iter()
                    .map(|length| format!("{0:^1$}", length.as_u64(), len))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let tempos: String = state
        .tempos(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|tempos| {
            format!(
                "[{}]",
                tempos
                    .into_iter()
                    .map(|tempo| format!(
                        "\u{1d15f} = {0:^1$}",
                        60_000_000 / tempo.0.to_u64().unwrap(),
                        len
                    ))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let programs: String = state
        .programs(ctx)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(|programs| {
            format!(
                "[{}]",
                programs
                    .into_iter()
                    .map(|program| format!("{0:^1$}", program.0, len))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<String>>()
        .join(" ");
    let mut text = format!(
        // "{:?} {:?} -> {:?} {:?}\nProg: [{programs}]\nPCs : [{pcs}]\nVel : [{velocities}]\nReg : [{registers}]\nLens: [{lengths}]\nTmps: [{tempos}]\nBnds: {:?}\nChil: {:?}\n",
        "{} -> {}\nPCs : [{pcs}]\nVel : [{velocities}]\nReg : [{registers}]\nLens: [{}]\nProg: [{programs}]\nTmps: [{}]\nChil: {:?}\n",
        parent_str,
        child_str,
        // state
        //     .bindings(ctx)
        //     .keys()
        //     .map(|key| key.0.clone())
        //     .collect::<Vec<_>>(),
        lengths,
        tempos,
        state.children(ctx),
    );
    text
}

pub fn info(msg: String) {
    execute!(
        stderr(),
        SavePosition,
        MoveTo(0, 2),
        Clear(ClearType::CurrentLine),
        Print(msg),
        Clear(ClearType::UntilNewLine),
        RestorePosition
    );
}

pub fn status(msg: String) {
    execute!(
        stderr(),
        SavePosition,
        MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        Print(msg),
        Clear(ClearType::UntilNewLine),
        RestorePosition
    );
}

#[inline(always)]
pub fn time_log(msg: String) {
    execute!(
        stderr(),
        SavePosition,
        MoveTo(0, 1),
        Clear(ClearType::CurrentLine),
        Print(msg),
        Clear(ClearType::UntilNewLine),
        RestorePosition
    );
}

pub fn log(msg: String) {
    execute!(
        stderr(),
        SavePosition,
        MoveTo(0, 3),
        Clear(ClearType::CurrentLine),
        Print(msg),
        Clear(ClearType::UntilNewLine),
        RestorePosition
    );
}

#[inline(always)]
pub fn misc(msg: String) {
    execute!(
        stderr(),
        SavePosition,
        MoveTo(0, 4),
        Clear(ClearType::FromCursorDown),
    );
    msg.split("\n").into_iter().for_each(|line| {
        execute!(
            stderr(),
            Print(line),
            Clear(ClearType::UntilNewLine),
            MoveToColumn(0),
            MoveDown(1)
        );
    });
    // execute!(stderr(), Clear(ClearType::FromCursorDown), RestorePosition);
}

fn to_edges(ctx: Ctx, state: &State, layers: usize) -> Vec<(u32, u32)> {
    // eprintln!("{}edge {ctx:?}{}", TextStyle::IntenseRed, TextStyle::ResetColor);
    if layers == 0 {
        return vec![];
    }
    let mut children = state.children(ctx);
    let mut edges = Vec::<(u32, u32)>::new();
    children.iter().cloned().for_each(|ctx_| {
        &mut edges.push((ctx.to_u32(), ctx_.to_u32()));
        &mut edges.extend(to_edges(ctx_, state, layers - 1));
    });

    edges.sort();

    edges
}

pub fn graph(state: &State, mut ctx: Ctx, layers: usize) {
    if ctx == Ctx::None {
        ctx = Ctx::Root;
    }

    let width = size().unwrap().0;

    let edges = to_edges(ctx, state, layers);

    let mut layouts = from_edges(
        edges.as_slice(),
        &Config {
            minimum_length: 1,
            vertex_spacing: 1.,
            dummy_vertices: false,
            dummy_size: 0.5,
            ranking_type: RankingType::MinimizeEdgeLength,
            ..Config::default()
        },
    );

    if let Some((layout, width, height)) = layouts.iter_mut().next() {
        layout.iter_mut().for_each(|(_, (x, _))| {
            *x = *width - *x;
        });

        // dbg!(&layout);
        // layout.reverse();
        // dbg!(&layout);
        // layout.sort_by(|(lhs, (_, _)), (rhs, (_, _))| lhs.cmp(rhs));
        // dbg!(&layout);
        let (c, r) = size().unwrap();
        let (mut c, mut r) = (c as usize, r as usize);
        let graph_width = layout
            .iter()
            .max_by(|(_, (x1, _)), (_, (x2, _))| {
                (f64::ceil(*x1) as usize).cmp(&(f64::ceil(*x2) as usize))
            })
            .unwrap()
            .1
            .0;

        // dbg!(graph_width, *height);

        let mut columns = (f64::ceil(graph_width).max(1.)) as usize;

        // columns += 1;

        let column_width = (f64::ceil(c as f64 / columns.max(1) as f64) as usize).max(2);
        // dbg!(f64::ceil(graph_width).max(1.), columns, column_width);
        let node_height: usize = 4;
        let height = f64::ceil(*height) as usize * node_height;

        let offset = match column_width % 2 {
            0 => 1,
            1 => 0,
            _ => todo!(),
        };

        let mut table = Vec::<Vec<String>>::from_iter(repeat_n(
            Vec::<String>::from_iter(repeat_n(
                format!(
                    "{}",
                    // "{2}\x1b[0;41m{0:^1$}{2}",
                    "\u{00A0}".repeat(column_width - offset),
                    // column_width,
                    // TextStyle::ResetColor
                ),
                columns,
            )),
            height,
        ));

        // for lines in table.clone() {
        //     let line = lines.join("");
        //     eprintln!("{}", line);
        // }

        // dbg!(table.len(), table[0].len());

        let mut visited = BTreeMap::<usize, (usize, usize)>::new();

        // let mut table_iter = table.iter_mut();

        for (node, (x, y)) in layout {
            let ctx = Ctx::Id(*node);

            let ctx = if *node == 0 {
                Ctx::Root
            } else {
                Ctx::Id(*node)
            };

            let mut x_ = x.clone();
            let parent = state.parent(ctx);
            let gp = state.parent(parent);
            let pibling_count = if parent.to_usize() == 0 {
                1
            } else {
                state.children(gp).len()
            };
            let sibling_count = if *node == 0 {
                1
            } else {
                state.children(parent).len()
            };

            let (x, y) = ((
                (f64::floor(x_ / *width * columns as f64) as usize).min(table[0].len().max(1) - 1),
                ((f64::floor(*y) as usize).min(table.len().max(1) - 1)),
            ));
            // dbg!(x, table.len(), y, table[0].len());
            // eprintln!(
            //     "{}{:?} -> {ctx:?} x: {x} y: {y}{}",
            //     TextStyle::IntenseRed,
            //     parent,
            //     TextStyle::ResetColor
            // );

            // dbg!(&visited);

            let table = &mut table;

            let scope = match state.scope(ctx) {
                Scope::Sequence => "SEQ",
                Scope::Stack => "ST",
                _ => " ",
            };
            let node_id = ctx.to_usize();
            // dbg!(ctx);
            let ctx_str = if ctx == Ctx::Root {
                format!("{0:^1$}", "ROOT", column_width - offset + 1)
            } else {
                format!("{0:^1$}", node_id, column_width - offset)
            };

            // dbg!(columns, column_width);
            let len = state.children(ctx).len();
            match len {
                0 => (),
                1.. => {
                    *get_branch_mut(x, y * node_height + 3, table) = format!(
                        "{0:^1$}|{2:^3$}",
                        "\u{a0}".repeat(column_width / 2 - offset),
                        column_width / 2,
                        "\u{a0}".repeat(column_width / 2 - offset),
                        column_width / 2 - offset
                    );
                }
            }

            // * node_height - 1
            if y > 0 {
                // *
                *get_branch_mut(x, y * node_height, table) =
                    format!("{0:^1$}", "|", column_width - offset);
            }
            let sibling =
                visited
                    .iter()
                    .fold((*node, (x, y)), |(node, (x, y)), (node_, (x_, y_))| {
                        if *y_ == y
                            && *x_ < x
                            && state.parent(ctx).to_usize()
                                == state.parent(Ctx::from(*node_)).to_usize()
                        {
                            (*node_, (*x_, *y_))
                        } else {
                            (node, (x, y))
                        }
                    });

            // dbg!(&node, &sibling);
            if sibling != (*node, (x, y)) {
                let (sibling_node, (sibling_x, sibling_y)) = sibling;
                let mut default_vec = Vec::<String>::new();
                let mut default_string = String::new();
                let node_branch = get_branch_mut(x, y * node_height - 1, table);
                *node_branch = format!(
                    "{0:<1$}",
                    "_".repeat(column_width / 2 - offset),
                    column_width / 2 - offset
                );
                let mut space = "\u{a0}".repeat(column_width / 2 - 1 - offset);
                let (x_, _) = visited
                    .get(&state.parent(ctx).to_usize())
                    .cloned()
                    .unwrap_or_default();
                if x_ == x {
                    *node_branch += "|";
                } else {
                    *node_branch += "_";
                    // space = "\u{a0}".repeat(column_width / 2 );
                }

                *node_branch += space.as_str();
                let leftmost_sibling_branch =
                    get_branch_mut(sibling_x, sibling_y * node_height - 1, table);

                let branch_len = leftmost_sibling_branch.len();
                let mut branch: String = space;

                // eprintln!("{}{}", color!(), sibling);
                // dbg!(ctx.to_usize(), x, y, node, x_, y_);

                if visited.iter().any(|(node, (x_, y_))| {
                    state.parent(ctx).to_usize() == Ctx::from(*node).to_usize() && *x_ == x
                }) {
                    branch += "|";
                } else {
                    branch += "_";
                }
                branch += "_".repeat(column_width / 2 - offset).as_str();
                // dbg!(size_of_val(&sibling.as_bytes()), column_width / 2, size_of_val(&'\u{a0}') );
                *leftmost_sibling_branch = branch;

                let mut x_ = sibling_x + 1;

                while x_ < x {
                    let branch = get_branch_mut(x_, node_height * sibling_y - 1, table);
                    *branch = "_".repeat(column_width / 2 - offset);
                    if let Some(parent) = visited.get(&parent.to_usize()) {
                        if parent.0 == x_ {
                            *branch += "|";
                        }
                    } else {
                        *branch += "_";
                    }
                    *branch += "_".repeat(column_width / 2 - offset).as_str();
                    x_ += 1
                }

                // eprintln!("{}{}", color!(), sibling)
            }

            *get_branch_mut(x, y * node_height + 1, table) =
                format!("{0:^1$}", ctx_str, column_width - offset);

            *get_branch_mut(x, y * node_height + 2, table) =
                format!("{0:^1$}", scope, column_width - offset);

            // dbg!(&visited);
            visited.insert(ctx.to_usize(), (x, y));
        }

        // drop(&mut *table);

        // for mut row in &mut *table {
        //     row.reverse();
        // }

        execute!(stderr(), SavePosition, MoveTo(0u16, height as u16 / 3 as u16));
        for lines in table.clone() {
            let line = lines.join("");
            if line.chars().all(|c| c == '\u{a0}' || c == ' ') {
                continue;
            }
            eprintln!("{0:^1$}", line, size().unwrap().0 as usize);
        }

        execute!(stderr(), RestorePosition);

    }
}

fn get_branch_mut(x: usize, y: usize, table: &mut Vec<Vec<String>>) -> &mut String {
    // dbg!(x, y);
    &mut table[y][x]
}

pub fn trace(style: TextStyle) {
    let s = format!(
        "{style}{}{}",
        std::backtrace::Backtrace::capture()
            .to_string()
            .split("\n")
            .filter(|s| s.contains("dsch") || s.contains("compiler"))
            .collect::<Vec<&str>>()
            .join("\n"),
        TextStyle::ResetColor
    );
    misc(s);
}

pub fn bezier<T: Clone + Into<f64>>(pts: Vec<T>, t: f64) -> f64 {
    match pts.len() {
        ..=1 => pts[0].clone().into(),
        2 => (1.0 - t) * pts[0].clone().into() + t * pts[1].clone().into(),
        len @ 2.. => bezier(
            vec![
                bezier(pts[..len - 1].to_vec(), t),
                bezier(pts[1..].to_vec(), t),
            ],
            t,
        ),
    }
}

// pub fn print_timeline(state: &mut State) {
//     state
//         .timeline()
//         .clone()
//         .iter()
//         .cloned()
//         .enumerate()
//         .for_each(|(idx, (t, ctx))| {
//             let lengths = state
//                 .lengths(ctx)
//                 .unwrap_or_default()
//                 .iter()
//                 .cloned()
//                 .map(|lengths| {
//                     // dbg!(&length);
//                     lengths
//                         .iter()
//                         .map(|length| format!("{}", length.as_u64()))
//                         .collect::<Vec<_>>()
//                         .join(" ")
//                 })
//                 .collect::<Vec<String>>()
//                 .join(", ");
//             // eprintln!(
//             //     "{}{idx}: T:{t} ID:{} DUR: {lengths}",
//             //     color!(),
//             //     ctx.to_usize(),
//             // );
//         });
// }

pub fn pause() {
    let _ = std::io::stdin().read_line(&mut String::new());
}

pub fn flatten<T: Clone + Debug>(data: &mut Vec<Vec<T>>) {
    // dbg!(&data);
    if data.len() > 0 {
        *data = vec![data.iter().cloned().flatten().collect()];
    }
    // dbg!(&data);
}

pub fn convert_vec<T: Clone + From<U>, U: Clone + From<T>>(vec: Vec<T>) -> Vec<U> {
    vec.into_iter()
        .map(|t| <U as From<T>>::from(t))
        .collect::<Vec<U>>()
}

/// B-Spline
#[allow(non_snake_case)]
pub fn B(i: usize, p: usize, t: f64, ts: &Vec<f64>) -> f64 {
    if p == 0 {
        1.0
    } else {
        let t0 = ts[i];
        let tp = ts[i + p];
        let tO = ts[i + p + 1];

        if t < t0 || t >= tO {
            0.0
        } else {
            (t - t0) / (tp - t0) * B(i, p - 1, t, ts)
                + (tO - t) / (tO - tp) * B(i + 1, p - 1, t, ts)
        }
    }
}

#[allow(non_snake_case)]
pub fn C(p: Vec<f64>) -> f64 {
    let j = p.len();
    todo!()
}

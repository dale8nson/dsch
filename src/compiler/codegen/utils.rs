use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    io::stderr,
    iter::repeat_n,
    ops::{Deref, Div, Mul, Rem},
};

use crate::compiler::{
    ast::{utils::abs_to_f64, *},
    codegen::{state::State, *},
};

use colonnade::{Alignment, Colonnade};
use crossterm::{
    cursor::{self, MoveToNextLine, RestorePosition, position},
    execute,
    terminal::{self, ClearType, size},
};
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

pub fn to_length(frac: Absolute, tempo: Mpb) -> MicroSeconds {
    let fr = abs_to_f64(frac);
    MicroSeconds(f64::round(fr / 4 as f64 * tempo.0 as f64) as u64)
}

pub fn duration_to_micros(minutes: Absolute, seconds: Absolute) -> MicroSeconds {
    MicroSeconds(f64::round(
        match minutes {
            Absolute::UInt(int) => (int * 60 * 1_000_000) as f64,
            Absolute::Float(float) => float * 1_000_000 as f64,
        } + match seconds {
            Absolute::UInt(int) => (int * 1_000_000) as f64,
            Absolute::Float(float) => float * 1_000_000 as f64,
        },
    ) as u64)
}

pub fn length_to_ticks(length: Length, tempo: Mpb) -> u64 {
    match length {
        Length::None => 0,
        Length::MicroSeconds(micros) => {
            f64::round(micros as f64 / tempo.0 as f64 * PPQ.as_int() as f64) as u64
        }
    }
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

pub fn gcd<
    T: Div<Output = T> + Rem<Output = T> + Ord + PartialEq + Default + Clone + Copy + Debug,
>(
    a: T,
    b: T,
) -> T {
    let max = a.max(b);
    let min = a.min(b);

    if min == T::default() {
        max
    } else {
        gcd(min, max % min)
    }
}

pub fn lcd<
    T: Mul<Output = T> + Div<Output = T> + Rem<Output = T> + Ord + PartialEq + Default + Debug + Copy,
>(
    a: T,
    b: T,
) -> T {
    let g = gcd(a, b);
    eprintln!(
        "{}gcd: {g:?}{}",
        TextStyle::IntensePurple,
        TextStyle::ResetColor
    );
    a * b / g
}

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

pub fn print_state(state: &State, ctx: Ctx) {
    let parent = state.parent(ctx);
    let mut text = format!(
        "\x1b[1;36m{:?} {:?} -> {:?} {:?}\x1b[0m\n\x1b[0;36m\nProg: {:?}\nPCs : {:?}\nVel : {:?}\nReg : {:?}\nLens: {:?}\nTmps: {:?}\nChil: {:?}\x1b[0m\n",
        parent,
        state.scope(parent),
        ctx,
        state.scope(ctx),
        state.programs(ctx),
        state.pcs(ctx),
        state.velocities(ctx),
        state.registers(ctx),
        state.lengths(ctx),
        state.tempos(ctx),
        // state.bindings(ctx),
        state.children(ctx),
    );

    let len = state.exps().len() - state.len();

    state.exps().iter().skip(len).for_each(|exp| {
        text += format!(
            "{}{exp}{}\n",
            TextStyle::IntenseYellow,
            TextStyle::ResetColor
        )
        .as_str();
    });

    let mut lines: Vec<String> = text.split("\n").map(str::to_string).collect();
    let (col, row) = (25, 5);

    lines.into_iter().for_each(|line| {
        eprintln!("{line}");
    });
    eprint!("\n");
}

fn to_edges(ctx: Ctx, state: &State) -> Vec<(u32, u32)> {
    // eprintln!("{}edge {ctx:?}{}", TextStyle::IntenseRed, TextStyle::ResetColor);
    let mut edges = Vec::<(u32, u32)>::new();
    let mut children = state.children(ctx);

    let mut edges: Vec<(u32, u32)> = edges
        .into_iter()
        .chain(children.iter().flat_map(|ctx_| {
            vec![(ctx.to_u32(), ctx_.to_u32())]
                .into_iter()
                .chain(to_edges(*ctx_, state))
        }))
        .collect();

    edges.sort();

    edges
}

pub fn graph(state: &mut State, ctx: Ctx) {
    let width = size().unwrap().0;

    let edges = to_edges(ctx, state);

    let mut layouts = from_edges(
        edges.as_slice(),
        &Config {
            minimum_length: 1,
            vertex_spacing: 1.,
            dummy_vertices: true,
            dummy_size: 0.5,
            ranking_type: RankingType::MinimizeEdgeLength,
            ..Config::default()
        },
    );

    if let Some((layout, width, height)) = layouts.iter_mut().next() {
        // dbg!(&layout);
        // layout.reverse();
        // dbg!(&layout);
        layout.sort_by(|(lhs, (_, _)), (rhs, (_, _))| lhs.cmp(rhs));
        // dbg!(&layout);
        let (c, r) = size().unwrap();
        let (mut c, mut r) = (c as usize, r as usize);
        let width = layout
            .iter()
            .max_by(|(_, (x1, _)), (_, (x2, _))| {
                (f64::ceil(*x1) as usize).cmp(&(f64::ceil(*x2) as usize))
            })
            .unwrap()
            .1
            .0;

        let mut columns = f64::ceil(width) as usize;

        columns += 1;

        let column_width = f64::floor(c as f64 / columns as f64) as usize;
        let height = f64::ceil(*height) as usize * 3;

        let mut table = Vec::<Vec<String>>::from_iter(repeat_n(
            Vec::<String>::from_iter(repeat_n("\u{00A0}".repeat(column_width / 2), columns)),
            height,
        ));

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

            let (x, y) = (f64::round(x_) as usize, f64::round(*y) as usize);
            // eprintln!(
            //     "{}{:?} -> {ctx:?} x: {x} y: {y}{}",
            //     TextStyle::IntenseRed,
            //     parent,
            //     TextStyle::ResetColor
            // );

            // dbg!(&visited);

            let scope = match state.scope(ctx) {
                Scope::Sequence => "SEQ",
                Scope::Stack => "ST",
                _ => " ",
            };
            let node_id = ctx.to_usize();
            // dbg!(ctx);
            let ctx_str = if ctx == Ctx::Root {
                "ROOT".to_string()
            } else {
                format!("{}", node_id)
            };
            let table = &mut table;
            *(table
                .iter_mut()
                .nth(y * 3 + 1)
                .unwrap_or(&mut Vec::new())
                .iter_mut()
                .nth(x)
                .unwrap_or(&mut String::new())) = format!("{0:^1$}", ctx_str, column_width);
            *(table
                .iter_mut()
                .nth(y * 3 + 2)
                .unwrap_or(&mut Vec::new())
                .iter_mut()
                .nth(x)
                .unwrap_or(&mut String::new())) = format!("{0:^1$}", scope, column_width);
            let mut left_branch =
                (repeat_n('\u{00A0}', column_width / 2).collect::<String>() + "|");
            let mut right_branch =
                (repeat_n('\u{00A0}', column_width / 2 - 1).collect::<String>() + "|");
            let mut branches = &mut String::new();
            visited.insert(*node, (x, y));
            // dbg!(&visited);
            for (node, (x_, y_)) in &visited {
                // eprintln!(
                //     "{}NODE: {node} x_: {x_} y_: {y_}{}",
                //     TextStyle::IntenseBoldBlue,
                //     TextStyle::ResetColor
                // );
                let parent = state.parent(ctx);
                let node_ctx = Ctx::Id(*node);
                // dbg!(parent, node_ctx);
                // dbg!(ctx, node, x, *x_, y, *y_);
                if *y_ == y {
                    if x < *x_ {
                        left_branch =
                            (repeat_n('_', column_width / 2).collect::<String>() + "|")
                    } else if x > *x_ {
                        right_branch = repeat_n('_', column_width / 2 - 1)
                            .collect::<String>()
                            ;
                        // dbg!();
                        // eprintln!("{branches}");
                    }
                }
            }
            *branches = ((branches.clone() + left_branch.as_str()) + "|") + right_branch.as_str();
            // dbg!();
            // eprintln!("{branches}");
            // else if node_ctx == parent && dbg!(x == *x_) && dbg!((y - *y_)) <= 2 {
            //     branches = &mut table[y * 3][x];
            //     *branches = format!("{0:^1$}", "|", column_width);
            // }
            // if dbg!(node_ctx == parent) {

            // }
        }

        // drop(&mut *table);

        for mut row in &mut *table {
            row.reverse();
        }

        if let Ok(mut colonnade) = Colonnade::new(columns, c) {
            colonnade.hyphenate(false);
            colonnade.fixed_width(column_width);
            colonnade.padding_horizontal(1);
            colonnade.left_margin(0);
            colonnade.alignment(Alignment::Center);
            // execute!(stderr(), cursor::MoveTo(0, 0));
            let mut table = colonnade.tabulate(table).unwrap();

            for lines in table {
                eprintln!(
                    "{}{lines}{}",
                    TextStyle::IntenseBoldPurple,
                    TextStyle::ResetColor
                );
            }
            eprintln!("");
        }
    }
}

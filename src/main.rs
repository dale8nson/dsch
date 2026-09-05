#![allow(unused)]
pub mod compiler;
mod pest_parser;

use std::io::stderr;

use crate::compiler::{ast::*, composer::compose_program, scheduler::schedule};
use crate::pest_parser::{CLParser, Rule, parse_program};

use clap::Parser;
use crossterm::{cursor, execute};
use pest::{Parser as PestParser, set_error_detail};

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: String,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    set_error_detail(true);

    let args = Args::parse();

    let src = std::fs::read_to_string(format!("{}.dsch", args.input))?;
    let pairs = CLParser::parse(Rule::program, &src)?.next().unwrap();
    let ast = parse_program(pairs)?;
    let state = compose_program(ast);
    let smf = schedule(state);
    let _ = smf.save(format!("{}.mid", args.input));

    execute!(stderr(), cursor::Show);

    Ok(())
}

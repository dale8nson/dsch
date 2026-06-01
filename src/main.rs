#![allow(unused)]
mod compiler;
mod pest_parser;

use crate::compiler::{ast::*, composer::compose_program, scheduler::schedule};
use crate::pest_parser::{CLParser, Rule, parse_program};

use clap::Parser;
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
    // dbg!(&src);
    let pairs = CLParser::parse(Rule::program, &src)?.next().unwrap();
    // dbg!(&pairs);
    let ast = parse_program(pairs)?;
    // println!("{ast:#?}");
    // let composition = Composer::new(State::default()).compose_program(ast);
    //
    let state = compose_program(ast);
    let smf = schedule(state);
    let _ = smf.save(format!("{}.mid", args.input));

    Ok(())
}

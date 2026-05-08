mod compiler;
mod pest_parser;

use crate::compiler::composer::State;
use crate::compiler::{ast::*, composer::compose_program};
use crate::pest_parser::{CLParser, Rule, parse_program};
use pest::{Parser, set_error_detail};

use std::{
    iter::IntoIterator,
    ops::{Index, IndexMut},
    str::FromStr,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    set_error_detail(true);

    let src = std::fs::read_to_string("test.dsch")?;
    // dbg!(&src);
    let pairs = CLParser::parse(Rule::program, &src)?.next().unwrap();
    // dbg!(&pairs);
    let ast = parse_program(pairs)?;
    // println!("{ast:#?}");
    // let composition = Composer::new(State::default()).compose_program(ast);
    //
    let mut state = compose_program(ast);

    Ok(())
}

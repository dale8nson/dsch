#![allow(unused)]
use pest::{
    iterators::{Pair, Pairs},
    set_error_detail,
};
pub use pest_derive::Parser;

use crate::compiler::{ast::*, functional::Monad};

use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[grammar = "../grammar.pest"]
pub struct CLParser;

type Rules<'a> = Pairs<'a, Rule>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn parse_program<'a>(mut pair: Pair<'a, Rule>) -> Result<Program> {
    let rule = pair.as_rule();
    let inner = pair.into_inner();
    let exps = match rule {
        Rule::exps => parse_exps(inner)?,
        _ => unreachable!(),
    };

    Ok(Program { exps })
}

fn parse_exps(mut rules: Rules) -> Result<Vec<Exp>> {
    Ok(rules
        .map(|rule| parse_exp(rule.into_inner()).unwrap())
        .collect())
}

fn parse_exp(mut rules: Rules) -> Result<Exp> {
    // dbg!(&rules);
    // pause();
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    // // dbg!(&rule);
    let inner = next.into_inner();

    Ok(match rule {
        Rule::simple => Exp::Simple(parse_simple(inner)?),
        Rule::compound => Exp::Compound(Box::new(parse_compound(inner)?)),
        _ => unreachable!(),
    })
}

fn parse_simple(mut rules: Rules) -> Result<Simple> {
    let next = rules.next().unwrap();
    // // dbg!(&next);
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::scalar => Ok(Simple::Scalar(parse_scalar(inner)?)),
        Rule::prefix => Ok(Simple::Prefix(parse_prefix(inner)?)),
        Rule::infix => Ok(Simple::Infix(parse_infix(inner)?)),
        Rule::suffix => Ok(Simple::Suffix(parse_suffix(inner)?)),
        Rule::ident => Ok(Simple::Ident(parse_ident(inner)?)),

        _ => unreachable!(),
    }
}

fn parse_scalar(mut rules: Rules) -> Result<Scalar> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::duration => Ok(Scalar::Duration(parse_duration(inner)?)),
        Rule::dynamic => Ok(Scalar::Dynamic(parse_dynamic(inner)?)),
        Rule::frequency => Ok(Scalar::Frequency(parse_frequency(inner)?)),
        Rule::tempo => Ok(Scalar::Tempo(parse_absolute(inner)?)),
        Rule::pure => Ok(Scalar::Pure(parse_pure(inner)?)),
        _ => unreachable!(),
    }
}

fn parse_dynamic(mut rules: Rules) -> Result<String> {
    Ok(String::from(rules.next().unwrap().as_span().as_str()))
}

fn parse_duration(mut rules: Rules) -> Result<Duration> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::fixed => Ok(Duration::Fixed(parse_fixed(inner)?)),
        Rule::fractional => Ok(Duration::Fractional(parse_fractional(inner)?)),
        _ => unreachable!(),
    }
}

fn parse_fixed(mut rules: Rules) -> Result<Fixed> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::seconds => Ok(Fixed {
            minutes: Absolute::UInt(0),
            seconds: parse_seconds(inner)?,
        }),
        Rule::minutes => {
            let secs = if let Some(secs) = rules.next() {
                parse_seconds(secs.into_inner())?
            } else {
                Absolute::UInt(0)
            };
            Ok(Fixed {
                minutes: parse_minutes(inner)?,
                seconds: secs,
            })
        }
        _ => unreachable!(),
    }
}

fn parse_minutes(mut rules: Rules) -> Result<Absolute> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::absolute => Ok(parse_absolute(inner)?),
        _ => unreachable!(),
    }
}

fn parse_seconds(mut rules: Rules) -> Result<Absolute> {
    parse_minutes(rules)
}

fn parse_fractional(mut rules: Rules) -> Result<Fractional> {
    // dbg!(&rules);
    // pause();
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::absolute => Ok(Fractional::Absolute(parse_absolute(inner)?)),
        Rule::tuplet => Ok(Fractional::Tuplet(parse_tuplet(inner)?)),
        _ => unreachable!(),
    }
}

fn parse_tuplet(mut rules: Rules) -> Result<Tuplet> {
    Ok(Tuplet {
        lhs: parse_absolute(rules.next().unwrap().into_inner())?,
        rhs: parse_absolute(rules.next().unwrap().into_inner())?,
    })
}

fn parse_frequency(mut rules: Rules) -> Result<Absolute> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    parse_absolute(inner)
}

fn parse_infix(mut rules: Rules) -> Result<Infix> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();

    match rule {
        Rule::colon => Ok(Infix::Colon),
        Rule::intercalate => Ok(Infix::Intercalate),
        Rule::range => Ok(Infix::Range),
        Rule::plus => Ok(Infix::Plus),
        Rule::minus => Ok(Infix::Minus),
        Rule::mul => Ok(Infix::Mul),
        Rule::div => Ok(Infix::Div),
        _ => unreachable!(),
    }
}

fn parse_pure(mut rules: Rules) -> Result<Pure> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    let inner = next.into_inner();
    match rule {
        Rule::relative => Ok(Pure::Relative(parse_relative(inner)?)),
        Rule::absolute => Ok(Pure::Absolute(parse_absolute(inner)?)),
        _ => unreachable!(),
    }
}

fn parse_relative(mut rules: Rules) -> Result<Relative> {
    let sgn = rules.next().unwrap().as_rule();
    let next = rules.next().unwrap();
    let abs = next.as_rule();

    let sgn = match sgn {
        Rule::plus => Sign::Plus,
        Rule::minus => Sign::Minus,
        _ => unreachable!(),
    };
    match abs {
        Rule::integer => Ok(Relative {
            sign: sgn,
            val: Absolute::UInt(parse_integer(next)?),
        }),
        Rule::float => Ok(Relative {
            sign: sgn,
            val: Absolute::Float(parse_float(next)?),
        }),
        _ => unreachable!(),
    }
}

fn parse_absolute(mut rules: Rules) -> Result<Absolute> {
    let next = rules.next().unwrap();

    let rule = next.as_rule();
    // dbg!(rule);
    // pause();
    // let inner = next.into_inner();

    match rule {
        Rule::integer => Ok(Absolute::UInt(parse_integer(next)?)),
        Rule::float => Ok(Absolute::Float(parse_float(next)?)),
        _ => unreachable!(),
    }
}

fn parse_compound(mut rules: Rules) -> Result<Compound> {
    let next = rules.next().unwrap();

    let rule = next.as_rule();
    // // dbg!(&rule);
    let mut inner = next.into_inner();
    match rule {
        Rule::parens => Ok(Compound::Parens(parse_exps(
            inner.next().unwrap().into_inner(),
        )?)),
        Rule::braces => Ok(Compound::Braces(parse_exps(
            inner.next().unwrap().into_inner(),
        )?)),
        Rule::brackets => Ok(Compound::Brackets(parse_exps(
            inner.next().unwrap().into_inner(),
        )?)),
        Rule::ratio => Ok(Compound::Ratio(parse_ratio(inner)?)),
        Rule::decl => Ok(Compound::Decl(Box::new(parse_decl(inner)?))),

        _ => unreachable!(),
    }
}

fn parse_decl(mut rules: Rules) -> Result<Decl> {
    let ident = rules.next().unwrap().into_inner();
    let exp = rules.next().unwrap().into_inner();
    Ok(Decl {
        ident: parse_ident(ident)?,
        binding: Box::new(parse_exp(exp)?),
    })
}

fn parse_ratio(mut rules: Rules) -> Result<Vec<Absolute>> {
    let absolutes: Vec<Absolute> = rules
        .map(|rule| match rule.as_rule() {
            Rule::integer => Absolute::UInt(parse_integer(rule).unwrap()),
            Rule::float => Absolute::Float(parse_float(rule).unwrap()),
            _ => unreachable!(),
        })
        .collect();
    Ok(absolutes)
}

fn parse_integer(rule: Pair<'_, Rule>) -> Result<u64> {
    Ok(u64::from_str_radix(rule.as_span().as_str(), 10).unwrap())
}

fn parse_float(rule: Pair<'_, Rule>) -> Result<f64> {
    f64::from_str(rule.as_span().as_str()).map_err(|err| err.into())
}

fn parse_ident(mut rules: Rules) -> Result<Ident> {
    let next = rules.next().unwrap();
    Ok(Ident(String::from(next.as_span().as_str())))
}

fn parse_prefix(mut rules: Rules) -> Result<Prefix> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    // dbg!(&rule);
    // pause();
    Ok(match rule {
        Rule::dur => Prefix::Dur,
        Rule::pc => Prefix::Pc,
        Rule::reg => Prefix::Reg,
        Rule::rest => Prefix::Rest,
        _ => unreachable!(),
    })
}

fn parse_suffix(mut rules: Rules) -> Result<Suffix> {
    let next = rules.next().unwrap();
    let rule = next.as_rule();
    Ok(match rule {
        Rule::amp => Suffix::Amp,
        Rule::bpm => Suffix::Bpm,
        Rule::freq => Suffix::Freq,
        _ => unreachable!(),
    })
}

fn pause() {
    let _ = std::io::stdin().read_line(&mut String::new());
}

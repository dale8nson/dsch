pub mod ast;
pub mod codegen;
pub mod composer;
pub mod functional;
pub mod scheduler;


use codegen::utils::TextStyle::*;

fn error(msg: String) {
    eprintln!("{IntenseBoldRed}ERROR: {}{ResetColor}", msg);
}

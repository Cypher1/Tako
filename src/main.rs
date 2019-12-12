use std::env;
use std::fs::File;
use std::io::prelude::*;

mod ast;
mod wasm;
mod interpreter;
mod parser;
mod tokens;
mod tree;

use ast::Visitor;
use wasm::Compiler;
use interpreter::Interpreter;

fn main() -> std::io::Result<()> {
    let all_args: Vec<String> = env::args().collect();
    let args: Vec<String> = all_args[1..].to_vec();
    let mut files: Vec<String> = vec![];
    let mut interactive = false;
    for f in args {
        match f.as_str() {
            "-i" => {
                interactive = true;
                files.push("/dev/stdin".to_string());
            }
            "-r" => interactive = true,
            _ => files.push(f),
        }
    }
    for f in files {
        work(f, interactive)?
    }
    Ok(())
}

fn work(filename: String, interactive: bool) -> std::io::Result<()> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    // println!("Content: '\n{}'", contents);

    let ast = parser::parse(contents);

    if interactive {
        println!("R: {:?}", ast);

        let mut interp = Interpreter::default();
        match interp.visit_root(&ast) {
            Ok(res) => {
                println!("{:?}", res);
            },
            Err(err) => {
                println!("{:?}", err);
            }
        }
        return Ok(());
    }
    let mut comp = Compiler::default();
    match comp.visit_root(&ast) {
        Ok(res) => {
            println!("{}", res);
        },
        Err(err) => {
            println!("{:?}", err);
        }
    }
    return Ok(());
}

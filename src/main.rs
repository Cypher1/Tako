use std::env;
use std::fs::File;
use std::io::prelude::*;
#[macro_use]
mod map_macros;

mod ast;
mod parser;
mod tokens;
mod tree;
mod location;
mod types;

mod wasm;
mod interpreter;
mod pretty_print;
mod rescoper;

use wasm::Compiler;
use pretty_print::PrettyPrint;
use ast::Visitor;
use interpreter::Interpreter;
use rescoper::ReScoper;

struct Options {
    files: Vec<String>,
    interactive: bool,
    show_ast: bool,
    show_full_ast: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
        files: vec![],
        interactive: false,
        show_ast: false,
        show_full_ast: false,
        }
    }
}

fn main() -> std::io::Result<()> {
    let all_args: Vec<String> = env::args().collect();
    let args: Vec<String> = all_args[1..].to_vec();
    let mut opts = Options::default();
    for f in args {
        if &f.chars().next() != &Some('-') {
            opts.files.push(f);
        } else {
            match f.as_str() {
                "-i" => {
                    opts.interactive = true;
                    opts.files.push("/dev/stdin".to_string());
                }
                "-r" => opts.interactive = true,
                "--ast" => opts.show_ast = true,
                "--full_ast" => opts.show_full_ast = true,
                _ => {
                    println!("unexpected flag '{}'", f);
                    return Ok(());
                },
            }
        }
    }
    for f in opts.files.iter() {
        work(&f, &opts)?
    }
    Ok(())
}

fn work(filename: &String, opts: &Options) -> std::io::Result<()> {
    let mut contents = String::new();
    let mut file = File::open(filename.clone())?;
    println!("Filename: '{}'", filename);

    file.read_to_string(&mut contents)?;
    // println!("Content: '\n{}'", contents);

    let ast = parser::parse_file(filename.clone(), contents);

    let mut scoper = ReScoper::default();
    let scoped = scoper.visit_root(&ast).expect("failed scoping");

    if opts.show_full_ast {
        println!("debug ast: {:#?}", scoped);
    }
    if opts.show_ast {
        println!("{}", scoped);
    }

    if opts.interactive {
        let mut interp = Interpreter::default();
        match interp.visit_root(&scoped) {
            Ok(res) => {
                let mut ppr = PrettyPrint::default();
                use ast::ToNode;
                match ppr.visit_root(&res.to_node()) {
                    Ok(res) => {
                        println!(">> {}", res);
                    },
                    Err(err) => {
                        println!("{:#?}", err);
                    }
                }
            },
            Err(err) => {
                println!("{:#?}", err);
            }
        }
        return Ok(());
    }
    let mut comp = Compiler::default();
    match comp.visit_root(&scoped) {
        Ok(res) => {
            println!("{}", res);
        },
        Err(err) => {
            println!("{:#?}", err);
        }
    }
    return Ok(());
}

#[cfg(test)]
    mod tests {
    use std::fs::File;
    use std::io::prelude::*;

    use super::ast::Visitor;
    use super::interpreter::Interpreter;
    use super::parser;

    include!(concat!(env!("OUT_DIR"), "/test.rs"));
}


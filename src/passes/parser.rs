use std::collections::VecDeque;
use std::sync::Arc;

use crate::ast::*;
use crate::database::Compiler;
use crate::errors::TError;
use crate::externs::{Direction, Semantic};
use crate::location::*;
use crate::primitives::{int32, string, Prim, Val};
use crate::tokens::*;

fn binding(db: &dyn Compiler, tok: &Token) -> Result<Semantic, TError> {
    db.get_extern_operator(tok.value.to_owned())
}

fn binding_dir(db: &dyn Compiler, tok: &Token) -> Result<Direction, TError> {
    Ok(match binding(db, tok)? {
        Semantic::Operator { assoc, .. } => assoc,
        Semantic::Func => Direction::Left,
    })
}

fn binding_power(db: &dyn Compiler, tok: &Token) -> Result<i32, TError> {
    Ok(match binding(db, tok)? {
        Semantic::Operator { binding, .. } => binding,
        Semantic::Func => 1000,
    })
}

impl Token {
    pub fn get_info(&self) -> Info {
        self.pos.clone().get_info()
    }
}

impl Loc {
    pub fn get_info(self) -> Info {
        Info {
            loc: Some(self),
            ..Info::default()
        }
    }
}

fn nud(db: &dyn Compiler, mut toks: VecDeque<Token>) -> Result<(Node, VecDeque<Token>), TError> {
    if let Some(head) = toks.pop_front() {
        match head.tok_type {
            TokenType::NumLit => Ok((
                Node::ValNode(
                    int32(head.value.parse().expect("Unexpected numeric character")),
                    head.get_info(),
                ),
                toks,
            )),
            TokenType::StringLit => Ok((Node::ValNode(string(&head.value), head.get_info()), toks)),
            TokenType::Op => {
                let lbp = binding_power(db, &head)?;
                let (right, new_toks) = expr(db, toks, lbp)?;
                Ok((
                    UnOp {
                        name: head.value.clone(),
                        inner: Box::new(right),
                        info: head.get_info(),
                    }
                    .into_node(),
                    new_toks,
                ))
            }
            TokenType::CloseBracket => Err(TError::ParseError(
                format!("Unexpected close bracket {}", head.value),
                head.get_info(),
            )),
            TokenType::OpenBracket => {
                let (inner, mut new_toks) = expr(db, toks, 0)?;
                // TODO require close bracket.
                let close = new_toks.front();
                match (head.value.as_str(), close) {
                    (
                        open,
                        Some(Token {
                            value: close,
                            tok_type: TokenType::CloseBracket,
                            pos: _,
                        }),
                    ) => {
                        match (open, close.as_str()) {
                            ("(", ")") => {}
                            ("[", "]") => {}
                            ("{", "}") => {}
                            (open, chr) => {
                                return Err(TError::ParseError(
                                    format!(
                                        "Unexpected closing bracket for {}, found {}",
                                        open, chr
                                    ),
                                    head.get_info(),
                                ));
                            }
                        };
                    }
                    (open, chr) => {
                        return Err(TError::ParseError(
                            format!("Unclosed bracket {} found {:?}", open, chr),
                            head.get_info(),
                        ));
                    }
                }
                new_toks.pop_front();
                Ok((inner, new_toks))
            }
            TokenType::Sym => {
                // TODO: Consider making these globals.
                if head.value == "true" {
                    return Ok((Val::PrimVal(Prim::Bool(true)).into_node(), toks));
                }
                if head.value == "false" {
                    return Ok((Val::PrimVal(Prim::Bool(false)).into_node(), toks));
                }
                Ok((
                    Sym {
                        name: head.value.clone(),
                        info: head.get_info(),
                    }
                    .into_node(),
                    toks,
                ))
            }
            TokenType::Unknown | TokenType::Whitespace => Err(TError::ParseError(
                "Lexer should not produce unknown or whitespace".to_string(),
                head.get_info(),
            )),
        }
    } else {
        Ok((
            TError::ParseError("Unexpected eof, expected expr".to_string(), Info::default())
                .into_node(),
            toks,
        ))
    }
}

fn get_defs(args: Node) -> Vec<Let> {
    if let Node::SymNode(symn) = args {
        return vec![symn.as_let()];
    }
    if let Node::LetNode(letn) = args {
        return vec![letn];
    }
    if let Node::BinOpNode(BinOp {
        name,
        left,
        right,
        info: _,
    }) = args.clone()
    {
        if name == "," {
            let mut left = get_defs(*left);
            left.append(&mut get_defs(*right));
            return left;
        }
    }
    vec![Let {
        name: "it".to_string(),
        args: None,
        info: args.get_info(),
        value: Box::new(args),
    }]
}

fn led(
    db: &dyn Compiler,
    mut toks: VecDeque<Token>,
    mut left: Node,
) -> Result<(Node, VecDeque<Token>), TError> {
    if let Some(Token {
        tok_type: TokenType::CloseBracket,
        pos,
        ..
    }) = toks.front()
    {
        return Ok((
            TError::ParseError("Exected Close bracket".to_string(), pos.clone().get_info())
                .into_node(),
            toks,
        ));
    }

    match toks.pop_front() {
        None => Ok((
            TError::ParseError(
                "Unexpected eof, expected expr tail".to_string(),
                left.get_info(),
            )
            .into_node(),
            toks,
        )),
        Some(head) => match head.tok_type {
            TokenType::NumLit | TokenType::StringLit | TokenType::Sym => {
                let pos = head.pos.clone();
                toks.push_front(head);
                toks.push_front(Token {
                    tok_type: TokenType::Op,
                    value: ",".to_string(),
                    pos,
                });
                Ok((left, toks))
            }
            TokenType::Op => {
                let lbp = binding_power(db, &head)?;
                let assoc = binding_dir(db, &head)?;
                let (right, new_toks) = expr(
                    db,
                    toks,
                    lbp - match assoc {
                        Direction::Left => 0,
                        Direction::Right => 1,
                    },
                )?;
                match head.value.as_str() {
                    ":" => {
                        left.get_mut_info().ty = Some(Box::new(right));
                        return Ok((left, new_toks));
                    }
                    "|-" => match left {
                        Node::SymNode(s) => {
                            return Ok((
                                Abs {
                                    name: s.name,
                                    value: Box::new(right),
                                    info: head.get_info(),
                                }
                                .into_node(),
                                new_toks,
                            ))
                        }
                        _ => {
                            return Err(TError::ParseError(
                                format!("Cannot abstract over {}", left),
                                head.get_info(),
                            ))
                        }
                    },
                    "=" => match left {
                        Node::SymNode(s) => {
                            return Ok((
                                Let {
                                    name: s.name,
                                    args: None,
                                    value: Box::new(right),
                                    info: head.get_info(),
                                }
                                .into_node(),
                                new_toks,
                            ))
                        }
                        Node::ApplyNode(a) => match *a.inner {
                            Node::SymNode(s) => {
                                return Ok((
                                    Let {
                                        name: s.name,
                                        args: Some(a.args),
                                        value: Box::new(right),
                                        info: head.get_info(),
                                    }
                                    .into_node(),
                                    new_toks,
                                ))
                            }
                            _ => {
                                return Err(TError::ParseError(
                                    format!("Cannot assign to {}", a.into_node()),
                                    head.get_info(),
                                ))
                            }
                        },
                        _ => {
                            return Err(TError::ParseError(
                                format!("Cannot assign to {}", left),
                                head.get_info(),
                            ))
                        }
                    },
                    _ => {}
                }
                Ok((
                    BinOp {
                        info: head.get_info(),
                        name: head.value,
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                    .into_node(),
                    new_toks,
                ))
            }
            TokenType::CloseBracket => Err(TError::ParseError(
                "Unexpected close bracket".to_string(),
                head.get_info(),
            )),
            TokenType::OpenBracket => {
                if head.value.as_str() == "("
                    && toks.front().map(|t| &t.value) == Some(&")".to_string())
                {
                    toks.pop_front();
                    return Ok((
                        Apply {
                            inner: Box::new(left),
                            args: vec![],
                            info: head.get_info(),
                        }
                        .into_node(),
                        toks,
                    ));
                }
                let (args, mut new_toks) = expr(db, toks, 0)?;
                let close = new_toks.front();
                match (head.value.as_str(), close) {
                    (
                        open,
                        Some(Token {
                            value: close,
                            tok_type: TokenType::CloseBracket,
                            ..
                        }),
                    ) => {
                        match (open, close.as_str()) {
                            ("(", ")") => {}
                            ("[", "]") => {}
                            ("{", "}") => {}
                            (open, chr) => {
                                return Err(TError::ParseError(
                                    format!(
                                        "Unexpected closing bracket for {}, found {}.",
                                        open, chr
                                    ),
                                    head.get_info(),
                                ));
                            }
                        };
                    }
                    (open, chr) => {
                        return Err(TError::ParseError(
                            format!("Unclosed bracket {}, found {:?}", open, chr),
                            head.get_info(),
                        ));
                    }
                }
                new_toks.pop_front();
                // Introduce arguments
                Ok((
                    Apply {
                        inner: Box::new(left),
                        args: get_defs(args),
                        info: head.get_info(),
                    }
                    .into_node(),
                    new_toks,
                ))
            }
            TokenType::Unknown | TokenType::Whitespace => Err(TError::ParseError(
                "Lexer should not produce unknown or whitespace".to_string(),
                head.get_info(),
            )),
        },
    }
}

fn expr(
    db: &dyn Compiler,
    init_toks: VecDeque<Token>,
    init_lbp: i32,
) -> Result<(Node, VecDeque<Token>), TError> {
    // TODO: Name update's fields, this is confusing (0 is tree, 1 is toks)
    let init_update = nud(db, init_toks)?;
    let mut left: Node = init_update.0;
    let mut toks: VecDeque<Token> = init_update.1;
    loop {
        match toks.front() {
            None => break,
            Some(token) => {
                if init_lbp >= binding_power(db, token)? {
                    break;
                }
            }
        }
        let update = led(db, toks, left.clone())?;
        if let (Node::Error(_), new_toks) = update {
            return Ok((left, new_toks));
        }
        left = update.0;
        toks = update.1;
    }
    Ok((left, toks))
}

pub fn lex(db: &dyn Compiler, module: PathRef) -> Result<VecDeque<Token>, TError> {
    let filename = db.filename(module.to_vec());
    lex_string(db, module, &db.file(filename)?.to_string())
}

pub fn lex_string(
    db: &dyn Compiler,
    module: PathRef,
    contents: &str,
) -> Result<VecDeque<Token>, TError> {
    let filename = db.filename(module.to_vec());
    let mut toks: VecDeque<Token> = VecDeque::new();

    let mut pos = Loc {
        filename: Some(filename),
        ..Loc::default()
    };
    let mut chars = contents.chars().peekable();
    loop {
        let (next, new_chars) = lex_head(chars, &mut pos);
        if next.tok_type == TokenType::Unknown {
            break; // TODO done / skip?
        }
        // If valid, take the token and move on.
        toks.push_back(next);
        chars = new_chars;
    }
    Ok(toks)
}

pub fn parse_string(
    db: &dyn Compiler,
    module: PathRef,
    text: &Arc<String>,
) -> Result<Node, TError> {
    let toks = db.lex_string(module.to_vec(), text.clone())?;
    if db.debug_level() > 0 {
        eprintln!("parsing str... {}", path_to_string(module));
    }
    let (root, left_over) = expr(db, toks, 0)?;

    if let Some(head) = left_over.front() {
        return Err(TError::ParseError(
            format!("Oh no: Left over tokens {:?}", left_over),
            head.get_info(),
        ));
    }
    if db.options().show_ast {
        eprintln!("ast: {}", root);
    }
    Ok(root)
}

pub fn parse(db: &dyn Compiler, module: PathRef) -> Result<Node, TError> {
    let toks = db.lex_file(module.to_vec())?;
    if db.debug_level() > 0 {
        eprintln!("parsing file... {}", path_to_string(module));
    }
    let (root, left_over) = expr(db, toks, 0)?;

    if let Some(head) = left_over.front() {
        return Err(TError::ParseError(
            format!("Oh no: Left over tokens {:?}", left_over),
            head.get_info(),
        ));
    }
    if db.options().show_ast {
        eprintln!("ast: {}", root);
    }
    Ok(root)
}

#[cfg(test)]
pub mod tests {
    use super::parse_string;
    use crate::ast::*;
    use crate::database::{Compiler, DB};
    use crate::primitives::{int32, string};

    fn parse(contents: String) -> Node {
        use crate::cli_options::Options;
        use std::sync::Arc;
        let mut db = DB::default();
        let filename = "test.tk";
        db.set_options(Options::default());
        let module = db.module_name(filename.to_owned());
        parse_string(&db, &module, &Arc::new(contents)).expect("failed to parse string")
    }

    fn num_lit(x: i32) -> Box<Node> {
        Box::new(int32(x).into_node())
    }

    fn str_lit(x: &str) -> Box<Node> {
        Box::new(string(x).into_node())
    }

    #[test]
    fn parse_num() {
        assert_eq!(parse("12".to_string()), int32(12).into_node());
    }

    #[test]
    fn parse_str() {
        assert_eq!(
            parse("\"hello world\"".to_string()),
            string("hello world").into_node()
        );
    }

    #[test]
    fn parse_un_op() {
        assert_eq!(
            parse("-12".to_string()),
            UnOp {
                name: "-".to_string(),
                inner: Box::new(int32(12).into_node()),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_min_op() {
        assert_eq!(
            parse("14-12".to_string()),
            BinOp {
                name: "-".to_string(),
                left: num_lit(14),
                right: num_lit(12),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_mul_op() {
        assert_eq!(
            parse("14*12".to_string()),
            BinOp {
                name: "*".to_string(),
                left: num_lit(14),
                right: num_lit(12),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_add_mul_precedence() {
        assert_eq!(
            parse("3+2*4".to_string()),
            BinOp {
                name: "+".to_string(),
                left: num_lit(3),
                right: Box::new(
                    BinOp {
                        name: "*".to_string(),
                        left: num_lit(2),
                        right: num_lit(4),
                        info: Info::default()
                    }
                    .into_node()
                ),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_mul_add_precedence() {
        assert_eq!(
            parse("3*2+4".to_string()),
            BinOp {
                name: "+".to_string(),
                left: Box::new(
                    BinOp {
                        name: "*".to_string(),
                        left: num_lit(3),
                        right: num_lit(2),
                        info: Info::default()
                    }
                    .into_node()
                ),
                right: num_lit(4),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_mul_add_parens() {
        assert_eq!(
            parse("3*(2+4)".to_string()),
            BinOp {
                name: "*".to_string(),
                left: num_lit(3),
                right: Box::new(
                    BinOp {
                        name: "+".to_string(),
                        left: num_lit(2),
                        right: num_lit(4),
                        info: Info::default()
                    }
                    .into_node()
                ),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_add_str() {
        assert_eq!(
            parse("\"hello\"+\" world\"".to_string()),
            BinOp {
                name: "+".to_string(),
                left: str_lit("hello"),
                right: str_lit(" world"),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_strings_followed_by_raw_values() {
        assert_eq!(
            parse("\"hello world\"\n7".to_string()),
            BinOp {
                name: ",".to_string(),
                left: Box::new(str_lit("hello world").into_node()),
                right: num_lit(7),
                info: Info::default()
            }
            .into_node()
        );
    }

    #[test]
    fn parse_strings_with_operators_and_trailing_values_in_let() {
        assert_eq!(
            parse("x()= !\"hello world\";\n7".to_string()),
            BinOp {
                name: ";".to_string(),
                left: Box::new(
                    Let {
                        name: "x".to_string(),
                        args: Some(vec![]),
                        value: Box::new(
                            UnOp {
                                name: "!".to_string(),
                                inner: str_lit("hello world"),
                                info: Info::default(),
                            }
                            .into_node()
                        ),
                        info: Info::default(),
                    }
                    .into_node()
                ),
                right: num_lit(7),
                info: Info::default()
            }
            .into_node()
        );
    }
}
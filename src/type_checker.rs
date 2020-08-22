use crate::ast::Node;
use crate::ast::{Node::*, Prim::*};
use crate::database::Compiler;
use crate::errors::TError;

pub fn infer(db: &dyn Compiler, expr: &Node) -> Result<Node, TError> {
    // Infer that expression t has type A, t => A
    // See https://ncatlab.org/nlab/show/bidirectional+typechecking
    use crate::ast::*;
    match expr {
        PrimNode(prim) => match prim {
            I32(_, _) => Ok(Sym {
                name: "I32".to_owned(),
                info: Info::default(),
            }
            .to_node()),
            Bool(_, _) => Ok(Sym {
                name: "Bool".to_owned(),
                info: Info::default(),
            }
            .to_node()),
            Str(_, _) => Ok(Sym {
                name: "String".to_owned(),
                info: Info::default(),
            }
            .to_node()),
            Lambda(node) => infer(db, node.as_ref()), // TODO: abstraction
        },
        UnOpNode(UnOp {
            name: _,
            inner: _,
            info: _,
        }) => panic!("TODO Impl type checking for UnOp"),
        BinOpNode(BinOp {
            name: _,
            left: _,
            right: _,
            info: _,
        }) => panic!("TODO Impl type checking for BinOp"),
        SymNode(Sym { name: _, info: _ }) => panic!("TODO Impl type checking for Sym"),
        ApplyNode(Apply {
            inner: _,
            args: _,
            info: _,
        }) => panic!("TODO Impl type checking for Apply"),
        LetNode(Let {
            name: _,
            value: _,
            args: _,
            info: _,
        }) => panic!("TODO Impl type checking for Let"),
        Error(Err { msg: _, info: _ }) => panic!("TODO Impl type checking for Let"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Info, Sym, ToNode};
    use crate::database::DB;
    use crate::parser::parse_string_for_test;

    fn assert_type(prog: &str, ty: &str) {
        let mut db = DB::default();
        let prog = parse_string_for_test(&mut db, prog.to_string());
        let prog = infer(&mut db, &prog);
        let ty = parse_string_for_test(&mut db, ty.to_string());
        assert_eq!(prog, Ok(ty));
    }

    #[test]
    fn infer_type_of_i32() {
        let mut db = DB::default();
        let num = I32(23, Info::default()).to_node();
        assert_eq!(
            infer(&mut db, &num),
            Ok(Sym {
                name: "I32".to_owned(),
                info: Info::default()
            }
            .to_node())
        );
        assert_type("23", "I32");
    }

    //#[test]
    fn infer_type_of_sym_i32() {
        assert_type("x=12;x", "I32");
    }

    //#[test]
    fn infer_type_of_sym_with_extra_lets_i32() {
        assert_type("x=12;y=4;x", "I32");
    }

    // #[test]
    fn infer_type_of_id() {
        assert_type("{x}", ",(x: X): X");
    }

    // #[test]
    fn infer_type_of_id_apply() {
        assert_type("{x}(x=12)", "I32");
    }

    // #[test]
    fn infer_type_of_id_apply_it_arg() {
        assert_type("{it}(12)", "I32");
    }

    // #[test]
    fn infer_type_of_plus_expr() {
        assert_type("12+32", "I32");
    }
}

use std::collections::HashMap;

use super::ast::*;
use super::database::Compiler;
use super::errors::TError;
use super::extern_impls::{Res, State, Frame};

//Vec<&dyn Fn() -> Res>
pub type ImplFn<'a> = &'a mut dyn FnMut(&dyn Compiler, &mut State, Info) -> Res;

// Walks the AST interpreting it.
pub struct Interpreter<'a> {
    pub impls: HashMap<String, ImplFn<'a>>,
}

impl<'a> Default for Interpreter<'a> {
    fn default() -> Interpreter<'a> {
        Interpreter {
            impls: HashMap::new(),
        }
    }
}

fn find_symbol<'a>(state: &'a [Frame], name: &str) -> Option<&'a Prim> {
    for frame in state.iter().rev() {
        if let Some(val) = frame.get(name) {
            return Some(val); // This is the variable
        }
        // Not in this frame, go back up.
    }
    None
}
// TODO: Return nodes.
impl<'a> Visitor<State, Prim, Prim> for Interpreter<'a> {
    fn visit_root(&mut self, db: &dyn Compiler, root: &Root) -> Res {
        let mut state = vec![HashMap::new()];
        self.visit(db, &mut state, &root.ast)
    }

    fn visit_sym(&mut self, db: &dyn Compiler, state: &mut State, expr: &Sym) -> Res {
        if db.debug() > 0 {
            eprintln!("evaluating let {}", expr.clone().to_node());
        }
        let name = &expr.name;
        let value = find_symbol(&state, name);
        if let Some(value) = value {
            if db.debug() > 0 {
                eprintln!("from stack {}", value.clone().to_node());
            }
            return Ok(value.clone());
        }
        if db.debug() > 2 {
            eprintln!("checking for interpreter impl {}", expr.name.clone());
        }
        if let Some(extern_impl) = &mut self.impls.get_mut(name) {
            return extern_impl(db, state, expr.get_info());
        }
        if db.debug() > 2 {
            eprintln!("checking for default impl {}", expr.name.clone());
        }
        if let Some(default_impl) = crate::externs::get_implementation(name.to_owned()) {
            return default_impl(db, state, expr.get_info());
        }
        Err(TError::UnknownSymbol(name.to_string(), expr.info.clone()))
    }

    fn visit_prim(&mut self, _db: &dyn Compiler, _state: &mut State, expr: &Prim) -> Res {
        Ok(expr.clone())
    }

    fn visit_apply(&mut self, db: &dyn Compiler, state: &mut State, expr: &Apply) -> Res {
        if db.debug() > 0 {
            eprintln!("evaluating apply {}", expr.clone().to_node());
        }
        state.push(Frame::new());
        for arg in expr.args.iter() {
            self.visit_let(db, state, arg)?;
        }
        // Visit the expr.inner
        let res = self.visit(db, state, &*expr.inner)?;
        state.pop();
        Ok(res)
    }

    fn visit_let(&mut self, db: &dyn Compiler, state: &mut State, expr: &Let) -> Res {
        if db.debug() > 0 {
            eprintln!("evaluating let {}", expr.clone().to_node());
        }

        if expr.args.is_some() {
            // TODO double check
            state
                .last_mut()
                .unwrap()
                .insert(expr.name.clone(), Prim::Lambda(Box::new(expr.value.clone().to_node())));
            return Ok(Prim::Lambda(Box::new(expr.to_sym().to_node())));
        }
        // Add a new scope
        state.push(Frame::new());
        let result = self.visit(db, state, &expr.value)?;
        // Drop the finished scope
        state.pop();
        match state.last_mut() {
            None => panic!("there is no stack frame"),
            Some(frame) => {
                frame.insert(expr.name.clone(), result.clone());
                Ok(result)
            }
        }
    }

    fn handle_error(&mut self, _db: &dyn Compiler, _state: &mut State, expr: &Err) -> Res {
        Err(TError::FailedParse(expr.msg.to_string(), expr.get_info()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::*;
    use super::super::cli_options::Options;
    use super::super::database::{Compiler, DB};
    use super::{Interpreter, Res};
    use std::collections::HashMap;
    use Node::*;
    use Prim::*;

    #[test]
    fn eval_num() {
        let mut db = DB::default();
        db.set_options(Options::default());
        let tree = PrimNode(I32(12, Info::default()));
        assert_eq!(
            Interpreter::default().visit(&db, &mut vec![], &tree),
            Ok(I32(12, Info::default()))
        );
    }

    fn eval_str(s: String) -> Res {
        use std::sync::Arc;
        let mut db = DB::default();
        let filename = "test.tk";
        let module = db.module_name(filename.to_owned());
        db.set_file(filename.to_owned(), Ok(Arc::new(s)));
        db.set_options(Options::default());
        let ast = db.parse_file(module)?;
        let mut state = vec![HashMap::new()];
        Interpreter::default().visit(&db, &mut state, &ast)
    }

    fn trace<T: std::fmt::Display, E>(t: Result<T, E>) -> Result<T, E> {
        match &t {
            Ok(t) => eprintln!(">> {}", &t),
            Err(_) => eprintln!(">> #error"),
        }
        t
    }

    #[test]
    fn parse_and_eval_bool() {
        assert_eq!(
            eval_str("true".to_string()),
            Ok(Bool(true, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_bool_and() {
        assert_eq!(
            eval_str("true&&true".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("false&&true".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("true&&false".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("false&&false".to_string()),
            Ok(Bool(false, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_bool_or() {
        assert_eq!(
            eval_str("true||true".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("false||true".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("true||false".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("false||false".to_string()),
            Ok(Bool(false, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_bool_eq() {
        assert_eq!(
            eval_str("true==true".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("false==true".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("true==false".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("false==false".to_string()),
            Ok(Bool(true, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_i32() {
        assert_eq!(eval_str("32".to_string()), Ok(I32(32, Info::default())));
    }

    #[test]
    fn parse_and_eval_i32_eq() {
        assert_eq!(
            eval_str("0==0".to_string()),
            Ok(Bool(true, Info::default()))
        );
        assert_eq!(
            eval_str("-1==1".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("1==123".to_string()),
            Ok(Bool(false, Info::default()))
        );
        assert_eq!(
            eval_str("1302==1302".to_string()),
            Ok(Bool(true, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_i32_pow() {
        assert_eq!(eval_str("2^3".to_string()), Ok(I32(8, Info::default())));
        assert_eq!(eval_str("3^2".to_string()), Ok(I32(9, Info::default())));
        assert_eq!(eval_str("-4^2".to_string()), Ok(I32(-16, Info::default())));
        assert_eq!(eval_str("(-4)^2".to_string()), Ok(I32(16, Info::default())));
        assert_eq!(eval_str("2^3^2".to_string()), Ok(I32(512, Info::default())));
    }

    #[test]
    fn parse_and_eval_str() {
        assert_eq!(
            eval_str("\"32\"".to_string()),
            Ok(Str("32".to_string(), Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_let() {
        assert_eq!(eval_str("x=3;x".to_string()), Ok(I32(3, Info::default())));
    }

    #[test]
    fn parse_and_eval_let_with_args() {
        assert_eq!(
            eval_str("x(it)=it*2;x(3)".to_string()),
            Ok(I32(6, Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_i32_type() {
        assert_eq!(
            eval_str("I32".to_string()),
            Ok(TypeValue(crate::types::i32_type(), Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_number_type() {
        assert_eq!(
            eval_str("Number".to_string()),
            Ok(TypeValue(crate::types::number_type(), Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_string_type() {
        assert_eq!(
            eval_str("String".to_string()),
            Ok(TypeValue(crate::types::string_type(), Info::default()))
        );
    }

    #[test]
    fn parse_and_eval_string_or_number_type() {
        use crate::types::{Type::*, *};
        assert_eq!(
            eval_str("String | Number".to_string()),
            Ok(TypeValue(
                Union(set![number_type(), string_type()]),
                Info::default()
            ))
        );
    }

    #[test]
    fn parse_and_eval_string_and_number_type() {
        use crate::types::{Type::*, *};
        assert_eq!(
            eval_str("String & Number".to_string()),
            Ok(TypeValue(
                Product(set![number_type(), string_type()]),
                Info::default()
            ))
        );
    }

    #[test]
    fn parse_and_eval_tagged_string_or_number_type() {
        use crate::types::*;
        assert_eq!(
            eval_str("String + I32".to_string()),
            Ok(TypeValue(
                sum(vec![string_type(), i32_type()]).unwrap(),
                Info::default()
            ))
        );
    }

    #[test]
    fn parse_and_eval_string_times_number_type() {
        use crate::types::*;
        assert_eq!(
            eval_str("String * I32".to_string()),
            Ok(TypeValue(
                record(vec![string_type(), i32_type()]).unwrap(),
                Info::default()
            ))
        );
    }

    #[test]
    fn tako_add_eq_rust_eq() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let num1: i32 = rng.gen();
            let num2: i32 = rng.gen();
            let res = num1.wrapping_add(num2);
            eprintln!("mul {:?} + {:?} = {:?}", num1, num2, res);
            assert_eq!(
                eval_str(format!("mul(x, y)=x+y;mul(x= {}, y= {})", num1, num2)),
                Ok(I32(res, Info::default()))
            );
        }
    }

    #[test]
    fn tako_mul_eq_rust_eq() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let num1: i32 = rng.gen();
            let num2: i32 = rng.gen();
            let res = num1.wrapping_mul(num2);
            eprintln!("mul {:?} * {:?} = {:?}", num1, num2, res);
            assert_eq!(
                eval_str(format!("mul(x, y)=x*y;mul(x= {}, y= {})", num1, num2)),
                Ok(I32(res, Info::default()))
            );
        }
    }
}

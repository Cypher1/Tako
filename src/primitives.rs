use crate::ast::Info;
use crate::ast::Node;
use crate::errors::TError;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::fmt;

// i32 here are sizes in bits, not bytes.
// This means that we don't need to have a separate systems for bit&byte layouts.
pub type Offset = i32;

// A list of types with an offset to get to the first bit (used for padding, frequently 0).
type Layout = Vec<Prim>;
type TypeSet = BTreeSet<Prim>;
type Pack = BTreeSet<(String, Prim)>;

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Debug, Hash)]
pub enum Prim {
    Void(),
    Unit(),
    Bool(bool),
    I32(i32),
    Str(String),
    Lambda(Box<Node>),
    Struct(Vec<(String, Prim)>), // Should really just store values, but we can't do that yet.
    Union(TypeSet),
    Product(TypeSet),
    StaticPointer(Offset),
    Padded(Offset, Box<Prim>),
    Tag(Offset, Offset), // A locally unique id and the number of bits needed for it, should be replaced with a bit pattern at compile time.
    Pointer(Offset, Box<Prim>), // Defaults to 8 bytes (64 bit)
    Function {
        intros: Pack,
        arguments: Box<Prim>,
        results: Box<Prim>,
    },
    App {
        inner: Box<Prim>,
        arguments: Box<Prim>,
    },
    // The following should be eliminated during lowering
    WithEffect(Box<Prim>, Vec<String>),
    Variable(String),
}


fn merge_vals(left: Vec<(String, Prim)>, right: Vec<(String, Prim)>) -> Vec<(String, Prim)> {
    let mut names = HashSet::<String>::new();
    for pair in right.iter() {
        names.insert(pair.0.clone());
    }
    let mut items = vec![];
    for pair in left.iter() {
        if !names.contains(&pair.0) {
            items.push(pair.clone());
        }
    }
    for pair in right.iter() {
        items.push(pair.clone());
    }
    items
}

impl Prim {
    pub fn merge(self: Prim, other: Prim) -> Prim {
        use Prim::*;
        match (self, other) {
            (Struct(vals), Struct(o_vals)) => Struct(merge_vals(vals, o_vals)),
            (Struct(vals), other) => {
                Struct(merge_vals(vals, vec![("it".to_string(), other)]))
            }
            (_, other) => other,
        }
    }
}

impl fmt::Display for Prim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let types = vec![
            (string_type(), "String"),
            (number_type(), "Number"),
            (i32_type(), "I32"),
            (byte_type(), "Byte"),
            (bit_type(), "Bit"),
        ];
        for (ty, name) in types.iter() {
            if self == ty {
                return write!(f, "{}", name);
            }
        }
        match self {
            Void() => write!(f, "Void"),
            Unit() => write!(f, "()"),
            Bool(val) => write!(f, "{}", val),
            I32(val) => write!(f, "{}", val),
            Str(val) => write!(f, "'{}'", val),
            Lambda(val) => write!(f, "{}", val),
            Struct(vals) => {
                write!(f, "{{").unwrap();
                let mut is_first = true;
                for val in vals.iter() {
                    if !is_first {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{} = {}", val.0, &val.1).unwrap();
                    is_first = false;
                }
                write!(f, "}}")
            }
            Union(s) => {
                write!(f, "Union(")?;
                let mut first = true;
                for sty in s {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", sty)?;
                }
                write!(f, ")")
            }
            Product(s) => {
                write!(f, "Product(")?;
                let mut first = true;
                for sty in s {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", sty)?;
                }
                write!(f, ")")
            }
            Pointer(ptr_size, t) => write!(f, "*<{}b>{}", ptr_size, t),
            Tag(tag, bits) => write!(f, "Tag<{}b>{}", bits, tag),
            Padded(size, t) => write!(f, "Pad<{}b>{}", size, t),
            StaticPointer(ptr_size) => write!(f, "*<{}b>Code", ptr_size),
            x => write!(f, "({:?})", x),
        }
    }
}

use Prim::*;

impl Prim {
    pub fn ptr(self: Prim) -> Prim {
        Pointer(8 * byte_size(), Box::new(self))
    }
    pub fn padded(self: Prim, size: Offset) -> Prim {
        if size == 0 {
            return self;
        }
        if let Padded(n, t) = self {
            return Padded(n + size, t);
        }
        Padded(size, Box::new(self))
    }
}

#[allow(dead_code)]
pub fn card(ty: &Prim) -> Result<Offset, TError> {
    use Prim::*;
    match ty {
        Union(s) => {
            let mut sum = 0;
            for sty in s {
                sum += card(&sty)?;
            }
            Ok(sum)
        }
        Product(s) => {
            let mut prod = 1;
            for sty in s {
                prod *= card(&sty)?;
            }
            Ok(prod)
        }
        Pointer(_ptr_size, t) => card(&t),
        Tag(_tag, _bits) => Ok(1),
        Padded(_size, t) => card(&t),
        StaticPointer(_ptr_size) => Err(TError::StaticPointerCardinality(Info::default())),
        x => panic!(format!("unhandled: card of {:#?}", x)),
    }
}

// Calculates the memory needed for a new instance in bits.
pub fn size(ty: &Prim) -> Result<Offset, TError> {
    use Prim::*;
    match ty {
        Union(s) => {
            let mut res = 0;
            for sty in s.iter() {
                // This includes padding in size.
                let c = size(&sty)?;
                if res <= c {
                    res = c;
                }
            }
            Ok(res)
        }
        Product(s) => {
            let mut res = 0;
            for sty in s.iter() {
                let c = size(&sty)?;
                if res <= c {
                    res = c;
                }
            }
            Ok(res)
        }
        Pointer(ptr_size, _t) => Ok(*ptr_size),
        Tag(_tag, bits) => Ok(*bits),
        StaticPointer(ptr_size) => Ok(*ptr_size),
        Padded(bits, t) => Ok(bits + size(t)?),
        Variable(name) => Err(TError::UnknownSizeOfVariableType(
            name.clone(),
            Info::default(),
        )),
        x => panic!(format!("unhandled: size of {:#?}", x)),
    }
}

fn num_bits(n: Offset) -> Offset {
    let mut k = 0;
    let mut p = 1;
    loop {
        if n <= p {
            return k;
        }
        k += 1;
        p *= 2;
    }
}

pub fn record(values: Layout) -> Result<Prim, TError> {
    let mut layout = set![];
    let mut off = 0;
    for val in values {
        // Work out the padding here
        let size = size(&val)?;
        layout.insert(val.padded(off));
        off += size;
    }
    Ok(Product(layout))
}

pub fn sum(values: Vec<Prim>) -> Result<Prim, TError> {
    let mut layout = set![];
    let tag_bits = num_bits(values.len() as Offset);
    for (count, val) in values.into_iter().enumerate() {
        let mut tagged = Tag(count as i32, tag_bits);
        if val != unit_type() {
            tagged = record(vec![tagged, val])?;
        }
        layout.insert(tagged);
    }
    Ok(Union(layout))
}

pub fn void_type() -> Prim {
    Union(set![])
}

pub fn unit_type() -> Prim {
    Product(set![])
}

pub fn bit_type() -> Prim {
    sum(vec![unit_type(), unit_type()]).expect("bit should be safe")
}

pub fn byte_type() -> Prim {
    record(vec![
        bit_type(),
        bit_type(),
        bit_type(),
        bit_type(),
        bit_type(),
        bit_type(),
        bit_type(),
        bit_type(),
    ])
    .expect("byte should be safe")
}

pub fn byte_size() -> Offset {
    8
}

pub fn char_type() -> Prim {
    byte_type()
}

pub fn string_type() -> Prim {
    char_type().ptr()
}

pub fn i32_type() -> Prim {
    record(vec![byte_type(), byte_type(), byte_type(), byte_type()]).expect("i32 should be safe")
}

pub fn number_type() -> Prim {
    variable("Number")
}

pub fn type_type() -> Prim {
    variable("Type")
}

pub fn variable(name: &str) -> Prim {
    Variable(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void() {
        assert_eq!(card(&void_type()), Ok(0));
        assert_eq!(size(&void_type()), Ok(0));
    }
    #[test]
    fn unit() {
        assert_eq!(card(&unit_type()), Ok(1));
        assert_eq!(size(&unit_type()), Ok(0));
    }
    #[test]
    fn tag1_type() {
        assert_eq!(card(&Tag(1, 1)), Ok(1));
        assert_eq!(size(&Tag(1, 1)), Ok(1));
    }
    #[test]
    fn tag2_type() {
        assert_eq!(card(&Tag(0, 2)), Ok(1));
        assert_eq!(size(&Tag(0, 2)), Ok(2));
        assert_eq!(card(&Tag(1, 2)), Ok(1));
        assert_eq!(size(&Tag(1, 2)), Ok(2));
        assert_eq!(card(&Tag(2, 2)), Ok(1));
        assert_eq!(size(&Tag(2, 2)), Ok(2));
        assert_eq!(card(&Tag(3, 2)), Ok(1));
        assert_eq!(size(&Tag(3, 2)), Ok(2));
    }
    #[test]
    fn tag4_type() {
        assert_eq!(card(&Tag(4, 3)), Ok(1));
        assert_eq!(size(&Tag(4, 3)), Ok(3));
    }

    #[test]
    fn union_n_type() {
        let union2 = Union(set![unit_type(), unit_type()]);
        assert_eq!(card(&union2), Ok(1));
        assert_eq!(size(&union2), Ok(0));
        let union3 = Union(set![unit_type(), unit_type(), unit_type()]);
        assert_eq!(card(&union3), Ok(1));
        assert_eq!(size(&union3), Ok(0));
    }
    #[test]
    fn bit() {
        let bitt = bit_type();
        assert_eq!(card(&bitt), Ok(2));
        assert_eq!(size(&bitt), Ok(1));
    }
    #[test]
    fn trit_type() {
        let trit = sum(vec![unit_type(), unit_type(), unit_type()]).unwrap();
        assert_eq!(card(&trit), Ok(3));
        assert_eq!(size(&trit), Ok(2));
    }
    #[test]
    fn nested_quad_type() {
        let quad = record(vec![bit_type(), bit_type()]).unwrap();
        assert_eq!(card(&quad), Ok(4));
        assert_eq!(size(&quad), Ok(2));
    }
    #[test]
    fn quad_type() {
        let quad = sum(vec![unit_type(), unit_type(), unit_type(), unit_type()]).unwrap();
        assert_eq!(card(&quad), Ok(4));
        assert_eq!(size(&quad), Ok(2));
    }
    #[test]
    fn pent_type() {
        let pent = sum(vec![
            unit_type(),
            unit_type(),
            unit_type(),
            unit_type(),
            unit_type(),
        ])
        .unwrap();
        assert_eq!(card(&pent), Ok(5));
        assert_eq!(size(&pent), Ok(3));
    }
    #[test]
    fn pair_bool_ptrs() {
        let bool_ptr = Pointer(64, Box::new(bit_type()));
        let quad = record(vec![bool_ptr.clone(), bool_ptr]).unwrap();
        assert_eq!(card(&quad), Ok(4));
        assert_eq!(size(&quad), Ok(2 * 64));
    }
    #[test]
    fn nested_nibble() {
        let quad = record(vec![bit_type(), bit_type()]).unwrap();
        let nibble = record(vec![quad.clone(), quad]).unwrap();
        assert_eq!(card(&nibble), Ok(16));
        assert_eq!(size(&nibble), Ok(4));
    }
    #[test]
    fn padded_nibble() {
        let quad = record(vec![bit_type().padded(2), bit_type()]).unwrap();
        let nibble = record(vec![quad.clone(), quad]).unwrap();
        assert_eq!(card(&nibble), Ok(16));
        assert_eq!(size(&nibble), Ok(8));
    }

    #[test]
    fn bool_and_fn() {
        let fn_ptr = StaticPointer(64);
        let closure = record(vec![bit_type(), fn_ptr]).unwrap();
        assert_eq!(size(&closure), Ok(65));
    }

    #[test]
    fn bool_or_fn() {
        let fn_ptr = StaticPointer(64);
        let closure = sum(vec![bit_type(), fn_ptr]).unwrap();
        assert_eq!(size(&closure), Ok(65));
    }
}
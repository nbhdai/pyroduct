//! A field's Rust type reduced to the shape its inodes take.
//!
//! This is only used to build the struct's const inode table. Reading and
//! writing never look at it: they call the field type's own
//! `ForeignWriter`/`ForeignReader` impl, so a type the wire has no impl for
//! fails there, at the field, with rustc's own error.

use pyro_schema::WireKind;
use syn::{Error, GenericArgument, PathArguments, Result, Type};

/// A fixed-width wire primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Prim {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F16,
    F32,
    F64,
    Bool,
    Time,
}

impl Prim {
    pub(crate) fn wire_kind(self, option: bool) -> WireKind {
        match (self, option) {
            (Prim::U8, false) => WireKind::U8,
            (Prim::I8, false) => WireKind::I8,
            (Prim::U16, false) => WireKind::U16,
            (Prim::I16, false) => WireKind::I16,
            (Prim::U32, false) => WireKind::U32,
            (Prim::I32, false) => WireKind::I32,
            (Prim::U64, false) => WireKind::U64,
            (Prim::I64, false) => WireKind::I64,
            (Prim::F16, false) => WireKind::F16,
            (Prim::F32, false) => WireKind::F32,
            (Prim::F64, false) => WireKind::F64,
            (Prim::Bool, false) => WireKind::Bool,
            (Prim::Time, false) => WireKind::Timestamp,
            (Prim::U8, true) => WireKind::OptionU8,
            (Prim::I8, true) => WireKind::OptionI8,
            (Prim::U16, true) => WireKind::OptionU16,
            (Prim::I16, true) => WireKind::OptionI16,
            (Prim::U32, true) => WireKind::OptionU32,
            (Prim::I32, true) => WireKind::OptionI32,
            (Prim::U64, true) => WireKind::OptionU64,
            (Prim::I64, true) => WireKind::OptionI64,
            (Prim::F16, true) => WireKind::OptionF16,
            (Prim::F32, true) => WireKind::OptionF32,
            (Prim::F64, true) => WireKind::OptionF64,
            (Prim::Bool, true) => WireKind::OptionBool,
            (Prim::Time, true) => WireKind::OptionTimestamp,
        }
    }
}

/// What sits under the `Vec`s of a non-struct, non-map field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leaf {
    Prim(Prim),
    OptionPrim(Prim),
    Array(usize, Prim),
    OptionArray(usize, Prim),
    /// `String`.
    Str,
    /// `Vec<u8>`, `Box<[u8]>`, `Bytes`, `BytesMut`. A `Vec<u8>` is a `u8`
    /// list instead only on a `#[pyro(numeric)]` field.
    Bytes,
    /// `pyroformat::BitVec`: a bool list whose own list is not a `Vec`.
    BitVec,
}

#[derive(Debug, Clone)]
pub(crate) enum ResolvedType {
    /// A leaf under `vec_depth` `Vec`s.
    Leaf { leaf: Leaf, vec_depth: usize },
    /// `BTreeMap<K, V>` / `pyroformat::Map<K, V>` under `vec_depth` `Vec`s.
    Map {
        key: Box<ResolvedType>,
        value: Box<ResolvedType>,
        vec_depth: usize,
    },
    /// Anything else is taken to be another derived struct, `ty`, under
    /// `vec_depth` `Vec`s, with an `Option` directly around it when
    /// `optional`.
    Struct {
        ty: Type,
        vec_depth: usize,
        optional: bool,
    },
}

impl ResolvedType {
    pub(crate) fn vec_depth(&self) -> usize {
        match self {
            ResolvedType::Leaf { vec_depth, .. }
            | ResolvedType::Map { vec_depth, .. }
            | ResolvedType::Struct { vec_depth, .. } => *vec_depth,
        }
    }
}

/// The most list levels a leaf may sit under. A `String`, bytes or a
/// `BitVec` is a list of its own and takes one of them, so
/// `Vec<Vec<Vec<Vec<String>>>>` and `Vec<Vec<Vec<Vec<Vec<u8>>>>>` (bytes
/// under four `Vec`s) are as deep as a field goes.
pub(crate) const MAX_LIST_LEVELS: usize = 5;

impl Leaf {
    /// Whether the leaf is itself a list on the wire.
    fn is_list(self) -> bool {
        matches!(self, Leaf::Str | Leaf::Bytes | Leaf::BitVec)
    }
}

/// `numeric` is the field's `#[pyro(numeric)]`: without it a `Vec<u8>` is
/// bytes, with it a list of `u8`.
pub(crate) fn resolve_type(ty: &Type, numeric: bool) -> Result<ResolvedType> {
    let resolved = resolve(ty, 0, numeric)?;
    check_list_levels(ty, &resolved)?;
    Ok(resolved)
}

/// Rejects a leaf under more than [`MAX_LIST_LEVELS`] list levels, here or in
/// a map's keys or values.
fn check_list_levels(ty: &Type, resolved: &ResolvedType) -> Result<()> {
    match resolved {
        ResolvedType::Leaf { leaf, vec_depth } => {
            let levels = vec_depth + leaf.is_list() as usize;
            if levels > MAX_LIST_LEVELS {
                let leaf_note = if leaf.is_list() {
                    " (a `String`, bytes or `BitVec` is a list level of its own)"
                } else {
                    ""
                };
                return Err(Error::new_spanned(
                    ty,
                    format!(
                        "this field nests {levels} list levels{leaf_note}; at most \
                         {MAX_LIST_LEVELS} are supported"
                    ),
                ));
            }
            Ok(())
        }
        ResolvedType::Map { key, value, .. } => {
            check_list_levels(ty, key)?;
            check_list_levels(ty, value)
        }
        ResolvedType::Struct { .. } => Ok(()),
    }
}

fn resolve(ty: &Type, vec_depth: usize, numeric: bool) -> Result<ResolvedType> {
    let err = |msg: &str| Err(Error::new_spanned(ty, msg));

    match ty {
        // A `ty` fragment forwarded by `macro_rules!` arrives wrapped in an
        // invisible group, and means just what it wraps.
        Type::Group(group) => resolve(&group.elem, vec_depth, numeric),
        Type::Paren(paren) => resolve(&paren.elem, vec_depth, numeric),
        Type::Path(path) => {
            let Some(segment) = path.path.segments.last() else {
                return err("empty type path");
            };
            let generics: Vec<&Type> = match &segment.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(t) => Some(t),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };

            let prim = match segment.ident.to_string().as_str() {
                "Vec" if generics.len() == 1 && !numeric && is_u8(generics[0]) => {
                    return Ok(ResolvedType::Leaf {
                        leaf: Leaf::Bytes,
                        vec_depth,
                    });
                }
                "Vec" if generics.len() == 1 => {
                    return resolve(generics[0], vec_depth + 1, numeric);
                }
                "Option" if generics.len() == 1 => {
                    return resolve_option(ty, generics[0], vec_depth);
                }
                // `#[pyro(numeric)]` is on the map field, not on its keys or
                // values, so a `Vec<u8>` in either is bytes.
                "BTreeMap" | "Map" if generics.len() == 2 => {
                    return Ok(ResolvedType::Map {
                        key: Box::new(resolve(generics[0], 0, false)?),
                        value: Box::new(resolve(generics[1], 0, false)?),
                        vec_depth,
                    });
                }
                "String" => {
                    return Ok(ResolvedType::Leaf {
                        leaf: Leaf::Str,
                        vec_depth,
                    });
                }
                "Bytes" | "BytesMut" => {
                    return Ok(ResolvedType::Leaf {
                        leaf: Leaf::Bytes,
                        vec_depth,
                    });
                }
                "Box" if generics.len() == 1 && is_u8_slice(generics[0]) => {
                    return Ok(ResolvedType::Leaf {
                        leaf: Leaf::Bytes,
                        vec_depth,
                    });
                }
                "BitVec" => {
                    validate_bitvec_generics(ty, &segment.arguments)?;
                    return Ok(ResolvedType::Leaf {
                        leaf: Leaf::BitVec,
                        vec_depth,
                    });
                }
                name @ ("Box" | "Rc" | "Arc") => {
                    return Err(Error::new_spanned(
                        ty,
                        format!(
                            "`{name}<T>` fields are not supported by #[derive(Layout, Writer, Reader)] — use `T` directly"
                        ),
                    ));
                }
                name @ ("HashMap" | "IndexMap") => {
                    return Err(Error::new_spanned(
                        ty,
                        format!(
                            "`{name}<K, V>` is not supported — use `pyroformat::Map<K, V>` or `BTreeMap<K, V>`, which keep their keys in order"
                        ),
                    ));
                }
                "usize" | "isize" => {
                    return err("`usize`/`isize` have no fixed width on the wire, use `u64`/`i64`");
                }
                "u8" => Prim::U8,
                "i8" => Prim::I8,
                "u16" => Prim::U16,
                "i16" => Prim::I16,
                "u32" => Prim::U32,
                "i32" => Prim::I32,
                "u64" => Prim::U64,
                "i64" => Prim::I64,
                "f16" => Prim::F16,
                "f32" => Prim::F32,
                "f64" => Prim::F64,
                "bool" => Prim::Bool,
                "Time" => Prim::Time,
                _ => {
                    return Ok(ResolvedType::Struct {
                        ty: ty.clone(),
                        vec_depth,
                        optional: false,
                    });
                }
            };
            Ok(ResolvedType::Leaf {
                leaf: Leaf::Prim(prim),
                vec_depth,
            })
        }
        Type::Array(array) => {
            let len = match ungroup(&array.len) {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => n.base10_parse::<usize>()?,
                _ => return err("a fixed array's length must be an integer literal"),
            };
            // An inode's `fixed_size: 0` means "not a fixed array", so a
            // zero-length one would read back as a scalar or a list.
            if len == 0 {
                return err("a fixed array must have at least one element");
            }
            match resolve(&array.elem, 0, numeric)? {
                ResolvedType::Leaf {
                    leaf: Leaf::Prim(p),
                    vec_depth: 0,
                } => Ok(ResolvedType::Leaf {
                    leaf: Leaf::Array(len, p),
                    vec_depth,
                }),
                _ => err("only arrays of primitives are supported"),
            }
        }
        Type::Tuple(_) => err(
            "tuple fields are not supported by #[derive(Layout, Writer, Reader)] — use a named struct instead",
        ),
        Type::Reference(_) | Type::Slice(_) => {
            err("borrowed fields are not supported, use an owned type")
        }
        _ => err("unsupported field type"),
    }
}

/// `expr` without the invisible groups `macro_rules!` wraps a forwarded
/// `literal`/`expr` fragment in.
pub(crate) fn ungroup(mut expr: &syn::Expr) -> &syn::Expr {
    while let syn::Expr::Group(group) = expr {
        expr = &group.expr;
    }
    expr
}

fn resolve_option(ty: &Type, inner: &Type, vec_depth: usize) -> Result<ResolvedType> {
    let err = |msg: &str| Err(Error::new_spanned(ty, msg));

    // Checked before resolving the inner type, so an optional `HashMap` says
    // what is wrong with the `Option` rather than with the map.
    if let Type::Path(path) = inner
        && let Some(segment) = path.path.segments.last()
        && matches!(
            segment.ident.to_string().as_str(),
            "HashMap" | "IndexMap" | "BTreeMap" | "Map"
        )
    {
        return err(
            "Option<HashMap<K, V>> (or BTreeMap/IndexMap) is not supported — an empty map already represents \"no value\"; use the map directly",
        );
    }

    match resolve(inner, 0, false)? {
        ResolvedType::Leaf {
            leaf: Leaf::Prim(p),
            vec_depth: 0,
        } => Ok(ResolvedType::Leaf {
            leaf: Leaf::OptionPrim(p),
            vec_depth,
        }),
        ResolvedType::Leaf {
            leaf: Leaf::Array(len, p),
            vec_depth: 0,
        } => Ok(ResolvedType::Leaf {
            leaf: Leaf::OptionArray(len, p),
            vec_depth,
        }),
        ResolvedType::Leaf {
            leaf: Leaf::Array(..),
            vec_depth: 1,
        } => err(
            "Option<Vec<[T; N]>> is not supported — an empty Vec already represents \"no value\"; use Vec<[T; N]> directly",
        ),
        ResolvedType::Struct {
            ty,
            vec_depth: 0,
            optional: false,
        } => Ok(ResolvedType::Struct {
            ty,
            vec_depth,
            optional: true,
        }),
        ResolvedType::Leaf {
            leaf: Leaf::Str, ..
        } => err(
            "Option<String> is not supported — an empty String already represents \"no value\"; use String directly",
        ),
        ResolvedType::Leaf {
            leaf: Leaf::BitVec, ..
        } => err(
            "Option<BitVec<u8, Lsb0>> is not supported (out of scope for v1) — use BitVec<u8, Lsb0> directly; an empty BitVec already represents \"no bits\"",
        ),
        ResolvedType::Leaf {
            leaf: Leaf::OptionPrim(_) | Leaf::OptionArray(..),
            ..
        }
        | ResolvedType::Struct { optional: true, .. } => err("Option<Option<T>> is not supported"),
        _ => err(
            "Option<Vec<T>> is not supported — an empty Vec already represents \"no value\"; use Vec<T> directly",
        ),
    }
}

/// Only `BitVec<u8, Lsb0>` (or the bare `BitVec<u8>`, or the
/// `pyroformat::BitVec` alias with no arguments) matches the wire's
/// byte-backed, least-significant-bit-first bool list.
fn validate_bitvec_generics(ty: &Type, args: &PathArguments) -> Result<()> {
    let angle = match args {
        PathArguments::None => return Ok(()),
        PathArguments::AngleBracketed(angle) => angle,
        PathArguments::Parenthesized(_) => {
            return Err(Error::new_spanned(
                ty,
                "pyroformat only supports `BitVec<u8, Lsb0>` (or the bare `BitVec<u8>` spelling, which defaults to `Lsb0`)",
            ));
        }
    };
    let mut args = angle.args.iter();
    let storage_is_u8 = matches!(
        args.next(),
        Some(GenericArgument::Type(Type::Path(p))) if p.path.segments.last().is_some_and(|s| s.ident == "u8")
    );
    if !storage_is_u8 {
        return Err(Error::new_spanned(
            ty,
            "pyroformat only supports `BitVec<u8, Lsb0>` — other backing storage types (e.g. u16/u32/u64) don't match the wire's byte layout",
        ));
    }
    if let Some(order) = args.next() {
        let order_is_lsb0 = matches!(
            order,
            GenericArgument::Type(Type::Path(p)) if p.path.segments.last().is_some_and(|s| s.ident == "Lsb0")
        );
        if !order_is_lsb0 {
            return Err(Error::new_spanned(
                ty,
                "pyroformat only supports `BitVec<u8, Lsb0>` — other bit orders (e.g. Msb0) don't match the wire's byte layout",
            ));
        }
    }
    Ok(())
}

fn is_u8(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.is_ident("u8"))
}

fn is_u8_slice(ty: &Type) -> bool {
    match ty {
        Type::Slice(slice) => matches!(&*slice.elem, Type::Path(p) if p.path.is_ident("u8")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn r(ty: Type) -> ResolvedType {
        resolve_type(&ty, false).unwrap()
    }

    #[test]
    fn primitives_and_depth() {
        assert!(matches!(
            r(parse_quote! { u32 }),
            ResolvedType::Leaf {
                leaf: Leaf::Prim(Prim::U32),
                vec_depth: 0
            }
        ));
        assert!(matches!(
            r(parse_quote! { Vec<Vec<u64>> }),
            ResolvedType::Leaf {
                leaf: Leaf::Prim(Prim::U64),
                vec_depth: 2
            }
        ));
        assert!(matches!(
            r(parse_quote! { Vec<Option<u32>> }),
            ResolvedType::Leaf {
                leaf: Leaf::OptionPrim(Prim::U32),
                vec_depth: 1
            }
        ));
    }

    #[test]
    fn vec_u8_is_bytes_unless_numeric() {
        assert!(matches!(
            r(parse_quote! { Vec<u8> }),
            ResolvedType::Leaf {
                leaf: Leaf::Bytes,
                vec_depth: 0
            }
        ));
        assert!(matches!(
            r(parse_quote! { Vec<Vec<Vec<Vec<u8>>>> }),
            ResolvedType::Leaf {
                leaf: Leaf::Bytes,
                vec_depth: 3
            }
        ));
        assert!(matches!(
            resolve_type(&parse_quote! { Vec<Vec<u8>> }, true).unwrap(),
            ResolvedType::Leaf {
                leaf: Leaf::Prim(Prim::U8),
                vec_depth: 2
            }
        ));
        assert!(matches!(
            r(parse_quote! { Vec<Box<[u8]>> }),
            ResolvedType::Leaf {
                leaf: Leaf::Bytes,
                vec_depth: 1
            }
        ));
        assert!(matches!(
            r(parse_quote! { bytes::Bytes }),
            ResolvedType::Leaf {
                leaf: Leaf::Bytes,
                vec_depth: 0
            }
        ));
    }

    #[test]
    fn structs_maps_and_arrays() {
        assert!(matches!(
            r(parse_quote! { Vec<Option<Point>> }),
            ResolvedType::Struct {
                vec_depth: 1,
                optional: true,
                ..
            }
        ));
        assert!(matches!(
            r(parse_quote! { Option<[u32; 4]> }),
            ResolvedType::Leaf {
                leaf: Leaf::OptionArray(4, Prim::U32),
                vec_depth: 0
            }
        ));
        let ResolvedType::Map { key, value, .. } = r(parse_quote! { BTreeMap<String, Vec<u8>> })
        else {
            panic!("expected a map")
        };
        assert!(matches!(
            *key,
            ResolvedType::Leaf {
                leaf: Leaf::Str,
                vec_depth: 0
            }
        ));
        assert!(matches!(
            *value,
            ResolvedType::Leaf {
                leaf: Leaf::Bytes,
                vec_depth: 0
            }
        ));
    }

    #[test]
    fn rejected_shapes() {
        let rejected: Vec<Type> = vec![
            parse_quote! { Option<String> },
            parse_quote! { Option<Vec<u32>> },
            parse_quote! { Option<Option<u8>> },
            parse_quote! { Box<Point> },
            parse_quote! { (u8, u8) },
            parse_quote! { usize },
            parse_quote! { &'a str },
            parse_quote! { [Point; 4] },
            parse_quote! { HashMap<String, u32> },
            parse_quote! { Option<BTreeMap<String, u32>> },
            parse_quote! { BitVec<u16, Lsb0> },
            parse_quote! { BitVec<u8, Msb0> },
            parse_quote! { Option<BitVec<u8, Lsb0>> },
        ];
        for ty in rejected {
            assert!(resolve_type(&ty, false).is_err(), "{}", quote::quote!(#ty));
        }
    }
}

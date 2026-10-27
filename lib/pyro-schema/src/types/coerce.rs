// =============================================================================
// PyroType coercion
// =============================================================================
//
// Finds the common supertype of two `PyroType`s, used when merging schemas
// from heterogeneous data sources (e.g. JSON rows with varying shapes).

use crate::Bow;
use crate::types::{PrimitiveDataType, PyroType};

/// Coerce two [`PyroType`]s to a common supertype. Returns `None` if incompatible.
pub fn coerce_pyro_types(a: &PyroType, b: &PyroType) -> Option<PyroType> {
    if a == b {
        return Some(a.clone());
    }

    use PyroType::*;

    match (a, b) {
        // Null widens to anything
        (Unknown, other) | (other, Unknown) => Some(other.clone()),

        // --- Primitive scalar coercion ---
        (PrimitiveScalar(pa), PrimitiveScalar(pb)) => {
            coerce_primitive(*pa, *pb).map(PrimitiveScalar)
        }

        // --- PrimitiveList coercion ---
        (PrimitiveList(pa), PrimitiveList(pb)) => coerce_primitive(*pa, *pb).map(PrimitiveList),

        // --- PrimitiveFixedList coercion ---
        // Same size + coercible element → PrimitiveFixedList
        // Different size → promote to PrimitiveList
        (PrimitiveFixedList(pa, sa), PrimitiveFixedList(pb, sb)) => {
            let elem = coerce_primitive(*pa, *pb)?;
            if sa == sb {
                Some(PrimitiveFixedList(elem, *sa))
            } else {
                Some(PrimitiveList(elem))
            }
        }

        // OptionPrimitiveFixedList coercion
        (OptionPrimitiveFixedList(pa, sa), OptionPrimitiveFixedList(pb, sb)) => {
            let elem = coerce_primitive(*pa, *pb)?;
            if sa == sb {
                Some(OptionPrimitiveFixedList(elem, *sa))
            } else {
                Some(List(Bow::new(OptionPrimitiveScalar(elem))))
            }
        }

        (PrimitiveFixedList(pa, sa), OptionPrimitiveFixedList(pb, sb))
        | (OptionPrimitiveFixedList(pa, sa), PrimitiveFixedList(pb, sb)) => {
            let elem = coerce_primitive(*pa, *pb)?;
            if sa == sb {
                Some(OptionPrimitiveFixedList(elem, *sa))
            } else {
                Some(List(Bow::new(OptionPrimitiveScalar(elem))))
            }
        }

        // PrimitiveFixedList + PrimitiveList → PrimitiveList
        (PrimitiveFixedList(pa, _), PrimitiveList(pb))
        | (PrimitiveList(pa), PrimitiveFixedList(pb, _)) => {
            coerce_primitive(*pa, *pb).map(PrimitiveList)
        }

        // OptionPrimitiveFixedList + PrimitiveList
        (OptionPrimitiveFixedList(pa, _), PrimitiveList(pb))
        | (PrimitiveList(pa), OptionPrimitiveFixedList(pb, _)) => {
            let elem = coerce_primitive(*pa, *pb)?;
            Some(List(Bow::new(OptionPrimitiveScalar(elem))))
        }

        // --- List coercion (merge inner types, union nullability) ---
        (List(inner_a), List(inner_b)) => {
            let inner = coerce_pyro_types(inner_a, inner_b)?;
            Some(List(Bow::new(inner)))
        }

        // List + PrimitiveList → List (promote primitive to scalar inner)
        (List(inner), PrimitiveList(p)) | (PrimitiveList(p), List(inner)) => {
            let promoted = PyroType::PrimitiveScalar(*p);
            let inner = coerce_pyro_types(inner, &promoted)?;
            Some(List(Bow::new(inner)))
        }

        // List + PrimitiveFixedList → List
        (List(inner), PrimitiveFixedList(p, _)) | (PrimitiveFixedList(p, _), List(inner)) => {
            let promoted = PyroType::PrimitiveScalar(*p);
            let inner = coerce_pyro_types(inner, &promoted)?;
            Some(List(Bow::new(inner)))
        }

        // --- Group (struct) coercion: positional merge ---
        // Both Groups must have the same number of children.
        // Child types are coerced pairwise.
        (Group(children_a), Group(children_b)) => {
            if children_a.len() != children_b.len() {
                return None;
            }
            let merged: Option<Vec<PyroType>> = children_a
                .iter()
                .zip(children_b.iter())
                .map(|(ca, cb)| coerce_pyro_types(ca, cb))
                .collect();
            Some(Group(merged?.into()))
        }

        // --- Map coercion ---
        (Map { key: ka, value: va }, Map { key: kb, value: vb }) => {
            let key = coerce_pyro_types(ka, kb)?;
            let value = coerce_pyro_types(va, vb)?;
            Some(Map {
                key: Bow::new(key),
                value: Bow::new(value),
            })
        }

        _ => None,
    }
}

// =============================================================================
// Primitive coercion table
// =============================================================================

/// Coerce two [`PrimitiveDataType`]s to a common supertype.
/// Returns `None` if the types are incompatible (e.g. Bool + Int).
pub fn coerce_primitive(a: PrimitiveDataType, b: PrimitiveDataType) -> Option<PrimitiveDataType> {
    if a == b {
        return Some(a);
    }

    use PrimitiveDataType as P;

    match (a, b) {
        // --- Signed integer widening ---
        (P::I8, P::I16) | (P::I16, P::I8) => Some(P::I16),
        (P::I8, P::I32) | (P::I32, P::I8) => Some(P::I32),
        (P::I8, P::I64) | (P::I64, P::I8) => Some(P::I64),
        (P::I16, P::I32) | (P::I32, P::I16) => Some(P::I32),
        (P::I16, P::I64) | (P::I64, P::I16) => Some(P::I64),
        (P::I32, P::I64) | (P::I64, P::I32) => Some(P::I64),

        // --- Unsigned integer widening ---
        (P::U8, P::U16) | (P::U16, P::U8) => Some(P::U16),
        (P::U8, P::U32) | (P::U32, P::U8) => Some(P::U32),
        (P::U8, P::U64) | (P::U64, P::U8) => Some(P::U64),
        (P::U16, P::U32) | (P::U32, P::U16) => Some(P::U32),
        (P::U16, P::U64) | (P::U64, P::U16) => Some(P::U64),
        (P::U32, P::U64) | (P::U64, P::U32) => Some(P::U64),

        // --- Float widening ---
        (P::F16, P::F32) | (P::F32, P::F16) => Some(P::F32),
        (P::F16, P::F64) | (P::F64, P::F16) => Some(P::F64),
        (P::F32, P::F64) | (P::F64, P::F32) => Some(P::F64),

        // --- Signed int → F64 ---
        (P::I8 | P::I16 | P::I32 | P::I64, P::F64) | (P::F64, P::I8 | P::I16 | P::I32 | P::I64) => {
            Some(P::F64)
        }

        // --- Unsigned int → F64 ---
        (P::U8 | P::U16 | P::U32 | P::U64, P::F64) | (P::F64, P::U8 | P::U16 | P::U32 | P::U64) => {
            Some(P::F64)
        }

        // --- Signed int → F32 (only small ints to avoid precision loss) ---
        (P::I8 | P::I16, P::F32) | (P::F32, P::I8 | P::I16) => Some(P::F32),

        // --- Unsigned int → F32 (only small ints) ---
        (P::U8 | P::U16, P::F32) | (P::F32, P::U8 | P::U16) => Some(P::F32),

        // --- Larger ints + F32 → F64 ---
        (P::I32 | P::I64, P::F32) | (P::F32, P::I32 | P::I64) => Some(P::F64),
        (P::U32 | P::U64, P::F32) | (P::F32, P::U32 | P::U64) => Some(P::F64),

        // --- Cross-sign int → widen signed (safe direction) ---
        (P::U8, P::I16) | (P::I16, P::U8) => Some(P::I16),
        (P::U8, P::I32) | (P::I32, P::U8) => Some(P::I32),
        (P::U8, P::I64) | (P::I64, P::U8) => Some(P::I64),
        (P::U16, P::I32) | (P::I32, P::U16) => Some(P::I32),
        (P::U16, P::I64) | (P::I64, P::U16) => Some(P::I64),
        (P::U32, P::I64) | (P::I64, P::U32) => Some(P::I64),

        // Everything else is incompatible (Bool + numeric, U64 + I8, etc.)
        _ => None,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Primitive coercion
    // -------------------------------------------------------------------------

    #[test]
    fn test_same_type() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I32, PrimitiveDataType::I32),
            Some(PrimitiveDataType::I32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::Bool, PrimitiveDataType::Bool),
            Some(PrimitiveDataType::Bool)
        );
    }

    #[test]
    fn test_signed_widening() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I8, PrimitiveDataType::I16),
            Some(PrimitiveDataType::I16)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I16, PrimitiveDataType::I64),
            Some(PrimitiveDataType::I64)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I32, PrimitiveDataType::I64),
            Some(PrimitiveDataType::I64)
        );
    }

    #[test]
    fn test_unsigned_widening() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U8, PrimitiveDataType::U32),
            Some(PrimitiveDataType::U32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U16, PrimitiveDataType::U64),
            Some(PrimitiveDataType::U64)
        );
    }

    #[test]
    fn test_float_widening() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::F16, PrimitiveDataType::F32),
            Some(PrimitiveDataType::F32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::F32, PrimitiveDataType::F64),
            Some(PrimitiveDataType::F64)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::F16, PrimitiveDataType::F64),
            Some(PrimitiveDataType::F64)
        );
    }

    #[test]
    fn test_int_to_float() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I8, PrimitiveDataType::F32),
            Some(PrimitiveDataType::F32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I32, PrimitiveDataType::F32),
            Some(PrimitiveDataType::F64)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::I64, PrimitiveDataType::F64),
            Some(PrimitiveDataType::F64)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U8, PrimitiveDataType::F32),
            Some(PrimitiveDataType::F32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U32, PrimitiveDataType::F32),
            Some(PrimitiveDataType::F64)
        );
    }

    #[test]
    fn test_cross_sign() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U8, PrimitiveDataType::I16),
            Some(PrimitiveDataType::I16)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U16, PrimitiveDataType::I32),
            Some(PrimitiveDataType::I32)
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::U32, PrimitiveDataType::I64),
            Some(PrimitiveDataType::I64)
        );
    }

    #[test]
    fn test_bool_incompatible() {
        assert_eq!(
            coerce_primitive(PrimitiveDataType::Bool, PrimitiveDataType::I32),
            None
        );
        assert_eq!(
            coerce_primitive(PrimitiveDataType::Bool, PrimitiveDataType::F64),
            None
        );
    }

    #[test]
    fn test_symmetric() {
        // Every coercion should be symmetric
        let all = [
            PrimitiveDataType::Bool,
            PrimitiveDataType::I8,
            PrimitiveDataType::I16,
            PrimitiveDataType::I32,
            PrimitiveDataType::I64,
            PrimitiveDataType::U8,
            PrimitiveDataType::U16,
            PrimitiveDataType::U32,
            PrimitiveDataType::U64,
            PrimitiveDataType::F16,
            PrimitiveDataType::F32,
            PrimitiveDataType::F64,
        ];
        for &a in &all {
            for &b in &all {
                assert_eq!(
                    coerce_primitive(a, b),
                    coerce_primitive(b, a),
                    "coerce_primitive is not symmetric for {a:?}, {b:?}"
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // PyroType coercion
    // -------------------------------------------------------------------------

    #[test]
    fn test_null_widens() {
        assert_eq!(
            coerce_pyro_types(&PyroType::Unknown, &PyroType::Str),
            Some(PyroType::Str)
        );
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                &PyroType::Unknown
            ),
            Some(PyroType::PrimitiveScalar(PrimitiveDataType::I32))
        );
        assert_eq!(
            coerce_pyro_types(&PyroType::Unknown, &PyroType::Unknown),
            Some(PyroType::Unknown)
        );
    }

    #[test]
    fn test_scalar_coercion() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveScalar(PrimitiveDataType::I8),
                &PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            ),
            Some(PyroType::PrimitiveScalar(PrimitiveDataType::I32))
        );
    }

    #[test]
    fn test_primitive_list_coercion() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveList(PrimitiveDataType::F32),
                &PyroType::PrimitiveList(PrimitiveDataType::F64),
            ),
            Some(PyroType::PrimitiveList(PrimitiveDataType::F64))
        );
    }

    #[test]
    fn test_fixed_list_same_size() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128),
                &PyroType::PrimitiveFixedList(PrimitiveDataType::F64, 128),
            ),
            Some(PyroType::PrimitiveFixedList(PrimitiveDataType::F64, 128))
        );
    }

    #[test]
    fn test_fixed_list_different_size() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 64),
                &PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128),
            ),
            Some(PyroType::PrimitiveList(PrimitiveDataType::F32))
        );
    }

    #[test]
    fn test_fixed_list_plus_list() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 10),
                &PyroType::PrimitiveList(PrimitiveDataType::I64),
            ),
            Some(PyroType::PrimitiveList(PrimitiveDataType::I64))
        );
    }

    #[test]
    fn test_list_coercion() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::List(Bow::new(PyroType::Str)),
                &PyroType::List(Bow::new(PyroType::Str)),
            ),
            Some(PyroType::List(Bow::new(PyroType::Str)))
        );
    }

    #[test]
    fn test_list_inner_coercion() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),),
                &PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),),
            ),
            Some(PyroType::List(Bow::new(PyroType::PrimitiveScalar(
                PrimitiveDataType::I64
            )),))
        );
    }

    #[test]
    fn test_list_plus_primitive_list() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),),
                &PyroType::PrimitiveList(PrimitiveDataType::I64),
            ),
            Some(PyroType::List(Bow::new(PyroType::PrimitiveScalar(
                PrimitiveDataType::I64
            )),))
        );
    }

    #[test]
    fn test_group_coercion() {
        let a = PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                PyroType::Str,
            ]
            .into(),
        );
        let b = PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                PyroType::Str,
            ]
            .into(),
        );

        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::Group(
                vec![
                    PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                    PyroType::Str,
                ]
                .into()
            ))
        );
    }

    #[test]
    fn test_group_different_length_fails() {
        let a = PyroType::Group(vec![PyroType::Str].into());
        let b = PyroType::Group(vec![PyroType::Str, PyroType::Str].into());
        assert_eq!(coerce_pyro_types(&a, &b), None);
    }

    #[test]
    fn test_group_incompatible_child_fails() {
        let a = PyroType::Group(vec![PyroType::Str].into());
        let b = PyroType::Group(vec![PyroType::PrimitiveScalar(PrimitiveDataType::I32)].into());
        assert_eq!(coerce_pyro_types(&a, &b), None);
    }

    #[test]
    fn test_map_coercion() {
        let a = PyroType::Map {
            key: Bow::new(PyroType::Str),
            value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
        };
        let b = PyroType::Map {
            key: Bow::new(PyroType::Str),
            value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        };

        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::Map {
                key: Bow::new(PyroType::Str),
                value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            })
        );
    }

    #[test]
    fn test_incompatible_top_level() {
        assert_eq!(
            coerce_pyro_types(
                &PyroType::Str,
                &PyroType::PrimitiveScalar(PrimitiveDataType::I32)
            ),
            None,
        );
        assert_eq!(
            coerce_pyro_types(
                &PyroType::Str,
                &PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp)
            ),
            None,
        );
        assert_eq!(
            coerce_pyro_types(
                &PyroType::PrimitiveList(PrimitiveDataType::I32),
                &PyroType::Str,
            ),
            None,
        );
    }

    #[test]
    fn test_nested_list_of_group_coercion() {
        let a = PyroType::List(Bow::new(PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                PyroType::Str,
            ]
            .into(),
        )));
        let b = PyroType::List(Bow::new(PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                PyroType::Str,
            ]
            .into(),
        )));

        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::List(Bow::new(PyroType::Group(
                vec![
                    PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                    PyroType::Str,
                ]
                .into()
            )),))
        );
    }

    #[test]
    fn test_coerce_is_symmetric() {
        let types = vec![
            PyroType::Unknown,
            PyroType::Str,
            PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            PyroType::PrimitiveList(PrimitiveDataType::I32),
            PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128),
            PyroType::List(Bow::new(PyroType::Str)),
            PyroType::Group(vec![PyroType::Str, PyroType::Str].into()),
            PyroType::Map {
                key: Bow::new(PyroType::Str),
                value: Bow::new(PyroType::Str),
            },
        ];

        for a in &types {
            for b in &types {
                assert_eq!(
                    coerce_pyro_types(a, b),
                    coerce_pyro_types(b, a),
                    "coerce_pyro_types is not symmetric for {a:?}, {b:?}"
                );
            }
        }
    }
}

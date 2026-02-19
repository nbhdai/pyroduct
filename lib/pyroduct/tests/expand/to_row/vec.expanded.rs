//! Test ToRow with Vec fields
use pyroduct::{ToRow, PyroValue, PrimitiveValueList};
use std::borrow::Cow;
struct WithVec {
    scores: Vec<i32>,
    values: Vec<f64>,
    strs: Vec<String>,
}
impl ::pyroduct::ToRow for WithVec {
    fn to_row(&self) -> ::pyroduct::PyroRow<'_> {
        ::pyroduct::PyroRow::from([
            ("scores", ::pyroduct::PyroValue::from(&self.scores)),
            ("values", ::pyroduct::PyroValue::from(&self.values)),
            ("strs", ::pyroduct::PyroValue::from(&self.strs)),
        ])
    }
}
fn main() {
    let data = WithVec {
        scores: ::alloc::boxed::box_assume_init_into_vec_unsafe(
            ::alloc::intrinsics::write_box_via_move(
                ::alloc::boxed::Box::new_uninit(),
                [1, 2, 3],
            ),
        ),
        values: ::alloc::boxed::box_assume_init_into_vec_unsafe(
            ::alloc::intrinsics::write_box_via_move(
                ::alloc::boxed::Box::new_uninit(),
                [1.1, 2.2],
            ),
        ),
        strs: ::alloc::boxed::box_assume_init_into_vec_unsafe(
            ::alloc::intrinsics::write_box_via_move(
                ::alloc::boxed::Box::new_uninit(),
                ["hi".to_string()],
            ),
        ),
    };
    let row = data.to_row();
    if let Some(PyroValue::PrimitiveList(PrimitiveValueList::I32(list))) = row
        .get("scores")
    {
        match list {
            Cow::Borrowed(slice) => {
                match (&slice, &&[1, 2, 3]) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            _ => {
                ::core::panicking::panic_fmt(format_args!("Expected borrowed slice"));
            }
        }
    } else {
        {
            ::core::panicking::panic_fmt(
                format_args!("Expected PrimitiveList for scores"),
            );
        };
    }
}

//! Test ToRow with Vec fields

use pyro_vec::{ToRow, ArrowValue, PrimitiveValueList};
use std::borrow::Cow;

#[derive(ToRow)]
struct WithVec {
    scores: Vec<i32>,
    values: Vec<f64>,
}

fn main() {
    let data = WithVec {
        scores: vec![1, 2, 3],
        values: vec![1.1, 2.2],
    };
    
    let row = data.to_row();
    
    // Vec should be converted to PrimitiveList (borrowed)
    if let Some(ArrowValue::PrimitiveList(PrimitiveValueList::I32(list))) = row.get("scores") {
        match list {
            Cow::Borrowed(slice) => {
                assert_eq!(slice, &[1, 2, 3]);
            }
            _ => panic!("Expected borrowed slice"),
        }
    } else {
        panic!("Expected PrimitiveList for scores");
    }
}
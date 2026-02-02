use pyroduct::arrow_scalars::{ArrowRow, ArrowValue};
use pyroduct::errors::{ArchivedFfiError, FfiError};
use pyroduct::wasm_module::{call, dealloc};
use pyroduct::{DeepRef, FromRow, ToRow};

struct UserInput {
    name: String,
    age: i32,
}

// Manually implementing these as the macro is messed up within pyroduct
#[derive(Debug, Clone, PartialEq)]
struct UserInputRef<'a> {
    name: &'a str,
    age: i32,
}

impl DeepRef for UserInput {
    type Ref<'a> = UserInputRef<'a>;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        UserInputRef {
            name: &self.name,
            age: self.age,
        }
    }
}

impl<'a> FromRow<'a> for UserInputRef<'a> {
    fn from_row(row: &ArrowRow<'a>) -> Result<Self, String> {
        let name_val = row.get("name").ok_or("Missing field 'name'")?;
        let age_val = row.get("age").ok_or("Missing field 'age'")?;

        let name = match name_val {
            ArrowValue::Str(c) => c.as_ref(),
            _ => return Err("Field 'name' is not a string".into()),
        };

        let age = match age_val {
            ArrowValue::I32(v) => *v,
            _ => return Err("Field 'age' is not an i32".into()),
        };

        // Lifetime reset to 'a
        let name = unsafe { std::mem::transmute(name) };

        Ok(UserInputRef { name, age })
    }
}

#[derive(Debug, Clone)]
struct UserOutput {
    greeting: String,
    is_adult: bool,
}

impl ToRow for UserOutput {
    fn to_row(&self) -> ArrowRow<'_> {
        ArrowRow::from([
            ("greeting", ArrowValue::from(self.greeting.clone())),
            ("is_adult", ArrowValue::from(self.is_adult)),
        ])
    }
}

#[test]
fn test_ffi_call_happy_path() {
    let input_row = ArrowRow::from([
        ("name", ArrowValue::from("Alice")),
        ("age", ArrowValue::from(30i32)),
    ]);
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input_row).unwrap();

    // This is memory managed by the wasm module, as we are both host and wasm in this test we hold it.
    let mut input_vec = input_bytes.into_vec();
    let input_ptr = input_vec.as_mut_ptr();
    let input_len = input_vec.len();

    let user_logic = |input: &UserInputRef<'_>| -> Result<UserOutput, String> {
        assert_eq!(input.name, "Alice");
        assert_eq!(input.age, 30);

        Ok(UserOutput {
            greeting: format!("Hello, {}!", input.name),
            is_adult: input.age >= 18,
        })
    };

    let (out_ptr, out_len) = call::<UserInput, UserOutput, _>(input_ptr, input_len, user_logic);

    let out_slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len as usize) };
    type ReturnType<'a> = Result<ArrowRow<'a>, String>;
    let archived =
        rkyv::access::<<ReturnType as rkyv::Archive>::Archived, rkyv::rancor::Error>(out_slice)
            .expect("Failed to access result archive");

    let result: Result<ArrowRow, String> =
        rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();

    match result {
        Ok(row) => {
            let greeting = row.get("greeting").expect("Missing greeting");
            assert_eq!(greeting, &ArrowValue::from("Hello, Alice!"));

            let is_adult = row.get("is_adult").expect("Missing is_adult");
            assert_eq!(is_adult, &ArrowValue::from(true));
        }
        Err(e) => panic!("FFI call returned logic error: {}", e),
    }

    unsafe { dealloc(out_ptr, out_len as usize) }
    // input_vec is dropped here naturally
}

#[test]
fn test_ffi_call_logic_panic() {
    let input_row = ArrowRow::from([
        ("name", ArrowValue::from("Bob")),
        ("age", ArrowValue::from(10i32)),
    ]);
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input_row).unwrap();

    // Keep the vector alive - don't forget it
    let mut input_vec = input_bytes.into_vec();
    let input_ptr = input_vec.as_mut_ptr();
    let input_len = input_vec.len();
    // Don't forget - let it live through the test

    let user_logic = |_: &UserInputRef<'_>| -> Result<UserOutput, String> {
        panic!("Something went terribly wrong!");
    };

    let (out_ptr, out_len) = call::<UserInput, UserOutput, _>(input_ptr, input_len, user_logic);

    let out_slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len as usize) };

    let archived = rkyv::access::<ArchivedFfiError, rkyv::rancor::Error>(out_slice)
        .expect("Output should be a valid FfiError archive");

    let error: FfiError = rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();

    match error {
        FfiError::ModuleLogicPanicked(info) => {
            assert!(info.message.contains("Something went terribly wrong"));
        }
        _ => panic!("Expected ModuleLogicPanicked, got {:?}", error),
    }

    unsafe { dealloc(out_ptr, out_len as usize) };
}

use odra::casper_types::ApiError;
#[test]
fn test_errs() {
    println!("MissingArg: {}", ApiError::MissingArgument as u16);
}

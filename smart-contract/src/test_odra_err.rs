use odra::ExecutionError;

#[test]
fn test_all_odra_errs() {
    for i in 64000..65535 {
        if let ExecutionError::User(code) = ExecutionError::from(i) {
            // Wait, from() constructs an ExecutionError from u16.
            // But how do we get the name of the ExecutionError if we have 64649?
        }
    }
}

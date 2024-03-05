
/// Makes a foo function that is compatible with the expectations of the guest code.
pub fn make_foo(
    structs: &[&str],
    fun_sig: &str,
    fun_body: &str,
) -> Vec<u8> {
    vec![0]
}

/// Uses make_foo to make a very simple function that returns a u64 (42)
pub fn return_u64() -> Vec<u8> {
    make_foo(&[], "(): u64", "16")
}
pub fn generate_registration_id(extend_range: bool) -> i32 {
    if extend_range {
        rand::random_range(0..i32::MAX) + 1
    } else {
        rand::random_range(0..16380) + 1
    }
}

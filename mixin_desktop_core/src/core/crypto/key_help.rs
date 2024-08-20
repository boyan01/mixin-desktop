use rand::Rng;

pub fn generate_registration_id(extend_range: bool) -> i32 {
    let mut rng = rand::thread_rng();
    if extend_range {
        rng.gen_range(0..i32::MAX) + 1
    } else {
        rng.gen_range(0..16380) + 1
    }
}

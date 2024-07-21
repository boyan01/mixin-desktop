pub fn unique_id_from_bytes(name: &[u8]) -> uuid::Uuid {
    let mut bytes = md5::compute(name).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // Set the version to 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Set the variant to DCE 1.1

    uuid::Builder::from_bytes(bytes).into_uuid()
}

pub fn unique_object_id<T: AsRef<str>>(args: &[T]) -> uuid::Uuid {
    let mut ctx = md5::Context::new();
    for s in args {
        ctx.consume(s.as_ref())
    }
    let mut bytes = ctx.compute().0;

    bytes[6] = (bytes[6] & 0x0f) | 0x30; // Set the version to 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Set the variant to DCE 1.1
    uuid::Builder::from_bytes(bytes).into_uuid()
}

#[cfg(test)]
mod test {
    use crate::core::util::unique_object_id;

    #[test]
    fn test_unique_object_id() {
        let id = unique_object_id(&vec!["a", "a"]);
        let id2 = unique_object_id(&vec!["a", "a"]);
        assert_eq!(id, id2)
    }
}

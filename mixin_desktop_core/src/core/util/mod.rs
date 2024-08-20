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

pub fn generate_conversation_id(sender_id: &str, recipient_id: &str) -> uuid::Uuid {
    let mut args = vec![sender_id, recipient_id];
    args.sort();
    unique_object_id(&args)
}

#[cfg(test)]
mod test {
    use crate::core::util::unique_object_id;

    #[test]
    fn test_unique_object_id() {
        let id = unique_object_id(&["a", "a"]);
        let id2 = unique_object_id(&["a", "a"]);
        assert_eq!(id, id2)
    }
}

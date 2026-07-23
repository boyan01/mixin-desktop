pub fn unique_object_id<T: AsRef<str>>(args: &[T]) -> uuid::Uuid {
    let mut context = md5::Context::new();
    for arg in args {
        context.consume(arg.as_ref());
    }
    let mut bytes = context.finalize().0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Builder::from_bytes(bytes).into_uuid()
}

pub fn generate_conversation_id(sender_id: &str, recipient_id: &str) -> uuid::Uuid {
    let mut ids = [sender_id, recipient_id];
    ids.sort();
    unique_object_id(&ids)
}

pub fn group_conversation_id<T: AsRef<str>>(
    owner_id: &str,
    name: &str,
    user_ids: &[T],
    random_id: &str,
) -> String {
    let mut conversation_id = generate_conversation_id(owner_id, name).to_string();
    conversation_id = generate_conversation_id(&conversation_id, random_id).to_string();
    let mut user_ids = user_ids
        .iter()
        .map(|user_id| user_id.as_ref())
        .collect::<Vec<_>>();
    user_ids.sort_unstable();
    for user_id in user_ids {
        conversation_id = generate_conversation_id(&conversation_id, user_id).to_string();
    }
    conversation_id
}

#[cfg(test)]
mod tests {
    use super::{group_conversation_id, unique_object_id};

    #[test]
    fn unique_object_id_is_deterministic() {
        assert_eq!(unique_object_id(&["a", "a"]), unique_object_id(&["a", "a"]));
    }

    #[test]
    fn group_conversation_id_matches_flutter() {
        let user_ids = [
            "f937ca18-d1ff-46f5-99e8-e23fbd6fd5f2",
            "0e0a20c8-31b8-4093-81b8-9cebd9bc8afc",
            "8391e472-cdbe-4704-be1f-7d184635b885",
            "831fdb67-13ed-4dc5-ac64-dda89aeda2bb",
            "f7ff9dde-18c2-4375-8097-b364068b120e",
            "088c1e3e-1f07-4065-85b5-6b49b4370d32",
        ];
        assert_eq!(
            group_conversation_id(
                "c8cb0ac7-d456-4341-be66-0b143aa09922",
                "Mixin Rocks",
                &user_ids,
                "01d21e2c-76f5-4940-8ea0-9b7f21728674",
            ),
            "5dac944e-2037-31b4-bbd9-e5fd3ffe571e"
        );
    }
}

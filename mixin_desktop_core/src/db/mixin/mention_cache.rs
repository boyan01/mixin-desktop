use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::user::User;

const CONTENT_CACHE_CAPACITY: usize = 4096;
const USER_CACHE_CAPACITY: usize = 8192;

#[derive(Clone, Default)]
pub struct MentionCache(Arc<Mutex<MentionCacheState>>);

#[derive(Default)]
struct MentionCacheState {
    contents: HashMap<String, Vec<String>>,
    users: HashMap<String, String>,
}

impl MentionCache {
    pub fn missing_identity_numbers(&self, contents: &[String]) -> Vec<String> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let mut identity_numbers = HashSet::new();
        for content in contents {
            let mentions = match state.contents.get(content) {
                Some(mentions) => mentions.clone(),
                None => {
                    let mentions = mention_identity_numbers(content);
                    if state.contents.len() >= CONTENT_CACHE_CAPACITY {
                        state.contents.clear();
                    }
                    state.contents.insert(content.clone(), mentions.clone());
                    mentions
                }
            };
            identity_numbers.extend(mentions);
        }
        identity_numbers
            .into_iter()
            .filter(|identity_number| !state.users.contains_key(identity_number))
            .collect()
    }

    pub fn cache_users(&self, users: &[User]) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.users.len() + users.len() > USER_CACHE_CAPACITY {
            state.users.clear();
        }
        for user in users {
            if user.full_name.is_empty() {
                state.users.remove(&user.identity_number);
            } else {
                state
                    .users
                    .insert(user.identity_number.clone(), user.full_name.clone());
            }
        }
    }

    pub fn replace_mentions(&self, content: &str) -> String {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let identity_numbers = match state.contents.get(content) {
            Some(identity_numbers) => identity_numbers.clone(),
            None => {
                let identity_numbers = mention_identity_numbers(content);
                if state.contents.len() >= CONTENT_CACHE_CAPACITY {
                    state.contents.clear();
                }
                state
                    .contents
                    .insert(content.to_string(), identity_numbers.clone());
                identity_numbers
            }
        };
        replace_mentions(content, &identity_numbers, &state.users)
    }

    pub fn mention_names(&self, contents: &[String]) -> HashMap<String, String> {
        let state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        contents
            .iter()
            .flat_map(|content| state.contents.get(content).into_iter().flatten())
            .filter_map(|identity_number| {
                state
                    .users
                    .get(identity_number)
                    .map(|full_name| (identity_number.clone(), full_name.clone()))
            })
            .collect()
    }
}

fn mention_identity_numbers(content: &str) -> Vec<String> {
    let mut identity_numbers = Vec::new();
    for (index, _) in content.match_indices('@') {
        let identity_number = content[index + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if identity_number.len() >= 4 && !identity_numbers.contains(&identity_number) {
            identity_numbers.push(identity_number);
        }
    }
    identity_numbers.sort_by_key(|identity_number| std::cmp::Reverse(identity_number.len()));
    identity_numbers
}

fn replace_mentions(
    content: &str,
    identity_numbers: &[String],
    users: &HashMap<String, String>,
) -> String {
    let mut result = content.to_string();
    for identity_number in identity_numbers {
        if let Some(full_name) = users.get(identity_number) {
            result = result.replace(&format!("@{identity_number}"), &format!("@{full_name}"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{mention_identity_numbers, replace_mentions};

    #[test]
    fn parses_and_replaces_flutter_style_mentions() {
        assert_eq!(
            mention_identity_numbers("hello @1039549 and @26832"),
            ["1039549", "26832"]
        );
        assert_eq!(
            replace_mentions(
                "hello @1039549 and @26832",
                &["1039549".to_string(), "26832".to_string()],
                &HashMap::from([
                    ("1039549".to_string(), "Alice".to_string()),
                    ("26832".to_string(), "Bob".to_string()),
                ]),
            ),
            "hello @Alice and @Bob"
        );
        assert_eq!(
            replace_mentions(
                "@1039549 paid 1039549",
                &["1039549".to_string()],
                &HashMap::from([("1039549".to_string(), "Alice".to_string())]),
            ),
            "@Alice paid 1039549"
        );
    }
}

use crate::db::storage::{IntoValue, Query};
pub fn expand_var(amount: usize) -> String {
    if amount == 0 {
        return "".to_string();
    }
    format!("?{}", ", ?".repeat(amount - 1))
}

pub fn expand_var_with_index(start: usize, amount: usize) -> String {
    if amount == 0 {
        return "".to_string();
    }
    let mut buffer = String::new();
    for i in start..start + amount {
        buffer.push('$');
        buffer.push_str(&i.to_string());
        if i != start + amount - 1 {
            buffer.push(',');
        }
    }
    buffer
}

pub trait BindList {
    fn bind_list<T: IntoValue + Clone>(self, list: &[T]) -> Self;
}
impl<O> BindList for Query<O> {
    fn bind_list<T: IntoValue + Clone>(mut self, list: &[T]) -> Self {
        for item in list {
            self = self.bind(item);
        }
        self
    }
}
pub use BindList as BindListForQuery;
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_expand_var() {
        assert_eq!("", expand_var(0));
        assert_eq!("?", expand_var(1));
        assert_eq!("?, ?", expand_var(2));
    }

    #[test]
    fn test_expand_var_with_index() {
        assert_eq!("$1", expand_var_with_index(1, 1));
        assert_eq!("$1,$2", expand_var_with_index(1, 2));
        assert_eq!("$1,$2,$3", expand_var_with_index(1, 3));
    }
}

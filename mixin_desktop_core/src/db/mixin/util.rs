use sqlx::query::{Query, QueryAs};
use sqlx::{Database, Encode, Type};

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

pub trait BindList<'q, DB: Database, O> {
    fn bind_list<T: 'q + Encode<'q, DB> + Type<DB>>(self, list: &'q [T]) -> Self;
}

impl<'q, DB: Database, O> BindList<'q, DB, O>
    for QueryAs<'q, DB, O, <DB as Database>::Arguments<'q>>
{
    fn bind_list<T: 'q + Encode<'q, DB> + Type<DB>>(self, list: &'q [T]) -> Self {
        let mut query = self;
        for item in list.iter() {
            query = query.bind(item);
        }
        query
    }
}

pub trait BindListForQuery<'q, DB: Database> {
    fn bind_list<T: 'q + Encode<'q, DB> + Type<DB>>(self, list: &'q [T]) -> Self;
}

impl<'q, DB: Database> BindListForQuery<'q, DB> for Query<'q, DB, <DB as Database>::Arguments<'q>> {
    fn bind_list<T: 'q + Encode<'q, DB> + Type<DB>>(self, list: &'q [T]) -> Self {
        let mut query = self;
        for item in list.iter() {
            query = query.bind(item);
        }
        query
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_expand_var() {
        assert_eq!("?", expand_var(0));
        assert_eq!("?1", expand_var(1));
        assert_eq!("?$1,?$2", expand_var(2));
    }

    #[test]
    fn test_expand_var_with_index() {
        assert_eq!("$1", expand_var_with_index(1, 1));
        assert_eq!("$1,$2", expand_var_with_index(1, 2));
        assert_eq!("$1,$2,$3", expand_var_with_index(1, 3));
    }
}

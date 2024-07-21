use chrono::NaiveDateTime;

use crate::db;
use crate::db::mixin::MixinDatabase;

impl MixinDatabase {
    pub fn update_message_expired_at(
        &self,
        mid: &String,
        time: &NaiveDateTime,
    ) -> Result<(), db::Error> {
        todo!();
        // let a = diesel::update(
        //     expired_messages.filter(message_id.eq(mid).and(expire_at.is_null().or(expire_at.gt(time)))))
        //     .set(expire_at.eq(time))
        //     .execute(&mut self.get_connection()?);
        Ok(())
    }
}

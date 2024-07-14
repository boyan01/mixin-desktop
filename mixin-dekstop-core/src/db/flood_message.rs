use chrono::NaiveDateTime;
use diesel::associations::HasTable;
use diesel::dsl::insert_into;
use diesel::prelude::*;

use db::Error;

use crate::db;
use crate::db::flood_messages;
use crate::db::flood_messages::dsl::*;
use crate::db::MixinDatabase;

#[derive(Selectable, Queryable)]
#[derive(Insertable)]
pub struct FloodMessage {
    pub message_id: String,
    pub data: String,
    pub created_at: NaiveDateTime,
}


impl MixinDatabase {
    pub fn insert_flood_message(&self, message: FloodMessage) -> Result<(), Error> {
        insert_into(flood_messages).values(message).execute(&mut self.get_connection()?)?;
        Ok(())
    }

    pub fn flood_messages(&self) -> Result<Vec<FloodMessage>, Error> {
        let r = flood_messages::table
            .order(created_at.asc())
            .limit(10)
            .load::<FloodMessage>(&mut self.get_connection()?)?;
        Ok(r)
    }

    pub fn delete_flood_message(&self, m_id: &String) -> Result<usize, Error> {
        let ret = diesel::delete(flood_messages::table.filter(message_id.eq(m_id)))
            .execute(&mut self.get_connection()?)?;
        Ok(ret)
    }

    pub fn latest_flood_message_created_at(&self) -> Result<Option<NaiveDateTime>, Error> {
        let result = flood_messages::table.select(created_at)
            .order(created_at.desc())
            .first::<NaiveDateTime>(&mut self.get_connection()?);
        match result {
            Ok(value) => Ok(Some(value)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(err) => Err(Error::from(err))
        }
    }
}


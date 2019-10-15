use chrono::naive::NaiveDate;
use diesel::prelude::*;
use std::ops::Deref;
use uuid::Uuid;

use super::{
    interface::SanctionsDb,
    models::{CreateSanction, Sanction},
};
use crate::database::{
    postgres::{DbConnection, DbError},
    schema::sanctions,
};

impl SanctionsDb for DbConnection {
    fn get_sanctions(
        &self,
        team_id: Uuid,
        date_interval: Option<(NaiveDate, NaiveDate)>,
    ) -> Result<Vec<Sanction>, DbError> {
        let sanctions = match date_interval {
            Some((min, max)) => sanctions::table
                .filter(
                    sanctions::created_at
                        .between(min, max)
                        .and(sanctions::team_id.eq(team_id)),
                )
                .get_results(self.deref())?,
            None => sanctions::table
                .filter(sanctions::team_id.eq(team_id))
                .get_results(self.deref())?,
        };

        Ok(sanctions)
    }

    fn create_sanction(&self, sanction: &CreateSanction) -> Result<Sanction, DbError> {
        let sanction: Sanction = diesel::insert_into(sanctions::table)
            .values(sanction)
            .get_result(self.deref())?;

        Ok(sanction)
    }

    fn delete_sanction(&self, team_id: Uuid, sanction_id: Uuid) -> Result<Sanction, DbError> {
        let sanction: Sanction = diesel::delete(
            sanctions::table.filter(
                sanctions::team_id
                    .eq(team_id)
                    .and(sanctions::id.eq(sanction_id)),
            ),
        )
        .get_result(self.deref())?;

        Ok(sanction)
    }
}

#[cfg(test)]
mod tests {
    use diesel::result::Error;

    use super::*;
    use crate::teams::{interface::TeamsDb, models::Team};
    use crate::test_utils::postgres::init_connection;
    use crate::users::{interface::UsersDb, models::User};

    #[test]
    fn test_get_sanctions() {
        let conn = init_connection();
        conn.deref().test_transaction::<_, Error, _>(|| {
            let team_id = conn.create_team(&Team::default()).unwrap().id;
            let user_id = conn
                .create_user(&User {
                    team_id,
                    ..Default::default()
                })
                .unwrap()
                .id;
            let sanction = conn
                .create_sanction(&CreateSanction {
                    user_id,
                    team_id,
                    ..Default::default()
                })
                .unwrap();

            let team_id_2 = conn
                .create_team(&Team {
                    id: Uuid::new_v4(),
                    name: String::from("CHBC"),
                    ..Default::default()
                })
                .unwrap()
                .id;
            let user_id_2 = conn
                .create_user(&User {
                    id: Uuid::new_v4(),
                    team_id: team_id_2,
                    ..Default::default()
                })
                .unwrap()
                .id;
            let sanction_2 = conn
                .create_sanction(&CreateSanction {
                    id: Uuid::new_v4(),
                    user_id: user_id_2,
                    team_id: team_id_2,
                    ..Default::default()
                })
                .unwrap();

            let sanctions: Vec<Sanction> = conn.get_sanctions(team_id, None).unwrap();
            let sanctions_2: Vec<Sanction> = conn.get_sanctions(team_id_2, None).unwrap();

            assert_eq!(vec![sanction], sanctions);
            assert_eq!(vec![sanction_2], sanctions_2);

            Ok(())
        });
    }

    #[test]
    fn test_get_sanctions_with_date_interval() {
        let conn = init_connection();

        conn.deref().test_transaction::<_, Error, _>(|| {
            let team_id = conn.create_team(&Team::default()).unwrap().id;
            let user_id = conn
                .create_user(&User {
                    team_id,
                    ..Default::default()
                })
                .unwrap()
                .id;

            let sanction = conn
                .create_sanction(&CreateSanction {
                    user_id,
                    team_id,
                    created_at: Some(NaiveDate::from_ymd(2019, 10, 13)),
                    ..Default::default()
                })
                .unwrap();

            conn.create_sanction(&CreateSanction {
                id: Uuid::new_v4(),
                user_id,
                team_id,
                created_at: Some(NaiveDate::from_ymd(2019, 10, 5)),
                ..Default::default()
            })
            .unwrap();

            conn.create_sanction(&CreateSanction {
                id: Uuid::new_v4(),
                user_id,
                team_id,
                created_at: Some(NaiveDate::from_ymd(2019, 10, 25)),
                ..Default::default()
            })
            .unwrap();

            let sanctions: Vec<Sanction> = conn
                .get_sanctions(
                    team_id,
                    Some((
                        NaiveDate::from_ymd(2019, 10, 6),
                        NaiveDate::from_ymd(2019, 10, 20),
                    )),
                )
                .unwrap();

            assert_eq!(vec![sanction], sanctions);

            Ok(())
        })
    }

    #[test]
    fn test_create_sanction() {
        let conn = init_connection();

        conn.deref().test_transaction::<_, Error, _>(|| {
            let id = Uuid::new_v4();

            let team_id = conn.create_team(&Team::default()).unwrap().id;

            let user_id = conn
                .create_user(&User {
                    team_id,
                    ..Default::default()
                })
                .unwrap()
                .id;

            let sanction = conn
                .create_sanction(&CreateSanction {
                    id,
                    user_id,
                    team_id,
                    ..Default::default()
                })
                .unwrap();

            assert_eq!(sanction.id, id);
            assert_eq!(sanction.user_id, user_id);
            assert_eq!(sanction.team_id, team_id);

            Ok(())
        });
    }

    #[test]
    fn test_create_sanction_fails() {
        let conn = init_connection();

        conn.deref().test_transaction::<_, Error, _>(|| {
            let error = conn
                .create_sanction(&CreateSanction::default())
                .unwrap_err();

            assert_eq!(
                error,
                DbError::ForeignKeyViolation(String::from(
                    "The key team_id doesn\'t refer to anything"
                ))
            );

            Ok(())
        });

        conn.deref().test_transaction::<_, Error, _>(|| {
            let team_id = conn.create_team(&Team::default()).unwrap().id;

            let error = conn
                .create_sanction(&CreateSanction {
                    team_id,
                    ..Default::default()
                })
                .unwrap_err();

            assert_eq!(
                error,
                DbError::ForeignKeyViolation(String::from(
                    "The key user_id doesn\'t refer to anything"
                ))
            );

            Ok(())
        });
    }

    #[test]
    fn test_delete_sanction() {
        let conn = init_connection();

        conn.deref().test_transaction::<_, Error, _>(|| {
            let team_id = conn.create_team(&Team::default()).unwrap().id;

            let user_id = conn
                .create_user(&User {
                    team_id,
                    ..Default::default()
                })
                .unwrap()
                .id;

            let sanction = conn
                .create_sanction(&CreateSanction {
                    team_id,
                    user_id,
                    ..Default::default()
                })
                .unwrap();

            let sanction_deleted = conn.delete_sanction(team_id, sanction.id).unwrap();
            assert_eq!(sanction.id, sanction_deleted.id);

            let sanctions = conn.get_sanctions(team_id, None).unwrap();
            assert_eq!(sanctions.len(), 0);

            Ok(())
        });
    }

    #[test]
    fn test_delete_sanction_fails() {
        let conn = init_connection();

        let error = conn
            .delete_sanction(Uuid::new_v4(), Uuid::new_v4())
            .unwrap_err();

        assert_eq!(error, DbError::NotFound);
    }
}

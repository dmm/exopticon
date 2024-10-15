use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::Binary,
    AsExpression, FromSqlRow,
};
use std::fmt::{self, Display, Formatter};
use uuid;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    SqlType,
    FromSqlRow,
    AsExpression,
    Hash,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
#[diesel(sql_type = Binary)]
pub struct Uuid(pub uuid::Uuid);

impl Uuid {
    pub fn new_v4() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl From<Uuid> for uuid::Uuid {
    fn from(s: Uuid) -> Self {
        s.0
    }
}

impl From<uuid::Uuid> for Uuid {
    fn from(s: uuid::Uuid) -> Self {
        Uuid(s)
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<B: Backend> FromSql<Binary, B> for Uuid
where
    Vec<u8>: FromSql<Binary, B>,
{
    fn from_sql(bytes: <B as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <Vec<u8>>::from_sql(bytes)?;
        uuid::Uuid::from_slice(&value)
            .map(Uuid)
            .map_err(|e| e.into())
    }
}

impl<B: Backend> ToSql<Binary, B> for Uuid
where
    [u8]: ToSql<Binary, B>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, B>) -> serialize::Result {
        //        self.0.as_bytes().to_sql(out)
        <[u8; 16] as ToSql<Binary, B>>::to_sql(self.0.as_bytes(), out)
    }
}

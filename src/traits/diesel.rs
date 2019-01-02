//! Diesel's `Insertable` implementation for [`VoluntaryServitude`]
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
//!
//! Batch Insert:
//!
//! **Cargo.toml**
//!
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-traits" }
//! ```

use diesel_lib::{backend::*, insertable::*, query_builder::*, *};
use std::{marker::PhantomData, sync::Arc};
use {Iter, voluntary_servitude::Inner, VoluntaryServitude};

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<T, Table> UndecoratedInsertRecord<Table> for VoluntaryServitude<T> where
    T: UndecoratedInsertRecord<Table>
{}

#[allow(missing_debug_implementations)]
pub struct InnerBatchInsert<'a, T, Tab>(Arc<Inner<T>>, PhantomData<(Tab, &'a T)>);

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<'a, T, Tab> Insertable<Tab> for &'a VoluntaryServitude<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = InnerBatchInsert<'a, T, Tab>;

    fn values(self) -> Self::Values {
        InnerBatchInsert(self.inner(), PhantomData)
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<'a, T, Tab> Insertable<Tab> for &'a Iter<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = InnerBatchInsert<'a, T, Tab>;

    fn values(self) -> Self::Values {
        InnerBatchInsert(self.inner(), PhantomData)
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<'a, T, Tab, DB, Inner> QueryFragment<DB> for InnerBatchInsert<'a, T, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
    &'a T: QueryFragment<DB> + Insertable<Tab, Values = ValuesClause<Inner, Tab>>,
    ValuesClause<Inner, Tab>: QueryFragment<DB>,
    Inner: QueryFragment<DB>
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let mut value = self.0.first_node().map(|nn| unsafe { &*nn.as_ptr() });
        if let Some(v) = value {
            v.value().walk_ast(out.reborrow())?;
            value = v.next();
        }

        while let Some(v) = value {
            out.push_sql(", (");
            v.value().walk_ast(out.reborrow())?;
            out.push_sql(")");
            value = v.next();
        }
        Ok(())
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<T: QueryId> QueryId for VoluntaryServitude<T> {
    type QueryId = T::QueryId;
    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-traits")))]
impl<'a, T, Tab, DB> CanInsertInSingleQuery<DB> for InnerBatchInsert<'a, T, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

#[cfg(test)]
mod tests {
    #![allow(proc_macro_derive_resolution_fallback)]
    #![allow(unused_import_braces)]

    use diesel_lib::{insert_into, prelude::*};

    table! {
        derives (id) {
            id -> Int4,
            name -> VarChar,
        }
    }

    #[derive(Queryable, Insertable, Clone)]
    struct Derive {
        name: String
    }

    impl Derive {
        pub fn new<S: Into<String>>(s: S) -> Self {
            let name = s.into();
            Self { name }
        }
    }

    /*
    #[test]
    fn insert_query_sqlite() {
        let conn = SqliteConnection::establish(":memory:").unwrap();
        //embedded_migrations::run(&conn).unwrap();
        let vs = vs![Derive::new("Name1"), Derive::new("Name2"), Derive::new("Name3")];

        let _ = insert_into(derives::table).values(&vs).execute(&conn).unwrap();
        let queried: Vec<String> = derives::table.select(derives::name).load(&conn).unwrap();
        assert_eq!(vs.iter().map(|d| d.name.to_owned()).collect::<Vec<_>>(), queried);
    }
    */
    
    #[test]
    #[ignore]
    fn insert_query_postgres() {
        let conn = PgConnection::establish("127.0.0.1").unwrap();
        let vs = vs![Derive::new("Name1"), Derive::new("Name2"), Derive::new("Name3")];

        let _ = insert_into(derives::table).values(&vs).execute(&conn).unwrap();
        let queried: Vec<String> = derives::table.select(derives::name).load(&conn).unwrap();
        assert_eq!(vs.iter().map(|d| d.name.to_owned()).collect::<Vec<_>>(), queried);

        let _ = insert_into(derives::table).values(&vs.iter().cloned().collect::<Vec<_>>()).execute(&conn).unwrap();
        let queried: Vec<String> = derives::table.select(derives::name).load(&conn).unwrap();
        assert_eq!(vs.iter().map(|d| d.name.to_owned()).collect::<Vec<_>>(), queried);
    }
}

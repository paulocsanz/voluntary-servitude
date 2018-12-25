//! Diesel's trait implementations for [`VoluntaryServitude`]
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
//!
//! Batch Insert:
//!
//! **Cargo.toml**
//!
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-insertable" }
//! ```
//!
//! [`VoluntaryServitude`] as postgres `Array` + batch insert
//!
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-postgres" }
//! ```

#[cfg(feature = "diesel-postgres")]
use byteorder::NetworkEndian;
#[cfg(feature = "diesel-postgres")]
use diesel_lib::{deserialize::*, serialize::IsNull, serialize::*, sql_types::*};
#[cfg(feature = "diesel-postgres")]
use std::io::Write;

use diesel_lib::{backend::*, insertable::*, query_builder::*, *};
use std::marker::PhantomData;
use {Iter, VoluntaryServitude};

/// Copied from: https://github.com/diesel-rs/diesel/blob/36078014717d6c2fb0d03d2a10d19177c06ed86d/diesel/src/pg/types/array.rs#L22
#[cfg(feature = "diesel-postgres")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
impl<T, ST> FromSql<Array<ST>, pg::Pg> for VoluntaryServitude<T>
where
    T: FromSql<ST, pg::Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        use byteorder::ReadBytesExt;
        use VS;
        let mut bytes = not_none!(bytes);
        let num_dimensions = bytes.read_i32::<NetworkEndian>()?;
        let has_null = bytes.read_i32::<NetworkEndian>()? != 0;
        let _oid = bytes.read_i32::<NetworkEndian>()?;

        if num_dimensions == 0 {
            return Ok(VS::new());
        }

        let num_elements = bytes.read_i32::<NetworkEndian>()?;
        let _lower_bound = bytes.read_i32::<NetworkEndian>()?;

        if num_dimensions != 1 {
            return Err("multi-dimensional arrays are not supported".into());
        }

        (0..num_elements)
            .map(|_| {
                let elem_size = bytes.read_i32::<NetworkEndian>()?;
                if has_null && elem_size == -1 {
                    T::from_sql(None)
                } else {
                    let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
                    bytes = new_bytes;
                    T::from_sql(Some(elem_bytes))
                }
            }).collect()
    }
}

/// Copied from: https://github.com/diesel-rs/diesel/blob/36078014717d6c2fb0d03d2a10d19177c06ed86d/diesel/src/pg/types/array.rs#L83
#[cfg(feature = "diesel-postgres")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
impl<ST, T> ToSql<Array<ST>, pg::Pg> for VoluntaryServitude<T>
where
    pg::Pg: HasSqlType<ST>,
    T: ToSql<ST, pg::Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, pg::Pg>) -> serialize::Result {
        use byteorder::WriteBytesExt;
        let num_dimensions = 1;
        out.write_i32::<NetworkEndian>(num_dimensions)?;
        let flags = 0;
        out.write_i32::<NetworkEndian>(flags)?;
        let element_oid = pg::Pg::metadata(out.metadata_lookup()).oid;
        out.write_u32::<NetworkEndian>(element_oid)?;
        out.write_i32::<NetworkEndian>(self.len() as i32)?;
        let lower_bound = 1;
        out.write_i32::<NetworkEndian>(lower_bound)?;

        let mut buffer = out.with_buffer(Vec::with_capacity(self.len()));
        for elem in &mut self.iter() {
            let is_null = elem.to_sql(&mut buffer)?;
            if let IsNull::No = is_null {
                out.write_i32::<NetworkEndian>(buffer.len() as i32)?;
                out.write_all(&buffer)?;
                buffer.clear();
            } else {
                // https://github.com/postgres/postgres/blob/82f8107b92c9104ec9d9465f3f6a4c6dab4c124a/src/backend/utils/adt/arrayfuncs.c#L1461
                out.write_i32::<NetworkEndian>(-1)?;
            }
        }

        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel-postgres")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
impl<ST, T> ToSql<Nullable<Array<ST>>, pg::Pg> for VoluntaryServitude<T>
where
    VoluntaryServitude<T>: ToSql<Array<ST>, pg::Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, pg::Pg>) -> serialize::Result {
        ToSql::<Array<ST>, pg::Pg>::to_sql(self, out)
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-insertable")))]
impl<T, Table> UndecoratedInsertRecord<Table> for VoluntaryServitude<T> where
    [T]: UndecoratedInsertRecord<Table>
{}

#[allow(missing_debug_implementations)]
pub struct IterBatchInsert<I, Tab>(I, PhantomData<Tab>);

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-insertable")))]
impl<T, Tab> Insertable<Tab> for VoluntaryServitude<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = IterBatchInsert<Iter<T>, Tab>;

    fn values(self) -> Self::Values {
        IterBatchInsert(self.iter(), PhantomData)
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-insertable")))]
impl<T, Tab> Insertable<Tab> for Iter<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = IterBatchInsert<Iter<T>, Tab>;

    fn values(self) -> Self::Values {
        IterBatchInsert(self, PhantomData)
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-insertable")))]
impl<Tab, DB, Inner> QueryFragment<DB> for IterBatchInsert<Iter<Inner>, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
    ValuesClause<Inner, Tab>: QueryFragment<DB>,
    Inner: QueryFragment<DB> + Clone + Insertable<Tab>,
    Inner::Values: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let mut iter = self.0.clone();
        let values = &mut iter.cloned().map(Insertable::values);
        if let Some(value) = values.next() {
            value.walk_ast(out.reborrow())?;
        }

        for value in values {
            out.push_sql(", (");
            value.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-postgres")))]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "diesel-insertable")))]
impl<T, Tab, DB> CanInsertInSingleQuery<DB> for IterBatchInsert<Iter<T>, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

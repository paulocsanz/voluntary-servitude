//! Diesel's trait implementations for [`VoluntaryServitude`]
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
//!
//! Enable the feature - choose the backend(s) that are appropriate:
//!
//! **Cargo.toml**
//!
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-postgres" }
//! ```
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-mysql" }
//! ```
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "diesel-sqlite" }
//! ```
use std::{io::Write, error::Error, marker::PhantomData};
use diesel_lib::{*, deserialize::*, backend::*, serialize::*, query_builder::*, insertable::*, serialize::IsNull, sql_types::*};
use byteorder::NetworkEndian;
use {VoluntaryServitude, Iter};

#[cfg(any(feature = "diesel-sqlite", feature = "diesel-mysql"))]
impl<ST, DB> FromSql<ST, DB> for VoluntaryServitude<u8>
where
    DB: Backend,
    *const [u8]: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        use std::iter::FromIterator;
        let slice_ptr = <*const [u8] as FromSql<ST, DB>>::from_sql(bytes)?;
        // We know that the pointer impl will never return null
        let bytes = unsafe { &*slice_ptr };
        Ok(Self::from_iter(bytes))
    }
}

#[cfg(not(any(feature = "diesel-sqlite", feature = "diesel-mysql")))]
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

impl<DB: Backend> ToSql<Binary, DB> for VoluntaryServitude<u8> {
	fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
		for byte in &mut self.iter() {
			out.write_all(&[*byte])
				.map_err(|e| Box::new(e) as Box<Error + Send + Sync>)?;
		}
        Ok(IsNull::No)
	}
}


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

impl<ST, T> ToSql<Nullable<Array<ST>>, pg::Pg> for VoluntaryServitude<T>
where
    VoluntaryServitude<T>: ToSql<Array<ST>, pg::Pg>,
{
	fn to_sql<W: Write>(&self, out: &mut Output<W, pg::Pg>) -> serialize::Result {
		ToSql::<Array<ST>, pg::Pg>::to_sql(self, out)
	}
}

impl<T, Table> UndecoratedInsertRecord<Table> for VoluntaryServitude<T> where [T]: UndecoratedInsertRecord<Table> {}

#[allow(missing_debug_implementations)]
pub struct IterBatchInsert<I, Tab>(I, PhantomData<Tab>);

impl<T, Tab> Insertable<Tab> for Iter<T>
where
	T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
	type Values = IterBatchInsert<Iter<T>, Tab>;

	fn values(self) -> Self::Values {
        IterBatchInsert(self, PhantomData)
	}
}


impl<Tab, DB, Inner> QueryFragment<DB> for IterBatchInsert<Iter<Inner>, Tab>
where
	DB: Backend + SupportsDefaultKeyword,
	ValuesClause<Inner, Tab>: QueryFragment<DB>,
	Inner: QueryFragment<DB> + Clone + Insertable<Tab>,
    Inner::Values: QueryFragment<DB>
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

impl<T, Tab, DB> CanInsertInSingleQuery<DB> for IterBatchInsert<Iter<T>, Tab>
where
	DB: Backend + SupportsDefaultKeyword,
{
	fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
	}
}

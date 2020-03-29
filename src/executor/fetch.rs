use boolinator::Boolinator;
use nom_sql::{Column, ColumnSpecification, Table};
use std::fmt::Debug;

use crate::data::Row;
use crate::executor::Filter;
use crate::storage::Store;
use crate::Result;

pub fn fetch_columns<T: 'static + Debug>(
    storage: &dyn Store<T>,
    table: &Table,
) -> Result<Vec<Column>> {
    Ok(storage
        .get_schema(&table.name)?
        .fields
        .into_iter()
        .map(|ColumnSpecification { column, .. }| column)
        .collect::<Vec<Column>>())
}

pub fn fetch<'a, T: 'static + Debug>(
    storage: &dyn Store<T>,
    table: &'a Table,
    columns: &'a Vec<Column>,
    filter: Filter<'a, T>,
) -> Result<Box<dyn Iterator<Item = Result<(&'a Vec<Column>, T, Row)>> + 'a>> {
    let rows = storage.get_data(&table.name)?.filter_map(move |item| {
        item.map_or_else(
            |error| Some(Err(error)),
            |(key, row)| {
                filter
                    .check(table, columns, &row)
                    .map(|pass| pass.as_some((columns, key, row)))
                    .transpose()
            },
        )
    });

    Ok(Box::new(rows))
}

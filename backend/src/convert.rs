use chrono::prelude::*;
use grafana_plugin_sdk::{arrow2::array::Array, data, prelude::*};
use rust_decimal::prelude::*;
use tokio_postgres::{
    types::{FromSql, Type},
    Row,
};

fn load_field<'a, T>(rows: &'a [Row], column: usize, name: &str) -> data::Field
where
    T: FromSql<'a> + data::IntoFieldType,
    <T as data::IntoFieldType>::ElementType: data::FieldType,
    <<T as data::IntoFieldType>::ElementType as data::FieldType>::Array:
        Array + FromIterator<Option<<T as data::IntoFieldType>::ElementType>> + 'static,
{
    rows.iter()
        .map(|row| row.get::<_, T>(column))
        .into_field(name)
}

fn unsupported_type_field(n: usize, type_: &Type, name: &str) -> data::Field {
    std::iter::repeat_with(|| format!("unsupported column type {type_}"))
        .take(n)
        .into_field(name)
}

/// Convert some rows returned from Materialize to a Grafana Plugin SDK Frame.
pub fn rows_to_frame(rows: Vec<Row>) -> data::Frame {
    let mut frame = data::Frame::new("tail");
    if rows.is_empty() {
        return frame;
    }

    for (i, column) in rows[0].columns().iter().enumerate() {
        let name = column.name();
        let field = if name == "mz_timestamp" {
            rows.iter()
                .map(|row| {
                    row.get::<_, Decimal>(i)
                        .to_i64()
                        .map(|i| Utc.timestamp_millis(i))
                })
                .into_opt_field(name)
        } else {
            match column.type_() {
                &Type::CHAR => load_field::<i8>(&rows, i, name),
                &Type::INT2 => load_field::<i16>(&rows, i, name),
                &Type::INT4 => load_field::<i32>(&rows, i, name),
                &Type::INT8 => load_field::<i64>(&rows, i, name),
                &Type::FLOAT4 => load_field::<f32>(&rows, i, name),
                &Type::FLOAT8 => load_field::<f64>(&rows, i, name),
                &Type::OID => load_field::<u32>(&rows, i, name),
                &Type::TEXT | &Type::VARCHAR => load_field::<String>(&rows, i, name),
                &Type::JSON | &Type::JSONB => rows
                    .iter()
                    .map(|row| row.get::<_, serde_json::Value>(i).to_string())
                    .into_field(name),
                &Type::DATE => load_field::<NaiveDate>(&rows, i, name),
                &Type::TIMESTAMP => load_field::<NaiveDateTime>(&rows, i, name),
                &Type::TIMESTAMPTZ => load_field::<DateTime<Utc>>(&rows, i, name),
                other => unsupported_type_field(rows.len(), other, name),
            }
        };
        frame.add_field(field);
    }
    frame
}

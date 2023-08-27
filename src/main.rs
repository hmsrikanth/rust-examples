use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::{DataType as ArrowDataType, Field as ArrowField};
use datafusion::error::Result;
use datafusion::logical_expr::{create_udf, Volatility};
use datafusion::physical_expr::functions::make_scalar_function;
use datafusion::prelude::*;
use anyhow;
use itertools::Itertools;

trait ArrowDataTypeExt {
    fn list_of(item_type: ArrowDataType) -> Self;
}

impl ArrowDataTypeExt for ArrowDataType {
    fn list_of(item_type: ArrowDataType) -> Self {
        ArrowDataType::List(Arc::new(ArrowField::new("item", item_type, true)))
    }
}

#[allow(dead_code)]
fn utf8_chunk_udf(input: &[ArrayRef]) -> datafusion::error::Result<ArrayRef> {
    utf8_chunk(&input[0], 2).map_err(|e| e.into())
}

fn utf8_chunk(input: &ArrayRef, size: usize) -> Result<ArrayRef> {
    // TODO support different string compatible types, largeutf8, dictionary encoding
    let input = input
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("string array");

    let mut builder = ListBuilder::new(StringBuilder::new());
    input.into_iter().for_each(|s| {
        if let Some(s) = s {
            let value_builder = builder.values();
            s.chars().chunks(size).into_iter().for_each(|c| {
                value_builder.append_value(c.collect::<String>());
            });
            builder.append(true);
        } else {
            builder.append_null();
        }
    });

    Ok(Arc::new(builder.finish()))
}


#[tokio::test]
async fn test_string_to_array() -> anyhow::Result<()> {
    let udf = create_udf(
        "utf8_chunk",
        vec![ArrowDataType::Utf8],
        Arc::new(ArrowDataType::list_of(ArrowDataType::Utf8)),
        Volatility::Immutable,
        make_scalar_function(utf8_chunk_udf),
    );

    let ctx = SessionContext::new();
    let df = ctx.read_csv("data/input.csv",get_csv_option()).await?;
    println!("{}", df.schema());



    let df = df.select(vec![col("data"), udf.call(vec![col("data")]).alias("c_chunks")])?;
    let df = df.unnest_column("c_chunks")?;

    //let count = df.count().await?;
    //assert_eq!(count, 23);
    df.show().await?;
    Ok(())
}
fn get_csv_option<'a>() -> CsvReadOptions<'a> {
    let mut csv_opt = CsvReadOptions::new();
    csv_opt.has_header = true;
    csv_opt.delimiter = b',';
    csv_opt
}

fn main() {}
use std::sync::Arc;

use arrow_array::{Float64Array, Int32Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use lancedb::{arrow::IntoArrow, connect, Result};

#[tokio::main]
async fn main() -> Result<()> {

    let uri = std::env::var("LANCE_URI").expect("LANCE_URI is not set");
    let api_key = std::env::var("LANCE_API_KEY").expect("LANCE_API_KEY is not set");
    let region = std::env::var("LANCE_REGION").unwrap_or("us-east-1".to_string());
    let db = connect(&uri)
        .api_key(&api_key)
        .region(&region)
        .execute()
        .await?;
    println!("Connected to LanceDB");

    let tbl = db.open_table("test").execute().await?;

    let version = tbl.version().await?;

    let n_rows = tbl.count_rows(None).await?;

    Ok(())
}

fn make_data() -> impl IntoArrow {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int32, true),
        Field::new("text", DataType::Utf8, false),
        Field::new("price", DataType::Float64, false),
    ]);

    let id = Int32Array::from(vec![1]);
    let text = StringArray::from_iter_values(vec!["socks"]);
    let price = Float64Array::from(vec![10.0]);
    let schema = Arc::new(schema);
    let rb = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(id), Arc::new(text), Arc::new(price)],
    )
    .unwrap();
    Box::new(RecordBatchIterator::new(vec![Ok(rb)], schema))
}

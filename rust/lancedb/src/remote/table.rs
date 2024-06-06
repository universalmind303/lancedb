use std::sync::Arc;

use arrow_array::RecordBatchReader;
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion_physical_plan::ExecutionPlan;
use lance::{
    arrow::json::JsonSchema,
    dataset::{scanner::DatasetRecordBatchStream, ColumnAlteration, NewColumnTransform},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    connection::NoData,
    error::Result,
    index::{IndexBuilder, IndexConfig},
    query::{Query, QueryExecutionOptions, VectorQuery},
    table::{
        merge::MergeInsertBuilder, AddDataBuilder, NativeTable, OptimizeAction, OptimizeStats,
        TableDefinition, TableInternal, UpdateBuilder,
    },
    Table,
};

use super::client::RestfulLanceDbClient;

#[derive(Debug)]
pub struct RemoteTable {
    #[allow(dead_code)]
    client: RestfulLanceDbClient,
    name: String,
}

impl RemoteTable {
    pub fn new(client: RestfulLanceDbClient, name: String) -> Self {
        Self { client, name }
    }
}

impl From<RemoteTable> for Table {
    fn from(table: RemoteTable) -> Self {
        Table::new(Arc::new(table))
    }
}

impl std::fmt::Display for RemoteTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoteTable({})", self.name)
    }
}

impl RemoteTable {
    async fn get_table_info(&self) -> Result<GetTableInfoResponse> {
        let uri = format!("/v1/table/{}/describe/", self.name);

        let resp = self.client.post(&uri).send().await?;
        if !resp.status().is_success() {
            return Err(crate::Error::Runtime {
                message: resp.text().await?,
            });
        }
        Ok(resp.json::<GetTableInfoResponse>().await?)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GetTableInfoResponse {
    pub table: String,
    pub version: u64,
    pub schema: JsonSchema,
    pub stats: TableStats,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TableStats {
    pub num_deleted_rows: usize,
    pub num_fragments: usize,
}

#[async_trait]
impl TableInternal for RemoteTable {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_native(&self) -> Option<&NativeTable> {
        None
    }
    fn name(&self) -> &str {
        &self.name
    }

    async fn version(&self) -> Result<u64> {
        let tbl_info = self.get_table_info().await?;
        Ok(tbl_info.version)
    }
    async fn checkout(&self, _version: u64) -> Result<()> {
        todo!()
    }
    async fn checkout_latest(&self) -> Result<()> {
        todo!()
    }
    async fn restore(&self) -> Result<()> {
        todo!()
    }
    async fn schema(&self) -> Result<SchemaRef> {
        let tbl_info = self.get_table_info().await?;
        Ok(Arc::new(tbl_info.schema.try_into()?))
    }

    async fn count_rows(&self, filter: Option<String>) -> Result<usize> {
        #[derive(Serialize, Deserialize)]
        struct CountRowsRequest {
            predicate: Option<String>,
        }
        let req = CountRowsRequest { predicate: filter };

        let query = format!("/v1/table/{}/count_rows/", self.name);
        let req = self
            .client
            .post(&query)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&req).unwrap());
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(crate::Error::Runtime {
                message: resp.text().await?,
            });
        }
        Ok(resp.json().await?)
    }
    async fn add(
        &self,
        _add: AddDataBuilder<NoData>,
        _data: Box<dyn RecordBatchReader + Send>,
    ) -> Result<()> {
        todo!()
    }
    async fn create_plan(
        &self,
        _query: &VectorQuery,
        _options: QueryExecutionOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        unimplemented!()
    }
    async fn plain_query(
        &self,
        _query: &Query,
        _options: QueryExecutionOptions,
    ) -> Result<DatasetRecordBatchStream> {
        todo!()
    }
    async fn update(&self, _update: UpdateBuilder) -> Result<()> {
        todo!()
    }
    async fn delete(&self, _predicate: &str) -> Result<()> {
        todo!()
    }
    async fn create_index(&self, _index: IndexBuilder) -> Result<()> {
        todo!()
    }
    async fn merge_insert(
        &self,
        _params: MergeInsertBuilder,
        _new_data: Box<dyn RecordBatchReader + Send>,
    ) -> Result<()> {
        todo!()
    }
    async fn optimize(&self, _action: OptimizeAction) -> Result<OptimizeStats> {
        todo!()
    }
    async fn add_columns(
        &self,
        _transforms: NewColumnTransform,
        _read_columns: Option<Vec<String>>,
    ) -> Result<()> {
        todo!()
    }
    async fn alter_columns(&self, _alterations: &[ColumnAlteration]) -> Result<()> {
        todo!()
    }
    async fn drop_columns(&self, _columns: &[&str]) -> Result<()> {
        todo!()
    }
    async fn list_indices(&self) -> Result<Vec<IndexConfig>> {
        todo!()
    }
    async fn table_definition(&self) -> Result<TableDefinition> {
        todo!()
    }
}

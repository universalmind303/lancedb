// Copyright 2024 LanceDB Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(feature = "openai")]
pub mod openai;

use lance::arrow::RecordBatchExt;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use arrow_array::{Array, RecordBatch, RecordBatchReader};
use arrow_schema::{DataType, Field, SchemaBuilder};
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    table::{ColumnDefinition, ColumnKind, TableDefinition},
};

/// Trait for embedding functions
///
/// An embedding function is a function that is applied to a column of input data
/// to produce an "embedding" of that input.  This embedding is then stored in the
/// database alongside (or instead of) the original input.
///
/// An "embedding" is often a lower-dimensional representation of the input data.
/// For example, sentence-transformers can be used to embed sentences into a 768-dimensional
/// vector space.  This is useful for tasks like similarity search, where we want to find
/// similar sentences to a query sentence.
///
/// To use an embedding function you must first register it with the `EmbeddingsRegistry`.
/// Then you can define it on a column in the table schema. That embedding will then be used
/// to embed the data in that column.
pub trait EmbeddingFunction: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    /// The type of the input data
    fn source_type(&self) -> Cow<DataType>;
    /// The type of the output data
    /// This should match the output of the `embed` function
    fn dest_type(&self) -> Cow<DataType>;
    /// Embed the input
    fn embed(&self, source: Arc<dyn Array>) -> Result<Arc<dyn Array>>;
}

/// Defines an embedding from input data into a lower-dimensional space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingDefinition {
    /// The name of the column in the input data
    pub source_column: String,
    /// The name of the embedding column, if not specified
    /// it will be the source column with `_embedding` appended
    pub dest_column: Option<String>,
    /// The name of the embedding function to apply
    pub embedding_name: String,
}

impl EmbeddingDefinition {
    pub fn new<S: Into<String>>(source_column: S, embedding_name: S, dest: Option<S>) -> Self {
        Self {
            source_column: source_column.into(),
            dest_column: dest.map(|d| d.into()),
            embedding_name: embedding_name.into(),
        }
    }
    pub fn dest_column(&self) -> String {
        self.dest_column.clone()
            .unwrap_or_else(|| format!("{}_embedding", self.source_column))
    }
}

/// A registry of embedding functions
pub trait EmbeddingRegistry: Send + Sync + std::fmt::Debug {
    /// Return the names of all registered embedding functions
    fn functions(&self) -> HashSet<String>;
    /// Register a new [`EmbeddingFunction`]
    /// Returns an error if the function can not be registered
    fn register(&self, name: &str, function: Arc<dyn EmbeddingFunction>) -> Result<()>;
    /// Get an embedding function by name
    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddingFunction>>;
}

/// A [`EmbeddingRegistry`] that uses in-memory [`HashMap`]s
#[derive(Debug, Default, Clone)]
pub struct MemoryRegistry {
    functions: Arc<RwLock<HashMap<String, Arc<dyn EmbeddingFunction>>>>,
}

impl EmbeddingRegistry for MemoryRegistry {
    fn functions(&self) -> HashSet<String> {
        self.functions.read().unwrap().keys().cloned().collect()
    }
    fn register(&self, name: &str, function: Arc<dyn EmbeddingFunction>) -> Result<()> {
        self.functions
            .write()
            .unwrap()
            .insert(name.to_string(), function);

        Ok(())
    }

    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddingFunction>> {
        self.functions.read().unwrap().get(name).cloned()
    }
}

impl MemoryRegistry {
    /// Create a new `MemoryRegistry`
    pub fn new() -> Self {
        Self::default()
    }
}

/// A record batch reader that has embeddings applied to it
/// This is a wrapper around another record batch reader that applies an embedding function
/// when reading from the record batch
pub struct WithEmbeddings<R: RecordBatchReader> {
    inner: R,
    embedding_func: Arc<dyn EmbeddingFunction>,
    embedding_def: EmbeddingDefinition,
}

/// A record batch that might have embeddings applied to it.
pub enum MaybeEmbedded<R: RecordBatchReader> {
    /// The record batch reader has embeddings applied to it
    Yes(WithEmbeddings<R>),
    /// The record batch reader does not have embeddings applied to it
    /// The inner record batch reader is returned as-is
    No(R),
}

impl<R: RecordBatchReader> MaybeEmbedded<R> {
    /// Create a new RecordBatchReader with embeddings applied to it if the table definition
    /// specifies an embedding column and the registry contains an embedding function with that name
    /// Otherwise, this is a no-op and the inner RecordBatchReader is returned.
    pub fn try_new(
        inner: R,
        table_definition: TableDefinition,
        registry: Option<Arc<dyn EmbeddingRegistry>>,
    ) -> Result<Self> {
        if registry.is_none() {
            return Ok(Self::No(inner));
        }

        let embedding_def =
            table_definition
                .column_definitions
                .iter()
                .find_map(|cd| match &cd.kind {
                    ColumnKind::Embedding(embedding_def) => Some(embedding_def.clone()),
                    _ => None,
                });

        if let Some(embedding_def) = embedding_def {
            let embedding_func = registry
                .unwrap()
                .get(&embedding_def.embedding_name)
                .expect("Embedding function not found in registry")
                .clone();

            Ok(Self::Yes(WithEmbeddings {
                inner,
                embedding_func,
                embedding_def,
            }))
        } else {
            Ok(Self::No(inner))
        }
    }
}

impl<R: RecordBatchReader> WithEmbeddings<R> {
    pub fn new(
        inner: R,
        embedding_func: Arc<dyn EmbeddingFunction>,
        embedding_def: EmbeddingDefinition,
    ) -> Self {
        Self {
            inner,
            embedding_func,
            embedding_def,
        }
    }
}

impl<R: RecordBatchReader> WithEmbeddings<R> {
    fn dest_field(&self, nullable: bool) -> Field {
        let field_name = self.embedding_def.dest_column();

        Field::new(
            field_name,
            self.embedding_func.dest_type().into_owned(),
            nullable,
        )
    }

    pub fn table_definition(&self) -> TableDefinition {
        let base_schema = self.inner.schema();

        let src_column = base_schema
            .field_with_name(&self.embedding_def.source_column)
            .unwrap();

        let field = Arc::new(self.dest_field(src_column.is_nullable()));
        let column_definitions: Vec<_> = base_schema
            .fields()
            .iter()
            .map(|_| ColumnDefinition {
                kind: ColumnKind::Physical,
            })
            .chain(std::iter::once(ColumnDefinition {
                kind: ColumnKind::Embedding(self.embedding_def.clone()),
            }))
            .collect();

        let mut sb: SchemaBuilder = base_schema.as_ref().into();
        sb.push(field);
        let schema = Arc::new(sb.finish());
        TableDefinition {
            schema,
            column_definitions,
        }
    }
}

impl<R: RecordBatchReader> Iterator for MaybeEmbedded<R> {
    type Item = std::result::Result<RecordBatch, arrow_schema::ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Yes(inner) => inner.next(),
            Self::No(inner) => inner.next(),
        }
    }
}

impl<R: RecordBatchReader> RecordBatchReader for MaybeEmbedded<R> {
    fn schema(&self) -> Arc<arrow_schema::Schema> {
        match self {
            Self::Yes(inner) => inner.schema(),
            Self::No(inner) => inner.schema(),
        }
    }
}

impl<R: RecordBatchReader> Iterator for WithEmbeddings<R> {
    type Item = std::result::Result<RecordBatch, arrow_schema::ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        let batch = self.inner.next()?;
        if let Ok(mut batch) = batch {
            let schema = batch.schema();
            let is_nullable = schema
                .field_with_name(&self.embedding_def.source_column)
                .unwrap()
                .is_nullable();

            let dst_field = Arc::new(self.dest_field(is_nullable));

            let src_column = batch
                .column_by_name(&self.embedding_def.source_column)
                .unwrap();
            let embedding = self.embedding_func.embed(src_column.clone()).unwrap();
            batch = batch
                .try_with_column(dst_field.as_ref().clone(), embedding)
                .unwrap();
            Some(Ok(batch))
        } else {
            Some(Err(batch.unwrap_err()))
        }
    }
}

impl<R: RecordBatchReader> RecordBatchReader for WithEmbeddings<R> {
    fn schema(&self) -> Arc<arrow_schema::Schema> {
        self.table_definition().into_rich_schema()
    }
}

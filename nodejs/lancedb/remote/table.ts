// Copyright 2023 LanceDB Developers.
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

import { Table as ArrowTable } from "apache-arrow";

import { Data, IntoVector } from "../arrow";

import { CreateTableOptions } from "../connection";
import { IndexOptions } from "../indices";
import { MergeInsertBuilder } from "../merge";
import { VectorQuery } from "../query";
import { AddDataOptions, Table, UpdateOptions } from "../table";
import { RestfulLanceDBClient } from "./client";

export class RemoteTable extends Table {
  #client: RestfulLanceDBClient;
  #name: string;

  // Used in the display() method
  #dbName: string;

  get #tablePrefix() {
    return `/v1/table/${encodeURIComponent(this.#name)}/`;
  }

  get name(): string {
    return this.#name;
  }

  public constructor(
    client: RestfulLanceDBClient,
    tableName: string,
    dbName: string,
  ) {
    super();
    this.#client = client;
    this.#name = tableName;
    this.#dbName = dbName;
  }

  isOpen(): boolean {
    return !this.#client.isOpen();
  }

  close(): void {
    this.#client.close();
  }

  display(): string {
    return `RemoteTable(${this.#dbName}; ${this.#name})`;
  }

  async schema(): Promise<import("apache-arrow").Schema> {
    const resp = await this.#client.post(`${this.#tablePrefix}/describe/`);
    // TODO: parse this into a valid arrow schema
    return resp.schema;
  }
  async add(data: Data, options?: Partial<AddDataOptions>): Promise<void> {
    const { buf, mode } = await Table.parseTableData(
      data,
      options as CreateTableOptions,
      true,
    );
    await this.#client.post(`${this.#tablePrefix}/insert/`, buf, {
      params: {
        mode,
      },
      headers: {
        "Content-Type": "application/vnd.apache.arrow.stream",
      },
    });
  }

  async update(
    updates: Map<string, string> | Record<string, string>,
    options?: Partial<UpdateOptions>,
  ): Promise<void> {
    await this.#client.post(`${this.#tablePrefix}/update/`, {
      predicate: options?.where ?? null,
      updates: Object.entries(updates).map(([key, value]) => [key, value]),
    });
  }
  async countRows(filter?: unknown): Promise<number> {
    const payload = { predicate: filter };
    return await this.#client.post(`${this.#tablePrefix}/count_rows/`, payload);
  }

  async delete(predicate: unknown): Promise<void> {
    const payload = { predicate };
    await this.#client.post(`${this.#tablePrefix}/delete/`, payload);
  }
  async createIndex(
    column: string,
    options?: Partial<IndexOptions>,
  ): Promise<void> {
    if (options !== undefined) {
      console.warn("options are not yet supported on the LanceDB cloud");
    }
    const indexType = "vector";
    const metric = "L2";
    const data = {
      column,
      // biome-ignore lint/style/useNamingConvention: external API
      index_type: indexType,
      // biome-ignore lint/style/useNamingConvention: external API
      metric_type: metric,
    };
    await this.#client.post(`${this.#tablePrefix}/create_index`, data);
  }
  query(): import("..").Query {
    throw new Error("query() is not yet supported on the LanceDB cloud");
  }
  search(query: IntoVector): VectorQuery;
  search(query: string): Promise<VectorQuery>;
  search(_query: string | IntoVector): VectorQuery | Promise<VectorQuery> {
    throw new Error("search() is not yet supported on the LanceDB cloud");
  }
  vectorSearch(_vector: unknown): import("..").VectorQuery {
    throw new Error("vectorSearch() is not yet supported on the LanceDB cloud");
  }
  addColumns(_newColumnTransforms: unknown): Promise<void> {
    throw new Error("addColumns() is not yet supported on the LanceDB cloud");
  }
  alterColumns(_columnAlterations: unknown): Promise<void> {
    throw new Error("alterColumns() is not yet supported on the LanceDB cloud");
  }
  dropColumns(_columnNames: unknown): Promise<void> {
    throw new Error("dropColumns() is not yet supported on the LanceDB cloud");
  }
  async version(): Promise<number> {
    const resp = await this.#client.post(`${this.#tablePrefix}/describe/`);
    return resp.version;
  }
  checkout(_version: unknown): Promise<void> {
    throw new Error("checkout() is not yet supported on the LanceDB cloud");
  }
  checkoutLatest(): Promise<void> {
    throw new Error(
      "checkoutLatest() is not yet supported on the LanceDB cloud",
    );
  }
  restore(): Promise<void> {
    throw new Error("restore() is not yet supported on the LanceDB cloud");
  }
  optimize(_options?: unknown): Promise<import("../native").OptimizeStats> {
    throw new Error("optimize() is not yet supported on the LanceDB cloud");
  }
  async listIndices(): Promise<import("../native").IndexConfig[]> {
    return await this.#client.post(`${this.#tablePrefix}/index/list/`);
  }
  toArrow(): Promise<ArrowTable> {
    throw new Error("toArrow() is not yet supported on the LanceDB cloud");
  }
  mergeInsert(_on: string | string[]): MergeInsertBuilder {
    throw new Error("mergeInsert() is not yet supported on the LanceDB cloud");
  }
}

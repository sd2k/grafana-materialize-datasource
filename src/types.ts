import { DataQuery, DataSourceJsonData } from '@grafana/data';

// Regular datasource queries.

/// Types of operation available.
///
/// Currently the backend only supports running the TAIL statement.
export enum MaterializeOperation {
  /// Tail a relation or the output of a select statement using the TAIL statement.
  /// See https://materialize.com/docs/sql/tail/ for details.
  Tail = 'tail',
}

export enum MaterializeTarget {
  /// An existing relation (source, table or view).
  Relation = 'relation',
  /// A SELECT statement.
  SelectStatement = 'select',
}

interface PartialQuery extends DataQuery {
  /// The type of operation to request from the backend.
  operation: MaterializeOperation;
}

/// A request to tail an existing relation.
export interface TailRelation extends PartialQuery {
  /// The operation to perform - here, TAIL.
  operation: MaterializeOperation.Tail;
  /// The type of target to tail - here, a relation.
  target: MaterializeTarget.Relation;
  /// The name of the relation to tail.
  name?: string;
}

/// A request to tail the output of a SELECT statement.
export interface TailStatement extends PartialQuery {
  /// The operation to perform - here, TAIL.
  operation: MaterializeOperation.Tail;
  /// The type of target to tail - here, a select statement.
  target: MaterializeTarget.SelectStatement;
  /// The SELECT statement to tail.
  statement?: string;
}

/// A query to send to the plugin backend.
export type MaterializeQuery = TailRelation | TailStatement;

export const defaultQuery: Partial<MaterializeQuery> = {
  operation: MaterializeOperation.Tail,
};

// Variable queries, used when populating variable values in dashboards.

/// The path of the variable query we're performing.
export enum VariableQueryPathName {
  /// Query for available relations.
  Relations = 'relations',
}

export interface VariableQuery {
  path?: VariableQueryPathName;
}

/**
 * These are options configured for each DataSource instance.
 */
export interface DataSourceOptions extends DataSourceJsonData {
  host?: string;
  port?: number;
  username?: string;
}

/**
 * These are secure options configured for each DataSource instance.
 */
export interface DataSourceSecureOptions extends DataSourceJsonData {
  password?: string;
}

import { DataQuery, DataSourceJsonData } from '@grafana/data';

// Regular datasource queries.

export enum MaterializeOperation {
  Tail = 'tail',
}

export enum MaterializeTarget {
  Relation = 'relation',
  SelectStatement = 'select',
}

interface PartialQuery extends DataQuery {
  operation: MaterializeOperation;
}

export interface TailRelation extends PartialQuery {
  operation: MaterializeOperation.Tail;
  target: MaterializeTarget.Relation;
  name?: string;
}

export interface TailStatement extends PartialQuery {
  operation: MaterializeOperation.Tail;
  target: MaterializeTarget.SelectStatement;
  selectStatement?: string;
}

export type MaterializeQuery = TailRelation | TailStatement;

export const defaultQuery: Partial<MaterializeQuery> = {
  operation: MaterializeOperation.Tail,
};

// Variable queries.

export enum VariableQueryPathName {
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

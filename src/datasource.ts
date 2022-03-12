import { DataSourceInstanceSettings, MetricFindValue } from '@grafana/data';
import { DataSourceWithBackend, StreamingFrameOptions } from '@grafana/runtime';
import { DataSourceOptions, MaterializeQuery, VariableQueryPathName, VariableQuery } from './types';

export class DataSource extends DataSourceWithBackend<MaterializeQuery, DataSourceOptions> {
  constructor(instanceSettings: DataSourceInstanceSettings<DataSourceOptions>) {
    super(instanceSettings);
  }

  streamOptionsProvider = (): Partial<StreamingFrameOptions> => ({ maxLength: 10000 });

  async metricFindQuery(query: VariableQuery): Promise<MetricFindValue[]> {
    if (query.path === VariableQueryPathName.Relations) {
      const url = 'relations';
      const tasks = await this.getResource(url);
      return tasks.map((text: string) => ({ text }));
    }
    return [];
  }
}

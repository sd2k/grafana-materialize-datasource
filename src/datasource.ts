import { DataQueryRequest, DataSourceInstanceSettings, MetricFindValue } from '@grafana/data';
import { DataSourceWithBackend, StreamingFrameOptions } from '@grafana/runtime';
import { DataSourceOptions, MaterializeQuery, VariableQueryPathName, VariableQuery } from './types';

export class DataSource extends DataSourceWithBackend<MaterializeQuery, DataSourceOptions> {
  constructor(instanceSettings: DataSourceInstanceSettings<DataSourceOptions>) {
    super(instanceSettings);
  }

  // applyTemplateVariables(query: MaterializeQuery): Record<string, any> {
  //   return query;
  // }

  streamOptionsProvider = (request: DataQueryRequest<MaterializeQuery>): Partial<StreamingFrameOptions> => {
    // const shouldOverwrite = request.targets.some((target) => target.path === ConsolePathName.TaskHistogram);
    return {
      maxLength: 10000 /*, action: shouldOverwrite ? StreamingFrameAction.Replace : StreamingFrameAction.Append */,
    };
  };

  async metricFindQuery(query: VariableQuery): Promise<MetricFindValue[]> {
    if (query.path === VariableQueryPathName.Relations) {
      const url = '/relations';
      let tasks = await this.getResource(url);
      return tasks.map((text: string) => ({ text }));
    }
    return [];
  }
}

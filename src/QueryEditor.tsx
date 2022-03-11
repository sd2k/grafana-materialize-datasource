import { defaults } from 'lodash';

import React, { useEffect, useState } from 'react';
import { QueryEditorProps, SelectableValue } from '@grafana/data';
import { Input, Select } from '@grafana/ui';

import { DataSource } from './datasource';
import { defaultQuery, DataSourceOptions, MaterializeQuery, MaterializeTarget } from './types';

type Props = QueryEditorProps<DataSource, MaterializeQuery, DataSourceOptions>;

const targetOptions = [
  { label: 'Relation', value: MaterializeTarget.Relation, description: 'Tail the output of a source, table or view.' },
  {
    label: 'Select statement',
    value: MaterializeTarget.SelectStatement,
    description: 'Tail the results of a select statement.',
  },
];

export const QueryEditor = ({ datasource, onChange, onRunQuery, query }: Props): JSX.Element => {
  defaults(query, defaultQuery);
  const { target } = query;

  const onTargetChange = (event: SelectableValue<MaterializeTarget>) => {
    onChange({ ...query, target: event.value ?? MaterializeTarget.Relation });
  };
  const onRelationChange = (event: SelectableValue<string>) => {
    if (target === MaterializeTarget.Relation) {
      onChange({ ...query, name: event.value });
    }
  };
  const onSelectStatementChange = (event: SelectableValue<string>) => {
    if (target === MaterializeTarget.SelectStatement) {
      onChange({ ...query, statement: event.target.value });
    }
  };

  const [relations, setRelations] = useState<SelectableValue[]>([]);

  useEffect(() => {
    if (target === MaterializeTarget.Relation) {
      datasource.getResource('relations').then((options: string[]) => {
        setRelations(options.map((value) => ({ label: value, value })));
      });
    }
  }, [datasource, target]);

  return (
    <div className="gf-form">
      <Select menuShouldPortal options={targetOptions} value={target} onChange={onTargetChange} />
      {target === MaterializeTarget.Relation ? (
        <Select
          menuShouldPortal
          options={relations}
          value={query.name}
          onChange={onRelationChange}
          onBlur={onRunQuery}
        />
      ) : null}
      {target === MaterializeTarget.SelectStatement ? (
        <Input value={query.statement} onChange={onSelectStatementChange} onBlur={onRunQuery} />
      ) : null}
    </div>
  );
};

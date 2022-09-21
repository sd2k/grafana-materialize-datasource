import React, { useCallback } from 'react';
import { DataSourcePluginOptionsEditorProps, DataSourceSettings } from '@grafana/data';
import { DataSourceOptions, DataSourceSecureOptions } from './types';
import { FieldSet, Form, InlineField, Input, SecretInput } from '@grafana/ui';

interface Props extends DataSourcePluginOptionsEditorProps<DataSourceOptions, DataSourceSecureOptions> { }

export const ConfigEditor = ({ options, onOptionsChange }: Props): JSX.Element => {
  const onSettingsChange = useCallback(
    (change: Partial<DataSourceSettings<DataSourceOptions, DataSourceSecureOptions>>) => {
      onOptionsChange({
        ...options,
        ...change,
      });
    },
    [options, onOptionsChange]
  );
  return (
    <div className="gf-form-group">
      <Form onSubmit={onOptionsChange}>
        {() => (
          <>
            <FieldSet label="Connection">
              <InlineField label="Host" labelWidth={20}>
                <Input
                  value={options.jsonData.host}
                  onChange={(event) =>
                    onSettingsChange({ jsonData: { ...options.jsonData, host: event.currentTarget.value } })
                  }
                />
              </InlineField>

              <InlineField label="Port" labelWidth={20}>
                <Input
                  type="number"
                  value={options.jsonData.port}
                  placeholder="6875"
                  onChange={(event) =>
                    onSettingsChange({
                      jsonData: { ...options.jsonData, port: parseInt(event.currentTarget.value, 10) },
                    })
                  }
                />
              </InlineField>

              <InlineField label="Username" labelWidth={20}>
                <Input
                  value={options.jsonData.username}
                  placeholder="materialize"
                  onChange={(event) =>
                    onSettingsChange({ jsonData: { ...options.jsonData, username: event.currentTarget.value } })
                  }
                />
              </InlineField>

              <InlineField label="Password" labelWidth={20}>
                <SecretInput
                  isConfigured={!!options.secureJsonData?.password ?? false}
                  onReset={() => onSettingsChange({ secureJsonData: { ...options.secureJsonData, password: undefined } })}
                  value={options.secureJsonData?.password}
                  onBlur={(event) =>
                    onSettingsChange({ secureJsonData: { ...options.secureJsonData, password: event.currentTarget.value } })
                  }
                />
              </InlineField>
            </FieldSet>
          </>
        )}
      </Form>
    </div>
  );
};

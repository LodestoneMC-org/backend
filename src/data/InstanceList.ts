import { useQuery, QueryClient } from '@tanstack/react-query';
import axios, { AxiosError } from 'axios';
import { InstanceInfo } from 'bindings/InstanceInfo';
import { useContext } from 'react';
import { LodestoneContext } from './LodestoneContext';

export const updateInstance = (
  uuid: string,
  queryClient: QueryClient,
  updater: (oldInfo: InstanceInfo) => InstanceInfo
) => {
  queryClient.setQueriesData(
    ['instances', 'list'],
    (oldData: { [uuid: string]: InstanceInfo } | undefined) => {
      if (!oldData) return oldData;
      return {
        ...oldData,
        [uuid]: updater(oldData[uuid]),
      };
    }
  );
};

export const deleteInstance = (
  uuid: string,
  queryClient: QueryClient
) => {
  queryClient.setQueriesData(
    ['instances', 'list'],
    (oldData: { [uuid: string]: InstanceInfo } | undefined) => {
      if (!oldData) return oldData;
      const newData = { ...oldData };
      delete newData[uuid];
      return newData;
    }
  );
};

export const useInstanceList = () =>
  useQuery<{ [uuid: string]: InstanceInfo }, AxiosError>(
    ['instances', 'list'],
    () => {
      return axios
        .get<InstanceInfo[]>('/instance/list')
        .then((response) => {
          if (response.status !== 200) {
            throw new Error('Invalid status code');
          }
          if (!response.data) {
            throw new Error('Invalid response');
          }
          return response.data.reduce(
            (acc, instance) => ({
              ...acc,
              [instance.uuid]: instance,
            }),
            {}
          );
        });
    },
    {
      enabled: useContext(LodestoneContext).isReady,
    }
  );
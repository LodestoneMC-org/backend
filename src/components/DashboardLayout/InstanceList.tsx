import { Key, useEffect, useState } from 'react';
import InstanceCard from 'components/InstanceCard';
import { useAppDispatch, useAppSelector } from 'utils/hooks';
import { fetchInstanceList, selectInstanceList } from 'data/InstanceList';
import { selectClientInfo } from 'data/ClientInfo';

export default function InstanceList() {
  const { instances, loading, error } = useAppSelector(selectInstanceList);
  const dispatch = useAppDispatch();
  const clientInfo = useAppSelector(selectClientInfo);

  useEffect(() => {
    console.log("fetching instance list");
    if (clientInfo.loading) return;
    console.log('fetching instance list');
    dispatch(fetchInstanceList(clientInfo));
  }, [dispatch, clientInfo]);

  // TODO: nicer looking loading and error indicators
  if (loading) {
    return <div>Loading...</div>;
  }
  if (error) {
    return <div>Error: {error}</div>;
  }
  if (!instances) {
    return <div>No instances found</div>;
  }

  return (
    <div className="flex flex-col overflow-y-auto h-fit gap-y-4 gap grow child:w-full">
      {instances &&
        Object.values(instances).map((instance) => (
          <InstanceCard key={instance.id} {...instance} />
        ))}
    </div>
  );
}
import axios from 'axios';
import DashboardCard from 'components/DashboardCard';
import DashboardLayout from 'components/DashboardLayout';
import PerformanceGraph from 'components/Graphs/PerformanceGraph';
import { useRouter } from 'next/router';
import { ReactElement, ReactNode, useEffect } from 'react';
import { round } from 'utils/util';
import { NextPageWithLayout } from './_app';

type CpuUsageReply = {
  cpu_speed: number;
  cpu_load: number;
};

type RamUsageReply = {
  total: number;
  free: number;
};

const bytesInGigabyte = 1073741824;

const getCpuUsage = async (): Promise<[number, number]> => {
  return await axios.get<CpuUsageReply>('/system/cpu').then((res) => {
    return [round(res.data.cpu_load, 1), 100];
  });
};

const getRamUsage = async (): Promise<[number, number]> => {
  return await axios.get<RamUsageReply>('/system/ram').then((res) => {
    return [
      round((res.data.total - res.data.free) / bytesInGigabyte, 1),
      round(res.data.total / bytesInGigabyte, 1),
    ];
  });
};

const Home: NextPageWithLayout = () => {
  const router = useRouter();
  // get rid of uuid from query
  const { uuid } = router.query;

  useEffect(() => {
    if (uuid) {
      router.push({
        pathname: `/dashboard`,
        query: {
          ...router.query,
        },
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [uuid]);

  return (
    <div className="relative h-full w-full overflow-y-auto bg-gray-800 px-12 pt-6 pb-10">
      <h1 className="font-heading text-2xlarge font-semibold tracking-tight text-gray-300">
        Home
      </h1>
      <p>Display some general information here maybe</p>
      <DashboardCard>
        <h1 className="text-medium font-bold"> Performance </h1>
        <div className="mb-10 grid grid-cols-1 gap-10 lg:grid-cols-2">
          <div>
            <PerformanceGraph
              title="CPU Usage"
              color="#62DD76"
              backgroundColor="#61AE3240"
              getter={getCpuUsage}
              unit="%"
            />
          </div>
          <div>
            <PerformanceGraph
              title="Memory Usage"
              color="#62DD76"
              backgroundColor="#61AE3240"
              getter={getRamUsage}
              unit="GiB"
            />
          </div>
        </div>
      </DashboardCard>
    </div>
  );
};

Home.getLayout = (page: ReactElement): ReactNode => (
  <DashboardLayout>{page}</DashboardLayout>
);

export default Home;
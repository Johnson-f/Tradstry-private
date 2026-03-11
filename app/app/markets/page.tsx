"use client";

import { ChartsCard } from '@/components/markets/charts-card';
import { MarketSummary } from '@/components/markets/market-summary';
import { RecentDevelopments } from '@/components/markets/recent-development';
import { Standouts } from '@/components/markets/standout';
import { Movers } from '@/components/markets/movers';
import { MarketSearch } from '@/components/markets/market-search';
import { AppPageHeader } from '@/components/app-page-header';

export default function MarketsPage() {
  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="Markets" actions={<MarketSearch />} />
      
      {/* Main content - Scrollable area with native overflow */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="p-8">
            <div className="grid grid-cols-1 lg:grid-cols-[1fr_400px] gap-8">
              {/* Left column - Main content */}
              <div className="space-y-8 min-w-0">
                <ChartsCard />
                <MarketSummary />
                <RecentDevelopments />
                <Standouts />
              </div>

              {/* Right column - Movers */}
              <div className="lg:sticky lg:top-8 h-fit">
                <Movers />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
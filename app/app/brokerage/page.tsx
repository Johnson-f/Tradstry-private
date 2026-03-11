"use client";

import { MergeTrades } from '@/components/brokerage/merge-trades';
import { ScrollArea } from '@/components/ui/scroll-area';
import { AppPageHeader } from '@/components/app-page-header';

export default function MergeTradesPage() {
  return (
    <div className="h-screen flex flex-col bg-background">
       <AppPageHeader title="Brokerage" />

      {/* Main content - Scrollable area with shadcn ScrollArea */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-8">
            <MergeTrades />
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
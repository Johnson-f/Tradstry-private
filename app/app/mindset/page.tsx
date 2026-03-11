"use client";

import { AppPageHeader } from '@/components/app-page-header';

export default function MindsetPage() {

  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="Mindset" />
      
      {/* Main content - Scrollable area with native overflow */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="p-8">
            <div>COMING SOON</div>
          </div>
        </div>
      </div>
    </div>
  );
}
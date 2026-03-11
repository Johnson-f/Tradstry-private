"use client";

import { AppPageHeader } from "@/components/app-page-header";
import { VideosList } from "@/components/education";

export default function EducationPage() {
  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="Education" />

      {/* Main content - Scrollable area */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="p-8 space-y-8">
            <section className="space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-xl font-semibold">Featured Videos</h2>
                  <p className="text-sm text-muted-foreground">
                    Curated talks and lessons from top trading educators.
                  </p>
                </div>
              </div>
              <VideosList />
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}

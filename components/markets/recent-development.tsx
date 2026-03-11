'use client';

import React, { useMemo, useRef } from 'react';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { useNews } from '@/lib/hooks/use-market-data-service';
import { formatDistanceToNow, parseISO } from 'date-fns';
import { cn } from '@/lib/utils';
import { ChevronRight } from 'lucide-react';

const NEWS_PER_SLIDE = 3;

// Helper function to get source icon initials
function getSourceIcon(source: string | null | undefined): string {
  if (!source) return '??';
  const words = source.split(' ');
  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  return source.substring(0, 2).toUpperCase();
}

// Helper function to get source color based on source name
function getSourceColor(source: string | null | undefined): string {
  if (!source) return 'bg-gray-500';
  
  const colors = [
    'bg-purple-500',
    'bg-blue-500',
    'bg-green-500',
    'bg-yellow-500',
    'bg-red-500',
    'bg-indigo-500',
    'bg-pink-500',
  ];
  
  const hash = source.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
}

// Generate placeholder summary from title (since NewsItem doesn't have summary)
function generateSummary(title: string): string {
  // This is a placeholder - in a real app, the API would provide summaries
  // For now, we'll create a generic summary based on the title
  const summaries = [
    `Market analysis reveals significant movements in ${title.toLowerCase()}. Investors are closely monitoring the developments as volatility continues to impact trading strategies.`,
    `Recent developments in ${title.toLowerCase()} have caught the attention of analysts. The market response suggests continued uncertainty in the current economic climate.`,
    `Breaking news regarding ${title.toLowerCase()} indicates potential shifts in market sentiment. Traders are adjusting their positions accordingly.`,
  ];
  
  const hash = title.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return summaries[hash % summaries.length];
}

export function RecentDevelopments() {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const { news, isLoading, error } = useNews({ limit: 10 });

  // Calculate last updated time
  const lastUpdated = useMemo(() => {
    if (!news || !Array.isArray(news) || news.length === 0) return '';
    
    const times = news
      .map(item => {
        if (item.publish_time) {
          return new Date(item.publish_time * 1000);
        }
        if (item.publish_time_formatted) {
        try {
            const date = parseISO(item.publish_time_formatted);
          return isNaN(date.getTime()) ? null : date;
        } catch {
          return null;
        }
        }
        return null;
      })
      .filter((date): date is Date => date !== null);
    
    if (times.length === 0) return '';
    
    const latestTime = new Date(Math.max(...times.map(d => d.getTime())));
    return formatDistanceToNow(latestTime, { addSuffix: true });
  }, [news]);

  // Handle scroll to next set of cards
  const handleScrollNext = () => {
    if (scrollContainerRef.current) {
      const scrollAmount = scrollContainerRef.current.offsetWidth * 0.8;
      scrollContainerRef.current.scrollBy({
        left: scrollAmount,
        behavior: 'smooth',
      });
    }
  };

  // Check if there are more items to scroll
  const canScroll = useMemo(() => {
    if (!scrollContainerRef.current || !news || !Array.isArray(news)) return false;
    const container = scrollContainerRef.current;
    return container.scrollWidth > container.clientWidth;
  }, [news]);

  if (error) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-2xl font-bold text-foreground">Recent Developments</h2>
        </div>
        <Card>
          <CardContent className="pt-6">
            <p className="text-muted-foreground">Failed to load recent developments</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-foreground">Recent Developments</h2>
        {lastUpdated && (
          <p className="text-sm text-muted-foreground">Updated {lastUpdated}</p>
        )}
      </div>

      {/* News Cards Container */}
      <div className="relative">
        {isLoading ? (
          <div className="flex gap-4 overflow-hidden">
            {[1, 2, 3].map((i) => (
              <Card key={i} className="w-[320px] flex-shrink-0 bg-card">
                <CardHeader className="pb-2">
                  <div className="flex items-center gap-2 mb-2">
                    <Skeleton className="h-4 w-4 rounded-full" />
                    <Skeleton className="h-3 w-20" />
                  </div>
                  <Skeleton className="h-5 w-3/4" />
                </CardHeader>
                <CardContent className="pt-0">
                  <Skeleton className="h-3 w-full mb-1" />
                  <Skeleton className="h-3 w-11/12" />
                </CardContent>
              </Card>
            ))}
          </div>
        ) : !news || !Array.isArray(news) || news.length === 0 ? (
          <Card>
            <CardContent className="pt-6">
              <p className="text-muted-foreground">No recent developments available</p>
            </CardContent>
          </Card>
        ) : (
          <>
            <div
              ref={scrollContainerRef}
              className="flex gap-4 overflow-x-auto pb-2 pr-12 scrollbar-none"
              style={{
                scrollbarWidth: 'none',
                msOverflowStyle: 'none',
              }}
            >
              {news.map((item, index) => (
                <Card
                  key={`${item.link}-${index}`}
                  className="w-[320px] flex-shrink-0 bg-card"
                >
                  <CardHeader className="pb-2">
                    <div className="flex items-center gap-2 mb-2">
                      {/* Source Icons - overlapping circles */}
                      <div className="flex -space-x-2">
                        {/* Show 2-3 source icons */}
                        {[0, 1, 2].map((i) => {
                          const sourceName = item.publisher || 'Unknown';
                          return (
                            <div
                              key={i}
                              className={cn(
                                'h-5 w-5 rounded-full flex items-center justify-center text-[9px] text-white font-bold border-2 border-card',
                                getSourceColor(sourceName + i)
                              )}
                              style={{ zIndex: 3 - i }}
                            >
                              {getSourceIcon(sourceName)}
                            </div>
                          );
                        })}
                      </div>
                      {/* Timestamp */}
                      <p className="text-xs text-muted-foreground">
                        {(() => {
                          if (item.publish_time) {
                            return formatDistanceToNow(new Date(item.publish_time * 1000), { addSuffix: true });
                          }
                          if (item.publish_time_formatted) {
                          try {
                              const date = parseISO(item.publish_time_formatted);
                            if (!isNaN(date.getTime())) {
                              return formatDistanceToNow(date, { addSuffix: true });
                              }
                              return item.publish_time_formatted;
                            } catch {
                              return item.publish_time_formatted;
                            }
                          }
                          return 'recently';
                        })()}
                      </p>
                    </div>
                    {/* Title */}
                    <h3 className="text-base font-semibold text-foreground leading-tight line-clamp-2">
                      {item.title}
                    </h3>
                  </CardHeader>
                  <CardContent className="pt-0">
                    {/* Summary/Content */}
                    <p className="text-xs text-muted-foreground leading-relaxed line-clamp-3">
                      {generateSummary(item.title)}
                    </p>
                  </CardContent>
                </Card>
              ))}
            </div>

            {/* Right Arrow for scrolling */}
            {canScroll && Array.isArray(news) && news.length > NEWS_PER_SLIDE && (
              <button
                onClick={handleScrollNext}
                className="absolute right-0 top-1/2 -translate-y-1/2 bg-background/80 backdrop-blur-sm p-2 rounded-full cursor-pointer shadow-lg hover:bg-background/90 transition-colors z-10"
                aria-label="Scroll to next"
              >
                <ChevronRight className="h-6 w-6 text-foreground" />
              </button>
            )}
          </>
        )}
      </div>

      {/* Hide scrollbar styles */}
      <style dangerouslySetInnerHTML={{
        __html: `
          .scrollbar-none::-webkit-scrollbar {
            display: none;
          }
        `
      }} />
    </div>
  );
}

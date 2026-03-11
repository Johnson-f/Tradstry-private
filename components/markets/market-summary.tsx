'use client';

import React, { useMemo, useState } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination';
import { useNews } from '@/lib/hooks/use-market-data-service';
import { formatDistanceToNow, parseISO } from 'date-fns';
import { cn } from '@/lib/utils';

const NEWS_PER_PAGE = 5;

function formatTimeAgo(timeString: string): string {
  try {
    // Try parsing as ISO string first
    const date = parseISO(timeString);
    if (!isNaN(date.getTime())) {
      return formatDistanceToNow(date, { addSuffix: true });
    }
    
    // Try parsing as relative time string (e.g., "2 hours ago")
    const relativeMatch = timeString.match(/(\d+)\s*(minutes?|hours?|days?)\s*ago/i);
    if (relativeMatch) {
      const amount = parseInt(relativeMatch[1]);
      const unit = relativeMatch[2].toLowerCase();
      
      if (unit.includes('minute')) {
        return `${amount} minute${amount !== 1 ? 's' : ''} ago`;
      } else if (unit.includes('hour')) {
        return `${amount} hour${amount !== 1 ? 's' : ''} ago`;
      } else if (unit.includes('day')) {
        return `${amount} day${amount !== 1 ? 's' : ''} ago`;
      }
    }
    
    return timeString;
  } catch {
    return timeString;
  }
}

function getLatestUpdateTime(news: Array<{ publish_time?: number | null; publish_time_formatted?: string | null }>): string {
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
  return formatTimeAgo(latestTime.toISOString());
}

export function MarketSummary() {
  const [currentPage, setCurrentPage] = useState(1);
  const { news: allNews, isLoading, error } = useNews({ limit: 50 });

  // Calculate pagination
  const totalPages = useMemo(() => {
    if (!allNews || !Array.isArray(allNews) || allNews.length === 0) return 1;
    return Math.ceil(allNews.length / NEWS_PER_PAGE);
  }, [allNews]);

  const paginatedNews = useMemo(() => {
    if (!allNews || !Array.isArray(allNews) || allNews.length === 0) return [];
    const startIndex = (currentPage - 1) * NEWS_PER_PAGE;
    const endIndex = startIndex + NEWS_PER_PAGE;
    return allNews.slice(startIndex, endIndex);
  }, [allNews, currentPage]);

  const latestUpdateTime = useMemo(() => {
    if (!allNews || !Array.isArray(allNews) || allNews.length === 0) return '';
    return getLatestUpdateTime(allNews);
  }, [allNews]);

  const handlePageChange = (page: number) => {
    setCurrentPage(page);
  };

  const handlePrevious = () => {
    if (currentPage > 1) {
      setCurrentPage(currentPage - 1);
    }
  };

  const handleNext = () => {
    if (currentPage < totalPages) {
      setCurrentPage(currentPage + 1);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-2xl font-bold text-foreground">Market Summary</h2>
          <Skeleton className="h-4 w-32" />
        </div>
        <Card>
          <CardContent className="space-y-6 pt-6">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="space-y-2">
                <Skeleton className="h-6 w-3/4" />
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-5/6" />
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold text-foreground">Market Summary</h2>
        <Card>
          <CardContent className="pt-6">
            <p className="text-muted-foreground">Failed to load market news</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!allNews || !Array.isArray(allNews) || allNews.length === 0) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold text-foreground">Market Summary</h2>
        <Card>
          <CardContent className="pt-6">
            <p className="text-muted-foreground">No market news available</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-foreground">Market Summary</h2>
        {latestUpdateTime && (
          <p className="text-sm text-muted-foreground">
            Updated {latestUpdateTime}
          </p>
        )}
      </div>
      <Card>
        <CardContent className="space-y-6 pt-6">
          {paginatedNews.map((item, index) => (
            <div
              key={`${item.link}-${index}`}
              className={cn(
                'space-y-2',
                index < paginatedNews.length - 1 && 'pb-6 border-b border-border'
              )}
            >
              <h3 className="font-semibold text-foreground text-base leading-tight">
                {item.title}
              </h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {item.title}
              </p>
            </div>
          ))}
        </CardContent>
      </Card>
      {totalPages > 1 && (
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                href="#"
                onClick={(e) => {
                  e.preventDefault();
                  handlePrevious();
                }}
                className={currentPage === 1 ? 'pointer-events-none opacity-50' : ''}
              />
            </PaginationItem>
            {Array.from({ length: totalPages }).map((_, idx) => {
              const pageNum = idx + 1;
              // Show max 5 page numbers
              const maxVisiblePages = 5;
              const startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
              const endPage = Math.min(totalPages, startPage + maxVisiblePages - 1);
              
              if (pageNum < startPage || pageNum > endPage) {
                // Show ellipsis
                if (pageNum === startPage - 1 || pageNum === endPage + 1) {
                  return (
                    <PaginationItem key={`ellipsis-${pageNum}`}>
                      <span className="flex size-9 items-center justify-center text-muted-foreground">
                        ...
                      </span>
                    </PaginationItem>
                  );
                }
                return null;
              }
              
              return (
                <PaginationItem key={pageNum}>
                  <PaginationLink
                    href="#"
                    isActive={pageNum === currentPage}
                    onClick={(e) => {
                      e.preventDefault();
                      handlePageChange(pageNum);
                    }}
                  >
                    {pageNum}
                  </PaginationLink>
                </PaginationItem>
              );
            })}
            <PaginationItem>
              <PaginationNext
                href="#"
                onClick={(e) => {
                  e.preventDefault();
                  handleNext();
                }}
                className={currentPage === totalPages ? 'pointer-events-none opacity-50' : ''}
              />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}
    </div>
  );
}

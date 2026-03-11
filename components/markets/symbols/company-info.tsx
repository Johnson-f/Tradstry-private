"use client";

import React, { useMemo, useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDetailedQuote } from "@/lib/hooks/use-market-data-service";

interface CompanyInfoProps {
  symbol: string;
  className?: string;
}

export function CompanyInfo({ symbol, className }: CompanyInfoProps) {
  const { quote: data, isLoading } = useDetailedQuote(symbol, !!symbol);
  const [isModalOpen, setIsModalOpen] = useState(false);

  // Debug: log modal state changes
  useEffect(() => {
    console.log('Modal state changed:', isModalOpen);
  }, [isModalOpen]);

  // Get CEO from company officers
  const ceo = useMemo(() => {
    if (!data?.assetProfile?.companyOfficers) return null;
    const ceoOfficer = data.assetProfile.companyOfficers.find(
      (o) => o.title?.toLowerCase().includes('chief executive') || 
             o.title?.toLowerCase().includes('ceo')
    );
    return ceoOfficer?.name || null;
  }, [data]);

  // Format address
  const address = useMemo(() => {
    if (!data?.assetProfile) return null;
    const parts = [
      data.assetProfile.address1,
      data.assetProfile.city,
      data.assetProfile.state,
      data.assetProfile.zip,
    ].filter(Boolean);
    return parts.length > 0 ? parts.join(', ') : null;
  }, [data]);

  const rows = useMemo(() => {
    if (!data) return [] as { label: string; value: React.ReactNode }[];
    return [
      { label: "Symbol", value: data.symbol || symbol?.toUpperCase() },
      { label: "Market Cap", value: data.marketCap ?? "—" },
      { label: "IPO Date", value: "—" }, // Not available in asset profile
      { label: "CEO", value: ceo ?? "—" },
      { label: "Fulltime Employees", value: data.assetProfile?.fullTimeEmployees?.toLocaleString() ?? data.employees ?? "—" },
      { label: "Sector", value: data.assetProfile?.sectorDisp ?? data.sector ?? "—" },
      { label: "Industry", value: data.assetProfile?.industryDisp ?? data.industry ?? "—" },
      { label: "Country", value: data.assetProfile?.country ?? "—" },
      { label: "Exchange", value: "—" }, // Not available in asset profile
      { label: "Address", value: address ?? "—" },
      { label: "Phone", value: data.assetProfile?.phone ?? "—" },
      { label: "Website", value: data.assetProfile?.website ? (
        <a 
          href={data.assetProfile.website} 
          target="_blank" 
          rel="noopener noreferrer"
          className="text-primary hover:underline"
        >
          {data.assetProfile.website}
        </a>
      ) : "—" },
    ];
  }, [data, symbol, ceo, address]);

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <div className="p-5 sm:p-6">
        {isLoading ? (
          <LoadingState />
        ) : (
          <div className="space-y-4">
            <dl className="divide-y">
              {rows.map((row) => (
                <div key={row.label} className="grid grid-cols-2 gap-4 py-3">
                  <dt className="text-sm text-muted-foreground">{row.label}</dt>
                  <dd className="text-right text-sm sm:text-base font-medium truncate">
                    {row.value}
                  </dd>
                </div>
              ))}
            </dl>

            {(data?.assetProfile?.longBusinessSummary || data?.about) ? (
              <div className="pt-2">
                <p className="text-sm text-muted-foreground leading-6 line-clamp-5"> 
                  {data.assetProfile?.longBusinessSummary || data.about}
                </p>
                {(data.assetProfile?.longBusinessSummary || data.about || '').length > 220 && (
                  <Button 
                    variant="link" 
                    size="sm" 
                    className="px-0 mt-1" 
                    type="button"
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      console.log('Read More clicked, opening modal');
                      setIsModalOpen(true);
                    }}
                  >
                    Read More
                  </Button>
                )}
              </div>
            ) : null}
          </div>
        )}
      </div>

      {/* Company Info Modal */}
      <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] sm:max-w-2xl flex flex-col p-0 gap-0">
          <DialogHeader className="px-6 pt-6 pb-4 flex-shrink-0">
            <DialogTitle>Company Information</DialogTitle>
            <DialogDescription>
              Complete company details for {data?.name || symbol}
            </DialogDescription>
          </DialogHeader>
          
          <ScrollArea className="h-[calc(90vh-180px)] px-6 pb-6">
            <div className="space-y-6">
            {/* Company Details Grid */}
            <dl className="divide-y">
              {rows.map((row) => (
                <div key={row.label} className="grid grid-cols-2 gap-4 py-3">
                  <dt className="text-sm font-medium text-muted-foreground">{row.label}</dt>
                  <dd className="text-right text-sm sm:text-base font-medium">
                    {row.value}
                  </dd>
                </div>
              ))}
            </dl>

            {/* Company Officers */}
            {data?.assetProfile?.companyOfficers && data.assetProfile.companyOfficers.length > 0 && (
              <div className="pt-4 border-t">
                <h3 className="text-sm font-semibold mb-3">Company Officers</h3>
                <div className="space-y-3">
                  {data.assetProfile.companyOfficers.map((officer, index) => (
                    <div key={index} className="flex justify-between items-start py-2 border-b last:border-0">
                      <div>
                        <p className="text-sm font-medium">{officer.name || "—"}</p>
                        <p className="text-xs text-muted-foreground">{officer.title || "—"}</p>
                      </div>
                      {officer.totalPay?.fmt && (
                        <p className="text-xs text-muted-foreground text-right">
                          {officer.totalPay.fmt}
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Business Summary */}
            {(data?.assetProfile?.longBusinessSummary || data?.about) && (
              <div className="pt-4 border-t">
                <h3 className="text-sm font-semibold mb-3">About</h3>
                <p className="text-sm text-muted-foreground leading-6">
                  {data.assetProfile?.longBusinessSummary || data.about}
                </p>
              </div>
            )}
            </div>
          </ScrollArea>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function LoadingState() {
  return (
    <div>
      <div className="space-y-3">
        {Array.from({ length: 8 }).map((_, i) => (
          <div key={i} className="grid grid-cols-2 gap-4 py-2">
            <Skeleton className="h-4 w-28" />
            <div className="flex justify-end">
              <Skeleton className="h-4 w-32" />
            </div>
          </div>
        ))}
      </div>
      <div className="pt-4">
        <Skeleton className="h-20 w-full" />
      </div>
    </div>
  );
}

export default CompanyInfo;

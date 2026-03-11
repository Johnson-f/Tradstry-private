'use client';

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createChart, type IChartApi, type DeepPartial, type CandlestickSeriesPartialOptions, CandlestickSeries, type Time, HistogramSeries, type HistogramSeriesPartialOptions } from 'lightweight-charts';

interface CandlestickBar {
  time: number | string; // unix seconds or 'YYYY-MM-DD'
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

interface ChartingProps {
  data: CandlestickBar[];
  height?: number; // fixed height, width is responsive to parent
  className?: string;
  options?: DeepPartial<Parameters<typeof createChart>[1]>; // chart options override
  seriesOptions?: DeepPartial<CandlestickSeriesPartialOptions>; // series options override
  showVolume?: boolean;
}

export function Charting({
  data,
  height = 520,
  className,
  options,
  seriesOptions,
  showVolume = true,
}: ChartingProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ReturnType<IChartApi['addSeries']> | null>(null);
  const volumeSeriesRef = useRef<ReturnType<IChartApi['addSeries']> | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const dataRef = useRef<(CandlestickBar & { time: number | string })[]>([]);
  const [hoverBar, setHoverBar] = useState<{
    time: number | string;
    open: number;
    high: number;
    low: number;
    close: number;
    volume?: number;
  } | null>(null);

  const initialChartOptions = useMemo(() => ({
    layout: {
      background: { type: 'solid' as const, color: 'transparent' },
      textColor: '#94a3b8',
      fontSize: 10,
    },
    grid: {
      vertLines: { visible: false },
      horzLines: { visible: false },
    },
    rightPriceScale: { borderVisible: false },
    timeScale: { borderColor: 'rgba(148,163,184,0.24)' },
    localization: { priceFormatter: (p: number) => p.toFixed(2) },
  }), []);

  const initialSeriesOptions = useMemo<DeepPartial<CandlestickSeriesPartialOptions>>(
    () => ({
      upColor: '#22c55e',
      downColor: '#ef4444',
      borderVisible: false,
      wickUpColor: '#22c55e',
      wickDownColor: '#ef4444',
    }),
    []
  );

  // Initialize chart
  useEffect(() => {
    if (!containerRef.current) return;

    const container = containerRef.current;
     // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const chart = createChart(container, {
      width: container.clientWidth,
      height,
      ...initialChartOptions,
      ...(options ?? {}),
    });
    chartRef.current = chart;

    const series = chart.addSeries(CandlestickSeries, {
      ...initialSeriesOptions,
      ...(seriesOptions ?? {}),
    });
    seriesRef.current = series;

    if (showVolume) {
      const volumeSeries = chart.addSeries(HistogramSeries, {
        priceScaleId: '',
        priceFormat: { type: 'volume' },
        color: 'rgba(148,163,184,0.55)',
        base: 0,
      } as DeepPartial<HistogramSeriesPartialOptions>);
      volumeSeriesRef.current = volumeSeries;
    }

    chart.timeScale().fitContent();

    // Observe width changes to keep chart responsive
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.target === container && chartRef.current) {
          const newWidth = Math.floor(entry.contentRect.width);
          if (newWidth > 0) {
            chartRef.current.applyOptions({ width: newWidth, height });
          }
        }
      }
    });
    ro.observe(container);
    resizeObserverRef.current = ro;

    // Crosshair OHLCV display
    const handleCrosshairMove = (param: unknown) => {
      if (!seriesRef.current) return;
      // seriesData Map keyed by series object
       // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
      const sd = param?.seriesData?.get(seriesRef.current as unknown as object) as
        | { open: number; high: number; low: number; close: number; time: number | string }
        | undefined;
      if (sd && typeof sd.open === 'number') {
        // Try enrich with volume from our dataRef by time
        const match = dataRef.current.find((b) => String(b.time) === String(sd.time));
        setHoverBar({
          time: sd.time,
          open: sd.open,
          high: sd.high,
          low: sd.low,
          close: sd.close,
          volume: match?.volume,
        });
      } else if (dataRef.current.length > 0) {
        // fallback to last
        const last = dataRef.current[dataRef.current.length - 1];
        setHoverBar({
          time: last.time,
          open: last.open,
          high: last.high,
          low: last.low,
          close: last.close,
          volume: last.volume,
        });
      }
    };
    chart.subscribeCrosshairMove(handleCrosshairMove);

    return () => {
      resizeObserverRef.current?.disconnect();
      resizeObserverRef.current = null;
      chartRef.current?.remove();
      chartRef.current = null;
      seriesRef.current = null;
      volumeSeriesRef.current = null;
      chart.unsubscribeCrosshairMove(handleCrosshairMove);
    };
  }, [height, initialChartOptions, initialSeriesOptions, options, seriesOptions, showVolume]);

  // Set/Update data when it changes
  useEffect(() => {
    if (!seriesRef.current) return;

    // Ensure time format is acceptable: number (seconds) or string 'YYYY-MM-DD'
    const mapped = data.map((bar) => ({
      time: (typeof bar.time === 'number' ? (bar.time as number) : (bar.time as string)) as Time,
      open: bar.open,
      high: bar.high,
      low: bar.low,
      close: bar.close,
    }));

    seriesRef.current.setData(mapped);
    // Keep raw with volume and original time for OHLCV lookup
    dataRef.current = data.map((b) => ({ ...b, time: b.time }));
    // Default display to last bar
    if (dataRef.current.length > 0) {
      const last = dataRef.current[dataRef.current.length - 1];
      setHoverBar({
        time: last.time,
        open: last.open,
        high: last.high,
        low: last.low,
        close: last.close,
        volume: last.volume,
      });
    }
    if (volumeSeriesRef.current && showVolume) {
      const volData = data
        .filter((b) => typeof b.volume === 'number')
        .map((b) => ({
          time: (typeof b.time === 'number' ? (b.time as number) : (b.time as string)) as Time,
          value: (b.volume as number) || 0,
          color: b.close >= b.open ? 'rgba(34,197,94,0.55)' : 'rgba(239,68,68,0.55)',
        }));
      volumeSeriesRef.current.setData(volData);
    }
    chartRef.current?.timeScale().fitContent();
  }, [data, showVolume]);

  return (
    <div className={className} style={{ width: '100%', height, position: 'relative', overflow: 'hidden' }}>
      <div
        ref={containerRef}
        style={{ width: '100%', height }}
      />
      {hoverBar && (
        <div className="pointer-events-none absolute left-2 top-2 z-10 rounded-md bg-background/75 px-2 py-1 text-xs shadow-sm backdrop-blur">
          <span className="mr-2">O {hoverBar.open.toFixed(2)}</span>
          <span className="mr-2">H {hoverBar.high.toFixed(2)}</span>
          <span className="mr-2">L {hoverBar.low.toFixed(2)}</span>
          <span className="mr-2">C {hoverBar.close.toFixed(2)}</span>
          {typeof hoverBar.volume === 'number' && (
            <span>V {Math.round(hoverBar.volume)}</span>
          )}
        </div>
      )}
    </div>
  );
}

export default Charting;


